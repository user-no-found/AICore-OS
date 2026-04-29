use aicore_foundation::AicoreResult;

use crate::envelope::EventEnvelope;
use crate::query::{EventGetRequest, EventGetResponse, EventQueryRequest, EventQueryResponse};

pub trait EventWriter {
    fn write(&self, envelope: &EventEnvelope) -> AicoreResult<()>;
}

pub trait EventReader {
    fn query(&self, request: &EventQueryRequest) -> AicoreResult<EventQueryResponse>;

    fn get(&self, request: &EventGetRequest) -> AicoreResult<EventGetResponse>;
}
