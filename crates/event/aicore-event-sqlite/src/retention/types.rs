#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionSkipReason {
    Protected,
    TooNew,
    Uncompacted,
    InvalidClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionSkip {
    pub event_id: String,
    pub reason: RetentionSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionPlan {
    pub scanned: usize,
    pub eligible_for_compaction: usize,
    pub eligible_for_delete: usize,
    pub protected_skipped: usize,
    pub too_new_skipped: usize,
    pub uncompacted_skipped: usize,
    pub invalid_class_skipped: usize,
    pub compaction_candidate_event_ids: Vec<String>,
    pub delete_candidate_event_ids: Vec<String>,
    pub skipped: Vec<RetentionSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionApplyResult {
    pub run_id: String,
    pub scanned: usize,
    pub eligible_for_compaction: usize,
    pub compacted: usize,
    pub eligible_for_delete: usize,
    pub deleted: usize,
    pub protected_skipped: usize,
    pub too_new_skipped: usize,
    pub uncompacted_skipped: usize,
    pub invalid_class_skipped: usize,
    pub failed: usize,
    pub compacted_event_ids: Vec<String>,
    pub deleted_event_ids: Vec<String>,
    pub skipped: Vec<RetentionSkip>,
}
