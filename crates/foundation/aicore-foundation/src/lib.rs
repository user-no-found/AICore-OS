pub mod cancellation;
pub mod error;
pub mod ids;
pub mod instance;
pub mod lease;
pub mod paths;
pub mod queue;
pub mod redaction;
pub mod time;

pub use cancellation::CancellationToken;
pub use error::{AicoreError, AicoreResult};
pub use ids::{
    AppId, CapabilityId, ComponentId, ContractId, ConversationId, EventId, InstanceId,
    InvocationId, RouteId, SessionId, TaskId, TurnId, WorkerId,
};
pub use instance::{
    DEFAULT_INSTANCE_SOUL, InstanceBinding, InstanceKind, InstancePaths, ensure_instance_layout,
    ensure_workspace_gitignore, instance_paths, resolve_instance_for_cwd,
};
pub use lease::{LeaseId, LeaseRecord, LeaseState};
pub use paths::{AicoreLayout, AicoreLayout as AicorePaths};
pub use queue::BoundedQueue;
pub use redaction::{RedactedText, redact_secret};
pub use time::{AicoreClock, SystemClock, Timestamp};

#[cfg(test)]
mod instance_tests;
