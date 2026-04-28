mod model;

pub use model::*;

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
