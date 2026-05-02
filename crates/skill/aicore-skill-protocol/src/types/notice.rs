use aicore_foundation::Timestamp;
use serde::{Deserialize, Serialize};

use super::{SkillChangeKind, SkillId, SkillNoticeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillChangeNotice {
    pub notice_id: SkillNoticeId,
    pub skill_id: SkillId,
    pub change_kind: SkillChangeKind,
    pub created_at: Timestamp,
    pub remaining_turns: u8,
    pub message_en: String,
    pub message_zh: Option<String>,
    pub authorizes_tools: bool,
}

impl SkillChangeNotice {
    pub fn new(
        notice_id: SkillNoticeId,
        skill_id: SkillId,
        change_kind: SkillChangeKind,
        created_at: Timestamp,
        message_en: String,
        message_zh: Option<String>,
    ) -> Self {
        Self {
            notice_id,
            skill_id,
            change_kind,
            created_at,
            remaining_turns: 3,
            message_en,
            message_zh,
            authorizes_tools: false,
        }
    }

    pub fn advance_one_turn(&mut self) {
        self.remaining_turns = self.remaining_turns.saturating_sub(1);
    }

    pub fn expired(&self) -> bool {
        self.remaining_turns == 0
    }
}
