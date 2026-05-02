use aicore_foundation::{AicoreResult, InstanceId, SessionId};

use crate::types::{
    ActiveTurnAcquireOutcome, ActiveTurnAcquireRequest, ActiveTurnReleaseOutcome,
    ActiveTurnReleaseRequest, AppendControlEventRequest, AppendLedgerWriteRequest,
    AppendMessageRequest, ApprovalRecord, ApprovalResponseOutcome, ApprovalResponseRecord,
    ApprovalResponseRequest, ApprovalStatus, BeginTurnRequest, CreateApprovalRequest,
    CreateSessionRequest, FinishTurnRequest, InstanceRuntimeSnapshot, MessageRecord,
    PendingInputCancelOutcome, PendingInputCancelRequest, PendingInputRecord,
    PendingInputSubmitOutcome, PendingInputSubmitRequest, SessionRecord, SessionSummary,
    SetRuntimeStateRequest, StopTurnOutcome, StopTurnRequest, TurnRecord,
};

pub trait SessionLedgerWriter {
    fn create_session(&self, request: &CreateSessionRequest) -> AicoreResult<()>;
    fn begin_turn(&self, request: &BeginTurnRequest) -> AicoreResult<()>;
    fn finish_turn(&self, request: &FinishTurnRequest) -> AicoreResult<()>;
    fn append_message(&self, request: &AppendMessageRequest) -> AicoreResult<()>;
    fn append_control_event(&self, request: &AppendControlEventRequest) -> AicoreResult<()>;
    fn append_ledger_write(&self, request: &AppendLedgerWriteRequest) -> AicoreResult<()>;
    fn set_runtime_state(&self, request: &SetRuntimeStateRequest) -> AicoreResult<()>;

    fn acquire_active_turn(
        &self,
        _request: &ActiveTurnAcquireRequest,
    ) -> AicoreResult<ActiveTurnAcquireOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "active turn lock not implemented yet".to_string(),
        ))
    }

    fn release_active_turn(
        &self,
        _request: &ActiveTurnReleaseRequest,
    ) -> AicoreResult<ActiveTurnReleaseOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "active turn release not implemented yet".to_string(),
        ))
    }

    fn submit_or_replace_pending_input(
        &self,
        _request: &PendingInputSubmitRequest,
    ) -> AicoreResult<PendingInputSubmitOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs not implemented yet".to_string(),
        ))
    }

    fn cancel_pending_input(
        &self,
        _request: &PendingInputCancelRequest,
    ) -> AicoreResult<PendingInputCancelOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs not implemented yet".to_string(),
        ))
    }

    fn request_stop_active_turn(
        &self,
        _request: &StopTurnRequest,
    ) -> AicoreResult<StopTurnOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "stop turn not implemented yet".to_string(),
        ))
    }

    fn create_approval(&self, _request: &CreateApprovalRequest) -> AicoreResult<ApprovalRecord> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals not implemented yet".to_string(),
        ))
    }

    fn respond_approval_first_writer_wins(
        &self,
        _request: &ApprovalResponseRequest,
    ) -> AicoreResult<ApprovalResponseOutcome> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses not implemented yet".to_string(),
        ))
    }

    fn invalidate_open_approvals_for_turn(
        &self,
        _instance_id: &InstanceId,
        _turn_id: &str,
        _status: ApprovalStatus,
    ) -> AicoreResult<u64> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval invalidation not implemented yet".to_string(),
        ))
    }

    fn create_pending_input(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs legacy placeholder".to_string(),
        ))
    }

    fn submit_approval(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals legacy placeholder".to_string(),
        ))
    }

    fn respond_approval(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses legacy placeholder".to_string(),
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

    fn get_pending_input(&self) -> AicoreResult<Option<PendingInputRecord>> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs not implemented yet".to_string(),
        ))
    }

    fn list_approvals_for_turn(&self, _turn_id: &str) -> AicoreResult<Vec<ApprovalRecord>> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals not implemented yet".to_string(),
        ))
    }

    fn list_approval_responses(
        &self,
        _approval_id: &str,
    ) -> AicoreResult<Vec<ApprovalResponseRecord>> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses not implemented yet".to_string(),
        ))
    }

    fn read_pending_inputs(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "pending_inputs legacy placeholder".to_string(),
        ))
    }

    fn read_approvals(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approvals legacy placeholder".to_string(),
        ))
    }

    fn read_approval_responses(&self) -> AicoreResult<()> {
        Err(aicore_foundation::AicoreError::Unavailable(
            "approval_responses legacy placeholder".to_string(),
        ))
    }
}

pub trait SessionLedger {
    fn instance_id(&self) -> &InstanceId;
    fn writer(&self) -> &dyn SessionLedgerWriter;
    fn reader(&self) -> &dyn SessionLedgerReader;
}
