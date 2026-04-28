mod codec;
mod events;
mod proposals;
mod records;
mod schema;
mod state;
mod transaction;

use std::path::Path;

use sqlx::{
    Executor, Row,
    sqlite::{SqliteConnectOptions, SqliteConnection, SqliteRow},
};

use crate::{
    projection::projection_state,
    search::{instance_id, scope_kind, workspace_root},
    types::{
        MemoryEdge, MemoryError, MemoryEvent, MemoryEventKind, MemoryPermanence, MemoryProposal,
        MemoryProposalStatus, MemoryRecord, MemoryScope, MemorySource, MemoryStatus, MemoryType,
        ProjectionState,
    },
};

pub use events::load_events;
#[cfg(test)]
pub use events::search_index_available;
#[cfg(test)]
pub use proposals::delete_proposal_for_tests;
pub use proposals::{accept_proposal, insert_proposal_and_event, load_proposals, reject_proposal};
#[cfg(test)]
pub use records::{
    delete_edge_for_tests, delete_record_for_tests, force_normalized_content_for_tests,
    force_record_status_for_tests,
};
pub use records::{
    insert_record_and_event, load_edges, load_records, supersede_record, update_record_status,
};
#[cfg(test)]
pub use schema::drop_search_index_for_tests;
pub use schema::{init_schema, rebuild_search_index, search_index_candidates, table_names};
pub use state::{load_projection_state, save_projection_state};
