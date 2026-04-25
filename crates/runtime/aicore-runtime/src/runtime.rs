use crate::{
    conversation::ConversationController,
    gateway::{GatewaySource, InstanceIoGateway, TransportEnvelope},
    ledger::{EventCursor, LedgerEvent, LedgerEventKind, LedgerRole},
    output::{OutputEvent, OutputRouter, OutputTarget, RoutedOutputs},
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
    Interrupted,
}

pub type TurnId = String;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnStatus {
    Running,
    Completed,
    Interrupted,
    CancelRequested,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveTurn {
    pub id: TurnId,
    pub status: TurnStatus,
    pub origin: Option<TransportEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterruptMode {
    Queue,
    AppendContext,
    SoftInterrupt,
    HardInterrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnState {
    pub active_turn_id: Option<String>,
    pub active_turn_status: Option<TurnStatus>,
    pub queue_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedMessage {
    pub envelope: TransportEnvelope,
    pub content: String,
    pub interrupt_mode: InterruptMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterruptDecision {
    StartTurn,
    Queue,
    AppendContext,
    SoftInterrupt,
    HardInterrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FollowSubscription {
    pub cursor: EventCursor,
    pub envelope: TransportEnvelope,
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
    active_turn: Option<ActiveTurn>,
    pending_queue: Vec<QueuedMessage>,
    follow_subscription: Option<FollowSubscription>,
    conversation_status: ConversationStatus,
}

impl InstanceRuntime {
    pub fn new(instance_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        let instance_id = instance_id.into();

        Self {
            gateway: InstanceIoGateway::new(instance_id.clone()),
            conversation: ConversationController::new(instance_id, conversation_id),
            output_router: OutputRouter::new(OutputTarget::ActiveViews),
            status: RuntimeStatus::Idle,
            active_turn: None,
            pending_queue: Vec::new(),
            follow_subscription: None,
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

    pub fn append_assistant_output(&mut self, content: &str) -> RoutedOutputs {
        self.conversation_status = ConversationStatus::Running;
        self.conversation.append(LedgerEvent {
            seq: 0,
            kind: LedgerEventKind::Message,
            role: LedgerRole::Assistant,
            content: content.to_string(),
        });

        let assistant_seq = self.conversation.events().len() - 1;
        let mut events = vec![self.output_router.route_reply(content)];

        if self.active_turn_origin_is_external() {
            events.push(OutputEvent {
                target: OutputTarget::Origin,
                content: content.to_string(),
            });
        }

        if let Some(follow) = &self.follow_subscription {
            if assistant_seq >= follow.cursor {
                events.push(OutputEvent {
                    target: OutputTarget::FollowedExternal,
                    content: content.to_string(),
                });
            }
        }

        self.conversation_status = ConversationStatus::Idle;
        RoutedOutputs { events }
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
            queue_len: self.pending_queue.len(),
            conversation_status: self.conversation_status.clone(),
        }
    }

    pub fn queue_len(&self) -> usize {
        self.pending_queue.len()
    }

    pub fn pending_queue_len(&self) -> usize {
        self.pending_queue.len()
    }

    pub fn active_turn_id(&self) -> Option<&str> {
        self.active_turn.as_ref().map(|turn| turn.id.as_str())
    }

    pub fn begin_turn(&mut self, turn_id: impl Into<String>) {
        self.active_turn = Some(ActiveTurn {
            id: turn_id.into(),
            status: TurnStatus::Running,
            origin: None,
        });
        self.conversation_status = ConversationStatus::Running;
    }

    pub fn begin_turn_with_origin(
        &mut self,
        turn_id: impl Into<String>,
        origin: TransportEnvelope,
    ) {
        self.active_turn = Some(ActiveTurn {
            id: turn_id.into(),
            status: TurnStatus::Running,
            origin: Some(origin),
        });
        self.conversation_status = ConversationStatus::Running;
    }

    pub fn complete_turn(&mut self) {
        if let Some(turn) = &mut self.active_turn {
            turn.status = TurnStatus::Completed;
        }

        self.active_turn = None;
        self.conversation_status = if self.pending_queue.is_empty() {
            ConversationStatus::Idle
        } else {
            ConversationStatus::Queued
        };
    }

    pub fn decide_interrupt(
        &self,
        _envelope: &TransportEnvelope,
        requested_mode: InterruptMode,
    ) -> InterruptDecision {
        if self.active_turn.is_none() {
            return InterruptDecision::StartTurn;
        }

        match requested_mode {
            InterruptMode::Queue => InterruptDecision::Queue,
            InterruptMode::AppendContext => InterruptDecision::AppendContext,
            InterruptMode::SoftInterrupt => InterruptDecision::SoftInterrupt,
            InterruptMode::HardInterrupt => InterruptDecision::HardInterrupt,
        }
    }

    pub fn queue_message(
        &mut self,
        envelope: TransportEnvelope,
        content: impl Into<String>,
        mode: InterruptMode,
    ) {
        self.pending_queue.push(QueuedMessage {
            envelope,
            content: content.into(),
            interrupt_mode: mode,
        });

        self.conversation_status = if self.active_turn.is_some() {
            ConversationStatus::Running
        } else {
            ConversationStatus::Queued
        };
    }

    pub fn clear_queue(&mut self) {
        self.pending_queue.clear();
        self.conversation_status = if self.active_turn.is_some() {
            ConversationStatus::Running
        } else {
            ConversationStatus::Idle
        };
    }

    pub fn interrupt(&mut self, mode: InterruptMode) {
        if let Some(turn) = &mut self.active_turn {
            turn.status = match mode {
                InterruptMode::HardInterrupt => TurnStatus::CancelRequested,
                InterruptMode::SoftInterrupt => TurnStatus::Interrupted,
                InterruptMode::AppendContext | InterruptMode::Queue => turn.status.clone(),
            };
        }

        if matches!(
            mode,
            InterruptMode::SoftInterrupt | InterruptMode::HardInterrupt
        ) {
            self.conversation_status = ConversationStatus::Interrupted;
        }
    }

    pub fn turn_state(&self) -> TurnState {
        TurnState {
            active_turn_id: self.active_turn.as_ref().map(|turn| turn.id.clone()),
            active_turn_status: self.active_turn.as_ref().map(|turn| turn.status.clone()),
            queue_len: self.pending_queue.len(),
        }
    }

    pub fn follow_external(&mut self, envelope: TransportEnvelope) {
        self.follow_subscription = Some(FollowSubscription {
            cursor: self.conversation.next_cursor(),
            envelope,
        });
    }

    pub fn follow_subscription(&self) -> Option<&FollowSubscription> {
        self.follow_subscription.as_ref()
    }

    fn active_turn_origin_is_external(&self) -> bool {
        self.active_turn
            .as_ref()
            .and_then(|turn| turn.origin.as_ref())
            .is_some_and(|origin| origin.source == GatewaySource::External)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConversationStatus, InstanceRuntime, InterruptDecision, InterruptMode, RuntimeStatus,
        TurnStatus,
    };
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

    fn external_envelope() -> TransportEnvelope {
        TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-1".to_string()),
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        }
    }

    fn tui_envelope() -> TransportEnvelope {
        TransportEnvelope {
            source: GatewaySource::Tui,
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
        assert_eq!(output.events.len(), 1);
        assert_eq!(output.events[0].target, OutputTarget::ActiveViews);
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
        assert_eq!(runtime.turn_state().active_turn_id, None);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Idle
        );

        runtime.queue_message(external_envelope(), "queued", InterruptMode::Queue);
        assert_eq!(runtime.queue_len(), 1);
        assert_eq!(runtime.turn_state().queue_len, 1);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Queued
        );

        runtime.interrupt(InterruptMode::SoftInterrupt);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Interrupted
        );

        runtime.clear_queue();
        assert_eq!(runtime.queue_len(), 0);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Idle
        );
    }

    #[test]
    fn idle_conversation_starts_a_turn() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        assert_eq!(runtime.active_turn_id(), None);
        assert_eq!(
            runtime.decide_interrupt(&cli_envelope(), InterruptMode::Queue),
            InterruptDecision::StartTurn
        );

        runtime.begin_turn("turn_1");
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(
            runtime.turn_state().active_turn_status,
            Some(TurnStatus::Running)
        );
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Running
        );
    }

    #[test]
    fn busy_external_message_queues() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        assert_eq!(
            runtime.decide_interrupt(&external_envelope(), InterruptMode::Queue),
            InterruptDecision::Queue
        );

        runtime.queue_message(external_envelope(), "next message", InterruptMode::Queue);
        assert_eq!(runtime.pending_queue_len(), 1);
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Running
        );
    }

    #[test]
    fn busy_local_message_append_context() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        assert_eq!(
            runtime.decide_interrupt(&tui_envelope(), InterruptMode::AppendContext),
            InterruptDecision::AppendContext
        );
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(runtime.pending_queue_len(), 0);
    }

    #[test]
    fn hard_interrupt_marks_active_turn() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        assert_eq!(
            runtime.decide_interrupt(&cli_envelope(), InterruptMode::HardInterrupt),
            InterruptDecision::HardInterrupt
        );

        runtime.interrupt(InterruptMode::HardInterrupt);
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(
            runtime.turn_state().active_turn_status,
            Some(TurnStatus::CancelRequested)
        );
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Interrupted
        );
    }

    #[test]
    fn complete_turn_clears_active_turn() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        runtime.complete_turn();
        assert_eq!(runtime.active_turn_id(), None);
        assert_eq!(runtime.turn_state().active_turn_status, None);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Idle
        );
    }

    #[test]
    fn queued_messages_remain_after_turn_completion() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");
        runtime.queue_message(external_envelope(), "queued", InterruptMode::Queue);

        runtime.complete_turn();
        assert_eq!(runtime.active_turn_id(), None);
        assert_eq!(runtime.pending_queue_len(), 1);
    }

    #[test]
    fn follow_starts_from_current_cursor() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.ingest_user_input(cli_envelope(), "hello");
        runtime.append_assistant_output("reply");

        runtime.follow_external(external_envelope());
        let follow = runtime
            .follow_subscription()
            .expect("follow subscription must exist");

        assert_eq!(follow.cursor, runtime.conversation().next_cursor());
    }

    #[test]
    fn external_origin_turn_routes_to_origin() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert!(targets.contains(&OutputTarget::ActiveViews));
        assert!(targets.contains(&OutputTarget::Origin));
    }

    #[test]
    fn follow_subscription_receives_only_outputs_after_cursor() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.ingest_user_input(cli_envelope(), "hello");
        runtime.append_assistant_output("before follow");
        runtime.follow_external(external_envelope());

        let outputs = runtime.append_assistant_output("after follow");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert!(targets.contains(&OutputTarget::FollowedExternal));
    }

    #[test]
    fn no_history_replay_on_follow_start() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.ingest_user_input(cli_envelope(), "hello");
        runtime.append_assistant_output("before follow");

        runtime.follow_external(external_envelope());

        let follow = runtime
            .follow_subscription()
            .expect("follow subscription must exist");
        assert_eq!(follow.cursor, runtime.conversation().next_cursor());
        assert_eq!(runtime.summary().event_count, 2);
    }
}
