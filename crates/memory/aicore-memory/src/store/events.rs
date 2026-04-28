use super::*;
#[cfg(test)]
use crate::store::schema::search_index_table_exists;
use crate::store::{
    codec::{event_kind_name, row_to_event},
    transaction::connect,
};

pub(super) async fn insert_event_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    sqlx::query(
        "INSERT INTO memory_events (
            event_id, event_kind, memory_id, proposal_id, scope_kind, instance_id,
            workspace_root, actor, reason, evidence_json, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&event.event_id)
    .bind(event_kind_name(&event.event_kind))
    .bind(&event.memory_id)
    .bind(&event.proposal_id)
    .bind(scope_kind(&event.scope))
    .bind(instance_id(&event.scope))
    .bind(workspace_root(&event.scope))
    .bind(&event.actor)
    .bind(&event.reason)
    .bind(&event.evidence_json)
    .bind(&event.created_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(())
}

pub async fn load_events(db_path: &Path) -> Result<Vec<MemoryEvent>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query("SELECT * FROM memory_events ORDER BY created_at ASC, event_id ASC")
        .fetch_all(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_event).collect()
}

#[cfg(test)]
pub async fn search_index_available(db_path: &Path) -> Result<bool, MemoryError> {
    let mut conn = connect(db_path).await?;
    search_index_table_exists(&mut conn).await
}
