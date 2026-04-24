use crate::ledger::{LedgerEvent, MessageLedger};

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
}
