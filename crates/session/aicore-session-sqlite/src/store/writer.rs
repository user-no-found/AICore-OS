use aicore_foundation::{AicoreResult, InstanceId};
use aicore_session::traits::SessionLedgerWriter;
use aicore_session::types::{
    ActiveTurnAcquireOutcome, ActiveTurnAcquireRequest, ActiveTurnReleaseOutcome,
    ActiveTurnReleaseRequest, AppendControlEventRequest, AppendLedgerWriteRequest,
    AppendMessageRequest, ApprovalRecord, ApprovalResponseOutcome, ApprovalResponseRequest,
    ApprovalStatus, BeginTurnRequest, CreateApprovalRequest, CreateSessionRequest,
    FinishTurnRequest, PendingInputCancelOutcome, PendingInputCancelRequest,
    PendingInputSubmitOutcome, PendingInputSubmitRequest, SetRuntimeStateRequest, StopTurnOutcome,
    StopTurnRequest,
};

use crate::store::SqliteSessionStore;

impl SessionLedgerWriter for SqliteSessionStore {
    fn create_session(&self, request: &CreateSessionRequest) -> AicoreResult<()> {
        self.create_session_impl(request)
    }

    fn begin_turn(&self, request: &BeginTurnRequest) -> AicoreResult<()> {
        self.begin_turn_impl(request)
    }

    fn finish_turn(&self, request: &FinishTurnRequest) -> AicoreResult<()> {
        self.finish_turn_impl(request)
    }

    fn append_message(&self, request: &AppendMessageRequest) -> AicoreResult<()> {
        self.append_message_impl(request)
    }

    fn append_control_event(&self, request: &AppendControlEventRequest) -> AicoreResult<()> {
        self.append_control_event_impl(request)
    }

    fn append_ledger_write(&self, request: &AppendLedgerWriteRequest) -> AicoreResult<()> {
        self.append_ledger_write_impl(request)
    }

    fn set_runtime_state(&self, request: &SetRuntimeStateRequest) -> AicoreResult<()> {
        self.set_runtime_state_impl(request)
    }

    fn acquire_active_turn(
        &self,
        request: &ActiveTurnAcquireRequest,
    ) -> AicoreResult<ActiveTurnAcquireOutcome> {
        self.acquire_active_turn_impl(request)
    }

    fn release_active_turn(
        &self,
        request: &ActiveTurnReleaseRequest,
    ) -> AicoreResult<ActiveTurnReleaseOutcome> {
        self.release_active_turn_impl(request)
    }

    fn submit_or_replace_pending_input(
        &self,
        request: &PendingInputSubmitRequest,
    ) -> AicoreResult<PendingInputSubmitOutcome> {
        self.submit_or_replace_pending_input_impl(request)
    }

    fn cancel_pending_input(
        &self,
        request: &PendingInputCancelRequest,
    ) -> AicoreResult<PendingInputCancelOutcome> {
        self.cancel_pending_input_impl(request)
    }

    fn request_stop_active_turn(&self, request: &StopTurnRequest) -> AicoreResult<StopTurnOutcome> {
        self.request_stop_active_turn_impl(request)
    }

    fn create_approval(&self, request: &CreateApprovalRequest) -> AicoreResult<ApprovalRecord> {
        self.create_approval_impl(request)
    }

    fn respond_approval_first_writer_wins(
        &self,
        request: &ApprovalResponseRequest,
    ) -> AicoreResult<ApprovalResponseOutcome> {
        self.respond_approval_first_writer_wins_impl(request)
    }

    fn invalidate_open_approvals_for_turn(
        &self,
        instance_id: &InstanceId,
        turn_id: &str,
        status: ApprovalStatus,
    ) -> AicoreResult<u64> {
        self.invalidate_open_approvals_for_turn_impl(instance_id, turn_id, status)
    }

    fn create_pending_input(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("create_pending_input"))
    }

    fn submit_approval(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("submit_approval"))
    }

    fn respond_approval(&self) -> AicoreResult<()> {
        Err(crate::error::unsupported_api("respond_approval"))
    }
}
