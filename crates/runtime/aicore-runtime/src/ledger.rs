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
}
