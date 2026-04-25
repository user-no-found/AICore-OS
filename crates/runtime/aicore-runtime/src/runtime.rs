use crate::{
    conversation::ConversationController,
    gateway::{GatewaySource, InstanceIoGateway, TransportEnvelope},
    ledger::{LedgerEvent, LedgerEventKind, LedgerRole},
    output::{OutputEvent, OutputRouter, OutputTarget},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStatus {
    Idle,
    HandlingInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationStatus {
    Idle,
    Running,
    Queued,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSummary {
    pub instance_id: String,
    pub conversation_id: String,
    pub event_count: usize,
    pub status: RuntimeStatus,
    pub queue_len: usize,
    pub conversation_status: ConversationStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressResult {
    pub accepted_source: GatewaySource,
    pub event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRuntime {
    gateway: InstanceIoGateway,
    conversation: ConversationController,
    output_router: OutputRouter,
    status: RuntimeStatus,
    queue_len: usize,
    conversation_status: ConversationStatus,
}

impl InstanceRuntime {
    pub fn new(instance_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        let instance_id = instance_id.into();

        Self {
            gateway: InstanceIoGateway::new(instance_id.clone()),
            conversation: ConversationController::new(instance_id, conversation_id),
            output_router: OutputRouter::new(OutputTarget::ActiveView),
            status: RuntimeStatus::Idle,
            queue_len: 0,
            conversation_status: ConversationStatus::Idle,
        }
    }

    pub fn ingest_user_input(
        &mut self,
        envelope: TransportEnvelope,
        content: &str,
    ) -> IngressResult {
        self.status = RuntimeStatus::HandlingInput;
        self.conversation_status = ConversationStatus::Running;
        let normalized = self.gateway.normalize_user_input(envelope, content);
        let accepted_source = normalized.envelope.source.clone();

        self.conversation.append(LedgerEvent {
            seq: 0,
            kind: LedgerEventKind::Message,
            role: LedgerRole::User,
            content: normalized.content,
        });

        self.status = RuntimeStatus::Idle;
        self.conversation_status = ConversationStatus::Idle;
        IngressResult {
            accepted_source,
            event_count: self.conversation.events().len(),
        }
    }

    pub fn append_assistant_output(&mut self, content: &str) -> OutputEvent {
        self.conversation_status = ConversationStatus::Running;
        self.conversation.append(LedgerEvent {
            seq: 0,
            kind: LedgerEventKind::Message,
            role: LedgerRole::Assistant,
            content: content.to_string(),
        });

        let output = self.output_router.route_reply(content);
        self.conversation_status = ConversationStatus::Idle;
        output
    }

    pub fn conversation(&self) -> &ConversationController {
        &self.conversation
    }

    pub fn status(&self) -> &RuntimeStatus {
        &self.status
    }

    pub fn summary(&self) -> RuntimeSummary {
        RuntimeSummary {
            instance_id: self.conversation.instance_id().to_string(),
            conversation_id: self.conversation.conversation_id().to_string(),
            event_count: self.conversation.events().len(),
            status: self.status.clone(),
            queue_len: self.queue_len,
            conversation_status: self.conversation_status.clone(),
        }
    }

    pub fn queue_len(&self) -> usize {
        self.queue_len
    }

    pub fn queue_message(&mut self) {
        self.queue_len += 1;
        self.conversation_status = ConversationStatus::Queued;
    }

    pub fn clear_queue(&mut self) {
        self.queue_len = 0;
        self.conversation_status = ConversationStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversationStatus, InstanceRuntime, RuntimeStatus};
    use crate::{
        gateway::{GatewaySource, TransportEnvelope},
        ledger::{LedgerEventKind, LedgerRole},
        output::OutputTarget,
    };

    fn cli_envelope() -> TransportEnvelope {
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        }
    }

    #[test]
    fn preserves_message_order_in_ledger() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        let ingress = runtime.ingest_user_input(cli_envelope(), "hello");
        let output = runtime.append_assistant_output("reply");

        let events = runtime.conversation().events();
        assert_eq!(ingress.event_count, 1);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 0);
        assert_eq!(events[0].kind, LedgerEventKind::Message);
        assert_eq!(events[0].role, LedgerRole::User);
        assert_eq!(events[1].seq, 1);
        assert_eq!(events[1].kind, LedgerEventKind::Message);
        assert_eq!(events[1].role, LedgerRole::Assistant);
        assert_eq!(output.target, OutputTarget::ActiveView);
        assert_eq!(runtime.status(), &RuntimeStatus::Idle);
    }

    #[test]
    fn binds_conversation_to_instance() {
        let runtime = InstanceRuntime::new("inst_project_a", "conv_a");

        assert_eq!(runtime.conversation().instance_id(), "inst_project_a");
        assert_eq!(runtime.conversation().conversation_id(), "conv_a");
    }

    #[test]
    fn exposes_queue_and_interrupt_skeleton() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        assert_eq!(runtime.queue_len(), 0);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Idle
        );

        runtime.queue_message();
        assert_eq!(runtime.queue_len(), 1);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Queued
        );

        runtime.clear_queue();
        assert_eq!(runtime.queue_len(), 0);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Idle
        );
    }
}
