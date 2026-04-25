use crate::{
    conversation::ConversationController,
    gateway::{GatewaySource, InstanceIoGateway, TransportEnvelope},
    ledger::{EventCursor, LedgerEvent, LedgerEventKind, LedgerRole},
    output::{
        DeliveryIdentity, OutputEvent, OutputRouter, OutputTarget, RoutedOutputs, dedupe_outputs,
    },
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
    pub decision: InterruptDecision,
    pub active_turn_id: Option<String>,
    pub queue_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRuntime {
    gateway: InstanceIoGateway,
    conversation: ConversationController,
    output_router: OutputRouter,
    status: RuntimeStatus,
    active_turn: Option<ActiveTurn>,
    pending_queue: Vec<QueuedMessage>,
    follow_subscriptions: Vec<FollowSubscription>,
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
            follow_subscriptions: Vec::new(),
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
            decision: InterruptDecision::StartTurn,
            active_turn_id: self.active_turn.as_ref().map(|turn| turn.id.clone()),
            queue_len: self.pending_queue.len(),
        }
    }

    pub fn handle_ingress(
        &mut self,
        envelope: TransportEnvelope,
        content: &str,
        requested_mode: InterruptMode,
    ) -> IngressResult {
        let accepted_source = envelope.source.clone();
        let decision = self.decide_interrupt(&envelope, requested_mode.clone());

        match decision {
            InterruptDecision::StartTurn => {
                let next_turn_id = format!("turn_{}", self.conversation.next_cursor());
                if accepted_source == GatewaySource::External {
                    self.begin_turn_with_origin(next_turn_id, envelope.clone());
                } else {
                    self.begin_turn(next_turn_id);
                }

                self.status = RuntimeStatus::HandlingInput;
                let normalized = self.gateway.normalize_user_input(envelope, content);
                self.conversation.append(LedgerEvent {
                    seq: 0,
                    kind: LedgerEventKind::Message,
                    role: LedgerRole::User,
                    content: normalized.content,
                });
                self.status = RuntimeStatus::Idle;
            }
            InterruptDecision::Queue => {
                self.queue_message(envelope, content, requested_mode);
            }
            InterruptDecision::AppendContext => {
                self.conversation_status = ConversationStatus::Running;
            }
            InterruptDecision::SoftInterrupt | InterruptDecision::HardInterrupt => {
                self.interrupt(requested_mode);
            }
        }

        IngressResult {
            accepted_source,
            event_count: self.conversation.events().len(),
            decision,
            active_turn_id: self.active_turn.as_ref().map(|turn| turn.id.clone()),
            queue_len: self.pending_queue.len(),
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

        if let Some(identity) = self.active_turn_origin_identity() {
            events.push(OutputEvent {
                target: OutputTarget::Origin,
                identity,
                content: content.to_string(),
            });
        }

        for follow in &self.follow_subscriptions {
            if assistant_seq >= follow.cursor {
                if let Some(identity) = Self::external_identity_from_envelope(&follow.envelope) {
                    events.push(OutputEvent {
                        target: OutputTarget::FollowedExternal,
                        identity,
                        content: content.to_string(),
                    });
                }
            }
        }

        self.conversation_status = if self.active_turn.is_some() {
            ConversationStatus::Running
        } else if self.pending_queue.is_empty() {
            ConversationStatus::Idle
        } else {
            ConversationStatus::Queued
        };
        dedupe_outputs(events)
    }

    fn active_turn_origin_identity(&self) -> Option<DeliveryIdentity> {
        self.active_turn
            .as_ref()
            .and_then(|turn| turn.origin.as_ref())
            .and_then(Self::external_identity_from_envelope)
    }

    fn external_identity_from_envelope(envelope: &TransportEnvelope) -> Option<DeliveryIdentity> {
        match (&envelope.platform, &envelope.target_id) {
            (Some(platform), Some(target_id)) => Some(DeliveryIdentity::External {
                platform: platform.clone(),
                target_id: target_id.clone(),
            }),
            _ => None,
        }
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
        let Some(identity) = Self::external_identity_from_envelope(&envelope) else {
            return;
        };

        let already_exists = self.follow_subscriptions.iter().any(|follow| {
            Self::external_identity_from_envelope(&follow.envelope)
                .is_some_and(|existing| existing == identity)
        });

        if already_exists {
            return;
        }

        self.follow_subscriptions.push(FollowSubscription {
            cursor: self.conversation.next_cursor(),
            envelope,
        });
    }

    pub fn follow_subscriptions(&self) -> &[FollowSubscription] {
        &self.follow_subscriptions
    }

    pub fn follow_count(&self) -> usize {
        self.follow_subscriptions.len()
    }

    pub fn unfollow_external(&mut self, identity: &DeliveryIdentity) -> bool {
        let original_len = self.follow_subscriptions.len();
        self.follow_subscriptions.retain(|follow| {
            Self::external_identity_from_envelope(&follow.envelope)
                .is_none_or(|existing| &existing != identity)
        });
        self.follow_subscriptions.len() != original_len
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
        output::{DeliveryIdentity, OutputTarget},
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
            .follow_subscriptions()
            .first()
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
            .follow_subscriptions()
            .first()
            .expect("follow subscription must exist");
        assert_eq!(follow.cursor, runtime.conversation().next_cursor());
        assert_eq!(runtime.summary().event_count, 2);
    }

    #[test]
    fn assistant_output_during_active_turn_keeps_conversation_running() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        runtime.append_assistant_output("reply");

        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Running
        );
    }

    #[test]
    fn idle_external_input_starts_turn_with_origin() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        let result = runtime.handle_ingress(external_envelope(), "hello", InterruptMode::Queue);

        assert_eq!(result.decision, InterruptDecision::StartTurn);
        assert!(result.active_turn_id.is_some());
        assert_eq!(runtime.active_turn_id(), result.active_turn_id.as_deref());
        assert_eq!(runtime.conversation().events().len(), 1);
        assert_eq!(runtime.conversation().events()[0].role, LedgerRole::User);
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Running
        );
    }

    #[test]
    fn idle_local_input_starts_turn_without_origin_reply_semantics() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        let result = runtime.handle_ingress(tui_envelope(), "hello", InterruptMode::Queue);
        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert_eq!(result.decision, InterruptDecision::StartTurn);
        assert!(result.active_turn_id.is_some());
        assert_eq!(targets, vec![OutputTarget::ActiveViews]);
    }

    #[test]
    fn busy_external_input_queues() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        let result = runtime.handle_ingress(external_envelope(), "queued", InterruptMode::Queue);

        assert_eq!(result.decision, InterruptDecision::Queue);
        assert_eq!(runtime.pending_queue_len(), 1);
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
        assert_eq!(
            runtime.summary().conversation_status,
            ConversationStatus::Running
        );
    }

    #[test]
    fn busy_local_append_context_does_not_queue() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        let result = runtime.handle_ingress(
            tui_envelope(),
            "context update",
            InterruptMode::AppendContext,
        );

        assert_eq!(result.decision, InterruptDecision::AppendContext);
        assert_eq!(runtime.pending_queue_len(), 0);
        assert_eq!(runtime.active_turn_id(), Some("turn_1"));
    }

    #[test]
    fn hard_interrupt_marks_active_turn_cancel_requested() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn("turn_1");

        let result = runtime.handle_ingress(cli_envelope(), "stop", InterruptMode::HardInterrupt);

        assert_eq!(result.decision, InterruptDecision::HardInterrupt);
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
    fn external_ingress_origin_causes_assistant_output_to_include_origin() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        let result = runtime.handle_ingress(external_envelope(), "hello", InterruptMode::Queue);
        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert_eq!(result.decision, InterruptDecision::StartTurn);
        assert!(targets.contains(&OutputTarget::ActiveViews));
        assert!(targets.contains(&OutputTarget::Origin));
    }

    #[test]
    fn external_origin_output_has_external_identity() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        let outputs = runtime.append_assistant_output("reply");
        let origin = outputs
            .events
            .iter()
            .find(|event| event.target == OutputTarget::Origin)
            .expect("origin output must exist");

        assert_eq!(
            origin.identity,
            DeliveryIdentity::External {
                platform: "feishu".to_string(),
                target_id: "chat-1".to_string(),
            }
        );
    }

    #[test]
    fn followed_external_output_has_external_identity() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });

        let outputs = runtime.append_assistant_output("reply");
        let followed = outputs
            .events
            .iter()
            .find(|event| event.target == OutputTarget::FollowedExternal)
            .expect("followed external output must exist");

        assert_eq!(
            followed.identity,
            DeliveryIdentity::External {
                platform: "feishu".to_string(),
                target_id: "chat-2".to_string(),
            }
        );
    }

    #[test]
    fn origin_and_followed_external_same_target_dedupes() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(external_envelope());
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();
        let external_count = outputs
            .events
            .iter()
            .filter(|event| matches!(event.identity, DeliveryIdentity::External { .. }))
            .count();

        assert!(targets.contains(&OutputTarget::ActiveViews));
        assert!(targets.contains(&OutputTarget::Origin));
        assert!(!targets.contains(&OutputTarget::FollowedExternal));
        assert_eq!(external_count, 1);
    }

    #[test]
    fn origin_and_followed_external_different_targets_both_remain() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert!(targets.contains(&OutputTarget::ActiveViews));
        assert!(targets.contains(&OutputTarget::Origin));
        assert!(targets.contains(&OutputTarget::FollowedExternal));
        assert_eq!(outputs.events.len(), 3);
    }

    #[test]
    fn missing_external_identity_skips_external_output() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.begin_turn_with_origin(
            "turn_1",
            TransportEnvelope {
                source: GatewaySource::External,
                platform: Some("feishu".to_string()),
                target_id: None,
                sender_id: Some("user-1".to_string()),
                is_group: true,
                mentioned_bot: true,
            },
        );

        let outputs = runtime.append_assistant_output("reply");
        let targets: Vec<_> = outputs
            .events
            .iter()
            .map(|event| event.target.clone())
            .collect();

        assert_eq!(targets, vec![OutputTarget::ActiveViews]);
    }

    #[test]
    fn adding_two_different_follow_targets_stores_both() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        runtime.follow_external(external_envelope());
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("wechat".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });

        assert_eq!(runtime.follow_count(), 2);
    }

    #[test]
    fn adding_same_follow_target_twice_dedupes() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        runtime.follow_external(external_envelope());
        runtime.follow_external(external_envelope());

        assert_eq!(runtime.follow_count(), 1);
    }

    #[test]
    fn invalid_follow_envelope_without_platform_or_target_id_is_ignored() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");

        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: None,
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        });

        assert_eq!(runtime.follow_count(), 0);
    }

    #[test]
    fn assistant_output_routes_to_all_followed_external_targets() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(external_envelope());
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("wechat".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });

        let outputs = runtime.append_assistant_output("reply");
        let followed_count = outputs
            .events
            .iter()
            .filter(|event| event.target == OutputTarget::FollowedExternal)
            .count();

        assert_eq!(followed_count, 2);
    }

    #[test]
    fn origin_and_one_followed_target_same_identity_dedupes() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(external_envelope());
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("wechat".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });
        runtime.begin_turn_with_origin("turn_1", external_envelope());

        let outputs = runtime.append_assistant_output("reply");
        let external_targets: Vec<_> = outputs
            .events
            .iter()
            .filter_map(|event| match &event.identity {
                DeliveryIdentity::External {
                    platform,
                    target_id,
                } => Some((event.target.clone(), platform.clone(), target_id.clone())),
                DeliveryIdentity::ActiveViews => None,
            })
            .collect();

        assert!(external_targets.contains(&(
            OutputTarget::Origin,
            "feishu".to_string(),
            "chat-1".to_string()
        )));
        assert!(external_targets.contains(&(
            OutputTarget::FollowedExternal,
            "wechat".to_string(),
            "chat-2".to_string()
        )));
        assert_eq!(external_targets.len(), 2);
    }

    #[test]
    fn removing_a_follow_target_stops_future_followed_output() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(external_envelope());

        let removed = runtime.unfollow_external(&DeliveryIdentity::External {
            platform: "feishu".to_string(),
            target_id: "chat-1".to_string(),
        });
        let outputs = runtime.append_assistant_output("reply");
        let followed_count = outputs
            .events
            .iter()
            .filter(|event| event.target == OutputTarget::FollowedExternal)
            .count();

        assert!(removed);
        assert_eq!(followed_count, 0);
    }

    #[test]
    fn list_follows_returns_current_subscriptions() {
        let mut runtime = InstanceRuntime::new("global-main", "conv_main");
        runtime.follow_external(external_envelope());
        runtime.follow_external(TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("wechat".to_string()),
            target_id: Some("chat-2".to_string()),
            sender_id: Some("user-2".to_string()),
            is_group: true,
            mentioned_bot: true,
        });

        assert_eq!(runtime.follow_subscriptions().len(), 2);
    }
}
