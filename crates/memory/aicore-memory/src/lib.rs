mod agent;
mod ids;
mod kernel;
mod lock;
mod paths;
mod projection;
mod safety;
mod search;
mod store;
mod types;

pub use agent::RuleBasedMemoryAgent;
pub use ids::{MemoryEventId, MemoryId, MemoryProposalId};
pub use kernel::{
    MemoryKernel, build_core_projection_for_tests, build_decisions_projection_for_tests,
    build_permanent_projection_for_tests, build_status_projection_for_tests, default_memory_kernel,
};
pub use paths::MemoryPaths;
pub use safety::blocks_secret;
pub use search::{build_memory_pack, build_memory_pack_for_tests};
pub use types::{
    MemoryAgentOutput, MemoryAuditReport, MemoryEdge, MemoryError, MemoryEvent, MemoryEventKind,
    MemoryPermanence, MemoryProposal, MemoryProposalStatus, MemoryRecord, MemoryRequestedOutput,
    MemoryScope, MemorySource, MemoryStatus, MemoryTrigger, MemoryType, MemoryWorkBatch,
    ProjectionState, RememberInput, SearchQuery, SearchResult,
};

#[cfg(test)]
mod tests;
