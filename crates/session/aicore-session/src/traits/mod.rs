use aicore_foundation::{AicoreResult, InstanceId, SessionId};

use crate::types::{
    AppendControlEventRequest, AppendLedgerWriteRequest, AppendMessageRequest, BeginTurnRequest,
    CreateSessionRequest, FinishTurnRequest, InstanceRuntimeSnapshot, MessageRecord, SessionRecord,
    SessionSummary, SetRuntimeStateRequest, TurnRecord,
};

pub trait SessionLedgerWriter {
    fn create_session(&self, request: &CreateSessionRequest) -> AicoreResult<()>;
    fn begin_turn(&self, request: &BeginTurnRequest) -> AicoreResult<()>;
    fn finish_turn(&self, request: &FinishTurnRequest) -> AicoreResult<()>;
    fn append_message(&self, request: &AppendMessageRequest) -> AicoreResult<()>;
    fn append_control_event(&self, request: &AppendControlEventRequest) -> AicoreResult<()>;
    fn append_ledger_write(&self, request: &AppendLedgerWriteRequest) -> AicoreResult<()>;
    fn set_runtime_state(&self, request: &SetRuntimeStateRequest) -> AicoreResult<()>;

    fn create_pending_input(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs not implemented yet".to_string(),
        ))
    }

    fn submit_approval(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals not implemented yet".to_string(),
        ))
    }

    fn respond_approval(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses not implemented yet".to_string(),
        ))
    }
}

pub trait SessionLedgerReader {
    fn get_session(&self, session_id: &SessionId) -> AicoreResult<Option<SessionRecord>>;
    fn get_turn(&self, turn_id: &str) -> AicoreResult<Option<TurnRecord>>;
    fn list_sessions(&self) -> AicoreResult<Vec<SessionSummary>>;
    fn read_messages(&self, session_id: &SessionId) -> AicoreResult<Vec<MessageRecord>>;
    fn get_messages_for_turn(&self, turn_id: &str) -> AicoreResult<Vec<MessageRecord>>;
    fn get_runtime_state(&self) -> AicoreResult<InstanceRuntimeSnapshot>;
    fn get_current_snapshot(&self) -> AicoreResult<InstanceRuntimeSnapshot>;

    fn read_pending_inputs(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs not implemented yet".to_string(),
        ))
    }

    fn read_approvals(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals not implemented yet".to_string(),
        ))
    }

    fn read_approval_responses(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses not implemented yet".to_string(),
        ))
    }
}

pub trait SessionLedger {
    fn instance_id(&self) -> &InstanceId;
    fn writer(&self) -> &dyn SessionLedgerWriter;
    fn reader(&self) -> &dyn SessionLedgerReader;
}
