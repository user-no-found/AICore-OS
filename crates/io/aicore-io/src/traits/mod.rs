use aicore_foundation::AicoreResult;

use crate::types::{
    AcknowledgeEventRequest, AcknowledgeEventResponse, AttachInstanceRequest,
    AttachInstanceResponse, BindInstanceRequest, BindInstanceResponse, DetachInstanceRequest,
    DetachInstanceResponse, GetCurrentSnapshotRequest, GetCurrentSnapshotResponse, IoEventEnvelope,
    StopTurnRequest, StopTurnResponse, SubmitInputRequest, SubmitInputResponse,
};

pub trait InstanceIoReader {
    fn get_current_snapshot(
        &self,
        request: &GetCurrentSnapshotRequest,
    ) -> AicoreResult<GetCurrentSnapshotResponse>;
}

pub trait InstanceIoWriter {
    fn submit_input(&self, request: &SubmitInputRequest) -> AicoreResult<SubmitInputResponse>;
    fn stop_turn(&self, request: &StopTurnRequest) -> AicoreResult<StopTurnResponse>;
    fn acknowledge_event(
        &self,
        request: &AcknowledgeEventRequest,
    ) -> AicoreResult<AcknowledgeEventResponse>;
}

pub trait InstanceIoGateway: InstanceIoReader + InstanceIoWriter {
    fn bind_instance(&self, request: &BindInstanceRequest) -> AicoreResult<BindInstanceResponse>;
    fn attach_instance(
        &self,
        request: &AttachInstanceRequest,
    ) -> AicoreResult<AttachInstanceResponse>;
    fn detach_instance(
        &self,
        request: &DetachInstanceRequest,
    ) -> AicoreResult<DetachInstanceResponse>;
    fn next_event(&self) -> AicoreResult<Option<IoEventEnvelope>>;
}
