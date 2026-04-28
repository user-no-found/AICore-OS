use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewaySource {
    Cli,
    Tui,
    Web,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportEnvelope {
    pub source: GatewaySource,
    pub platform: Option<String>,
    pub target_id: Option<String>,
    pub sender_id: Option<String>,
    pub is_group: bool,
    pub mentioned_bot: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayInput {
    pub envelope: TransportEnvelope,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceIoGateway {
    instance_id: String,
}

impl InstanceIoGateway {
    pub fn new(instance_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
        }
    }

    pub fn normalize_user_input(
        &self,
        envelope: TransportEnvelope,
        content: impl Into<String>,
    ) -> GatewayInput {
        GatewayInput {
            envelope,
            content: content.into(),
        }
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerRole {
    User,
    Assistant,
    System,
    Control,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerEventKind {
    Message,
    Output,
    Control,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEvent {
    pub seq: usize,
    pub kind: LedgerEventKind,
    pub role: LedgerRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLedger {
    events: Vec<LedgerEvent>,
}

pub type EventCursor = usize;

impl MessageLedger {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, mut event: LedgerEvent) {
        event.seq = self.events.len();
        self.events.push(event);
    }

    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }

    pub fn next_cursor(&self) -> EventCursor {
        self.events.len()
    }
}

impl Default for MessageLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationController {
    instance_id: String,
    conversation_id: String,
    ledger: MessageLedger,
}

impl ConversationController {
    pub fn new(instance_id: impl Into<String>, conversation_id: impl Into<String>) -> Self {
        Self {
            instance_id: instance_id.into(),
            conversation_id: conversation_id.into(),
            ledger: MessageLedger::new(),
        }
    }

    pub fn append(&mut self, event: LedgerEvent) {
        self.ledger.append(event);
    }

    pub fn events(&self) -> &[LedgerEvent] {
        self.ledger.events()
    }

    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn conversation_id(&self) -> &str {
        &self.conversation_id
    }

    pub fn next_cursor(&self) -> EventCursor {
        self.ledger.next_cursor()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeliveryIdentity {
    ActiveViews,
    External { platform: String, target_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputTarget {
    Origin,
    ActiveViews,
    FollowedExternal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputEvent {
    pub target: OutputTarget,
    pub identity: DeliveryIdentity,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutedOutputs {
    pub events: Vec<OutputEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputRouter {
    default_target: OutputTarget,
}

impl OutputRouter {
    pub fn new(default_target: OutputTarget) -> Self {
        Self { default_target }
    }

    pub fn route_reply(&self, content: impl Into<String>) -> OutputEvent {
        OutputEvent {
            target: self.default_target.clone(),
            identity: DeliveryIdentity::ActiveViews,
            content: content.into(),
        }
    }
}

pub fn dedupe_outputs(events: Vec<OutputEvent>) -> RoutedOutputs {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();

    for event in events {
        if seen.insert(event.identity.clone()) {
            deduped.push(event);
        }
    }

    RoutedOutputs { events: deduped }
}

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
