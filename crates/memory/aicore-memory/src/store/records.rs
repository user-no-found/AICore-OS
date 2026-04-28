use super::*;
use crate::store::{
    codec::{
        memory_permanence_name, memory_source_name, memory_status_name, memory_type_name,
        row_to_record,
    },
    events::insert_event_tx,
    transaction::connect,
};
use sqlx::Connection;

pub async fn insert_record_and_event(
    db_path: &Path,
    record: &MemoryRecord,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    sqlx::query(
        "INSERT INTO memory_records (
            memory_id, record_version, memory_type, status, permanence, scope_kind, instance_id, workspace_root,
            content, content_language, normalized_content, normalized_language, localized_summary,
            source, evidence_json, state_key, state_version, current_state, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.memory_id)
    .bind(record.record_version)
    .bind(memory_type_name(&record.memory_type))
    .bind(memory_status_name(&record.status))
    .bind(memory_permanence_name(&record.permanence))
    .bind(scope_kind(&record.scope))
    .bind(instance_id(&record.scope))
    .bind(workspace_root(&record.scope))
    .bind(&record.content)
    .bind(&record.content_language)
    .bind(&record.normalized_content)
    .bind(&record.normalized_language)
    .bind(&record.localized_summary)
    .bind(memory_source_name(&record.source))
    .bind(&record.evidence_json)
    .bind(&record.state_key)
    .bind(record.state_version)
    .bind(&record.current_state)
    .bind(&record.created_at)
    .bind(&record.updated_at)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn supersede_record(
    db_path: &Path,
    old_memory_id: &str,
    expected_version: i64,
    new_record: &MemoryRecord,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_records
         SET status = ?, updated_at = ?, record_version = record_version + 1
         WHERE memory_id = ? AND record_version = ?",
    )
    .bind(memory_status_name(&MemoryStatus::Superseded))
    .bind(&new_record.updated_at)
    .bind(old_memory_id)
    .bind(expected_version)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!(
            "stale memory version: {old_memory_id}"
        )));
    }

    sqlx::query(
        "INSERT INTO memory_records (
            memory_id, record_version, memory_type, status, permanence, scope_kind, instance_id, workspace_root,
            content, content_language, normalized_content, normalized_language, localized_summary,
            source, evidence_json, state_key, state_version, current_state, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&new_record.memory_id)
    .bind(new_record.record_version)
    .bind(memory_type_name(&new_record.memory_type))
    .bind(memory_status_name(&new_record.status))
    .bind(memory_permanence_name(&new_record.permanence))
    .bind(scope_kind(&new_record.scope))
    .bind(instance_id(&new_record.scope))
    .bind(workspace_root(&new_record.scope))
    .bind(&new_record.content)
    .bind(&new_record.content_language)
    .bind(&new_record.normalized_content)
    .bind(&new_record.normalized_language)
    .bind(&new_record.localized_summary)
    .bind(memory_source_name(&new_record.source))
    .bind(&new_record.evidence_json)
    .bind(&new_record.state_key)
    .bind(new_record.state_version)
    .bind(&new_record.current_state)
    .bind(&new_record.created_at)
    .bind(&new_record.updated_at)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    sqlx::query(
        "INSERT INTO memory_edges (from_memory_id, to_memory_id, relation) VALUES (?, ?, ?)",
    )
    .bind(&new_record.memory_id)
    .bind(old_memory_id)
    .bind("supersedes")
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn update_record_status(
    db_path: &Path,
    memory_id: &str,
    expected_version: i64,
    status: MemoryStatus,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_records
         SET status = ?, updated_at = ?, record_version = record_version + 1
         WHERE memory_id = ? AND record_version = ?",
    )
    .bind(memory_status_name(&status))
    .bind(&event.created_at)
    .bind(memory_id)
    .bind(expected_version)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!("stale memory version: {memory_id}")));
    }

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn load_records(db_path: &Path) -> Result<Vec<MemoryRecord>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query("SELECT * FROM memory_records ORDER BY created_at ASC, memory_id ASC")
        .fetch_all(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_record).collect()
}

pub async fn load_edges(db_path: &Path) -> Result<Vec<MemoryEdge>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query(
        "SELECT from_memory_id, to_memory_id, relation FROM memory_edges ORDER BY rowid ASC",
    )
    .fetch_all(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter()
        .map(|row| {
            Ok(MemoryEdge {
                from_memory_id: row.get("from_memory_id"),
                to_memory_id: row.get("to_memory_id"),
                relation: row.get("relation"),
            })
        })
        .collect()
}

#[cfg(test)]
pub async fn delete_record_for_tests(db_path: &Path, memory_id: &str) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query("DELETE FROM memory_records WHERE memory_id = ?")
        .bind(memory_id)
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
pub async fn delete_edge_for_tests(
    db_path: &Path,
    from_memory_id: &str,
    to_memory_id: &str,
    relation: &str,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query(
        "DELETE FROM memory_edges WHERE from_memory_id = ? AND to_memory_id = ? AND relation = ?",
    )
    .bind(from_memory_id)
    .bind(to_memory_id)
    .bind(relation)
    .execute(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
pub async fn force_record_status_for_tests(
    db_path: &Path,
    memory_id: &str,
    status: MemoryStatus,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query("UPDATE memory_records SET status = ? WHERE memory_id = ?")
        .bind(memory_status_name(&status))
        .bind(memory_id)
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}

#[cfg(test)]
pub async fn force_normalized_content_for_tests(
    db_path: &Path,
    memory_id: &str,
    normalized_content: &str,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query("UPDATE memory_records SET normalized_content = ? WHERE memory_id = ?")
        .bind(normalized_content)
        .bind(memory_id)
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}
