use aicore_foundation::AicoreResult;
use aicore_session::traits::SessionLedgerWriter;
use aicore_session::types::{
    AppendControlEventRequest, AppendLedgerWriteRequest, AppendMessageRequest, BeginTurnRequest,
    CreateSessionRequest, FinishTurnRequest, SetRuntimeStateRequest,
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
