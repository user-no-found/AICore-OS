use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{ToolHotPlugChangeKind, ToolId, ToolNoticeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolHotPlugNotice {
    pub notice_id: ToolNoticeId,
    pub tool_id: ToolId,
    pub change_kind: ToolHotPlugChangeKind,
    pub created_at: Timestamp,
    pub remaining_turns: u8,
    pub message_en: String,
    pub message_zh: Option<String>,
}

impl ToolHotPlugNotice {
    pub fn new(
        notice_id: ToolNoticeId,
        tool_id: ToolId,
        change_kind: ToolHotPlugChangeKind,
        created_at: Timestamp,
        message_en: impl Into<String>,
        message_zh: Option<String>,
    ) -> Self {
        Self {
            notice_id,
            tool_id,
            change_kind,
            created_at,
            remaining_turns: 3,
            message_en: message_en.into(),
            message_zh,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_turns == 0
    }

    pub fn advance_one_turn(&mut self) {
        self.remaining_turns = self.remaining_turns.saturating_sub(1);
    }
}
