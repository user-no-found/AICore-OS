use crate::{
    conversation::ConversationController,
    gateway::{GatewaySource, InstanceIoGateway},
    ledger::{LedgerEvent, LedgerEventKind, LedgerRole},
    output::{OutputEvent, OutputRouter, OutputTarget},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeStatus {
    Idle,
    HandlingInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSummary {
    pub instance_id: String,
    pub conversation_id: String,
    pub event_count: usize,
    pub status: RuntimeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRuntime {
    gateway: InstanceIoGateway,
    conversation: ConversationController,
    output_router: OutputRouter,
    status: RuntimeStatus,
}

impl InstanceRuntime {
    pub fn new(instance_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        let instance_id = instance_id.into();

        Self {
            gateway: InstanceIoGateway::new(instance_id.clone()),
            conversation: ConversationController::new(instance_id, conversation_id),
            output_router: OutputRouter::new(OutputTarget::ActiveView),
            status: RuntimeStatus::Idle,
        }
    }

    pub fn handle_user_input(&mut self, source: GatewaySource, content: &str) -> OutputEvent {
        self.status = RuntimeStatus::HandlingInput;
        let normalized = self.gateway.normalize_user_input(source, content);

        self.conversation.append(LedgerEvent {
            seq: 0,
            kind: LedgerEventKind::Message,
            role: LedgerRole::User,
            content: normalized.content,
        });

        let source_name = match normalized.source {
            GatewaySource::Cli => "cli",
            GatewaySource::Tui => "tui",
            GatewaySource::Web => "web",
            GatewaySource::External => "external",
        };
        let reply = format!("已收到来自 {} 的输入。", source_name);

        self.conversation.append(LedgerEvent {
            seq: 0,
            kind: LedgerEventKind::Message,
            role: LedgerRole::Assistant,
            content: reply.clone(),
        });

        let output = self.output_router.route_reply(reply);
        self.status = RuntimeStatus::Idle;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InstanceRuntime, RuntimeStatus};
    use crate::{
        gateway::GatewaySource,
        ledger::{LedgerEventKind, LedgerRole},
        output::OutputTarget,
    };

    #[test]
    fn preserves_message_order_in_ledger() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        let output = runtime.handle_user_input(GatewaySource::Cli, "hello");

        let events = runtime.conversation().events();
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
}
