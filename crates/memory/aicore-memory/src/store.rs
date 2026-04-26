use std::{path::Path, str::FromStr};

use sqlx::{
    Connection, Executor, Row,
    sqlite::{SqliteConnectOptions, SqliteConnection},
};

use crate::{
    projection::projection_state,
    search::{instance_id, scope_kind, workspace_root},
    types::{
        MemoryEdge, MemoryError, MemoryEvent, MemoryEventKind, MemoryPermanence, MemoryProposal,
        MemoryProposalStatus, MemoryRecord, MemoryScope, MemoryStatus, MemoryType, ProjectionState,
    },
};

fn connect_options(db_path: &Path) -> Result<SqliteConnectOptions, MemoryError> {
    SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
        .map_err(|error| MemoryError(error.to_string()))
        .map(|options| options.create_if_missing(true))
}

pub async fn connect(db_path: &Path) -> Result<SqliteConnection, MemoryError> {
    let options = connect_options(db_path)?;
    SqliteConnection::connect_with(&options)
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn init_schema(db_path: &Path) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_records (
            memory_id TEXT PRIMARY KEY,
            record_version INTEGER NOT NULL,
            memory_type TEXT NOT NULL,
            status TEXT NOT NULL,
            permanence TEXT NOT NULL,
            scope_kind TEXT NOT NULL,
            instance_id TEXT NOT NULL,
            workspace_root TEXT,
            content TEXT NOT NULL,
            content_language TEXT NOT NULL,
            normalized_content TEXT NOT NULL,
            normalized_language TEXT NOT NULL,
            localized_summary TEXT NOT NULL,
            source TEXT NOT NULL,
            evidence_json TEXT NOT NULL,
            state_key TEXT,
            state_version INTEGER NOT NULL,
            current_state TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_events (
            event_id TEXT PRIMARY KEY,
            event_kind TEXT NOT NULL,
            memory_id TEXT,
            proposal_id TEXT,
            scope_kind TEXT NOT NULL,
            instance_id TEXT NOT NULL,
            workspace_root TEXT,
            actor TEXT NOT NULL,
            reason TEXT,
            evidence_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_proposals (
            proposal_id TEXT PRIMARY KEY,
            memory_type TEXT NOT NULL,
            scope_kind TEXT NOT NULL,
            instance_id TEXT NOT NULL,
            workspace_root TEXT,
            source TEXT NOT NULL,
            status TEXT NOT NULL,
            content TEXT NOT NULL,
            content_language TEXT NOT NULL,
            normalized_content TEXT NOT NULL,
            normalized_language TEXT NOT NULL,
            localized_summary TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_edges (
            from_memory_id TEXT NOT NULL,
            to_memory_id TEXT NOT NULL,
            relation TEXT NOT NULL
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_snapshots (
            rev INTEGER PRIMARY KEY,
            core_markdown TEXT NOT NULL,
            status_markdown TEXT NOT NULL
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS memory_projection_state (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            stale INTEGER NOT NULL,
            warning TEXT,
            last_rebuild_at TEXT
        )",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    conn.execute(
        "INSERT OR IGNORE INTO memory_projection_state(singleton, stale, warning, last_rebuild_at)
         VALUES (1, 0, NULL, NULL)",
    )
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    let _ = conn
        .execute("ALTER TABLE memory_projection_state ADD COLUMN last_rebuild_at TEXT")
        .await;

    let _ = ensure_search_index_table(&mut conn).await;

    Ok(())
}

pub async fn table_names(db_path: &Path) -> Result<Vec<String>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .fetch_all(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    Ok(rows.into_iter().map(|row| row.get("name")).collect())
}

pub async fn rebuild_search_index(db_path: &Path) -> Result<bool, MemoryError> {
    let mut conn = connect(db_path).await?;
    if !ensure_search_index_table(&mut conn).await? {
        return Ok(false);
    }

    conn.execute("DELETE FROM memory_records_fts")
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let rows = sqlx::query(
        "SELECT memory_id, content, normalized_content, localized_summary
         FROM memory_records
         WHERE status = 'active'",
    )
    .fetch_all(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    for row in rows {
        sqlx::query(
            "INSERT INTO memory_records_fts(memory_id, content, normalized_content, localized_summary)
             VALUES (?, ?, ?, ?)",
        )
        .bind(row.get::<String, _>("memory_id"))
        .bind(row.get::<String, _>("content"))
        .bind(row.get::<String, _>("normalized_content"))
        .bind(row.get::<String, _>("localized_summary"))
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    }

    Ok(true)
}

#[cfg(test)]
pub async fn search_index_available(db_path: &Path) -> Result<bool, MemoryError> {
    let mut conn = connect(db_path).await?;
    search_index_table_exists(&mut conn).await
}

pub async fn search_index_candidates(
    db_path: &Path,
    query_text: &str,
    limit: Option<usize>,
) -> Result<Option<Vec<String>>, MemoryError> {
    let mut conn = connect(db_path).await?;
    if !search_index_table_exists(&mut conn).await? {
        return Ok(None);
    }

    let sanitized = query_text.replace('"', " ").trim().to_string();
    if sanitized.is_empty() {
        return Ok(Some(Vec::new()));
    }

    let search_limit = i64::try_from(limit.unwrap_or(64)).unwrap_or(64);
    let query = format!("\"{sanitized}\"");
    let rows = match sqlx::query(
        "SELECT memory_id
         FROM memory_records_fts
         WHERE memory_records_fts MATCH ?
         LIMIT ?",
    )
    .bind(query)
    .bind(search_limit)
    .fetch_all(&mut conn)
    .await
    {
        Ok(rows) => rows,
        Err(_) => return Ok(None),
    };

    Ok(Some(
        rows.into_iter()
            .map(|row| row.get::<String, _>("memory_id"))
            .collect(),
    ))
}

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

pub async fn insert_proposal_and_event(
    db_path: &Path,
    proposal: &MemoryProposal,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    let scope = match &proposal.scope {
        MemoryScope::GlobalMain { instance_id } => MemoryScope::GlobalMain {
            instance_id: instance_id.clone(),
        },
        MemoryScope::Workspace {
            instance_id,
            workspace_root,
        } => MemoryScope::Workspace {
            instance_id: instance_id.clone(),
            workspace_root: workspace_root.clone(),
        },
    };

    sqlx::query(
        "INSERT INTO memory_proposals (
            proposal_id, memory_type, scope_kind, instance_id, workspace_root,
            source, status, content, content_language, normalized_content, normalized_language,
            localized_summary, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&proposal.proposal_id)
    .bind(memory_type_name(&proposal.memory_type))
    .bind(scope_kind(&scope))
    .bind(instance_id(&scope))
    .bind(workspace_root(&scope))
    .bind(memory_source_name(&proposal.source))
    .bind(proposal_status_name(&proposal.status))
    .bind(&proposal.content)
    .bind(&proposal.content_language)
    .bind(&proposal.normalized_content)
    .bind(&proposal.normalized_language)
    .bind(&proposal.localized_summary)
    .bind(&proposal.created_at)
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

pub async fn accept_proposal(
    db_path: &Path,
    proposal_id: &str,
    record: &MemoryRecord,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_proposals
         SET status = ?
         WHERE proposal_id = ? AND status = ?",
    )
    .bind(proposal_status_name(&MemoryProposalStatus::Accepted))
    .bind(proposal_id)
    .bind(proposal_status_name(&MemoryProposalStatus::Open))
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
    }

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

pub async fn reject_proposal(
    db_path: &Path,
    proposal_id: &str,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_proposals
         SET status = ?
         WHERE proposal_id = ? AND status = ?",
    )
    .bind(proposal_status_name(&MemoryProposalStatus::Rejected))
    .bind(proposal_id)
    .bind(proposal_status_name(&MemoryProposalStatus::Open))
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
    }

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

async fn insert_event_tx(
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

pub async fn load_records(db_path: &Path) -> Result<Vec<MemoryRecord>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query("SELECT * FROM memory_records ORDER BY created_at ASC, memory_id ASC")
        .fetch_all(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_record).collect()
}

pub async fn load_proposals(db_path: &Path) -> Result<Vec<MemoryProposal>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows =
        sqlx::query("SELECT * FROM memory_proposals ORDER BY created_at ASC, proposal_id ASC")
            .fetch_all(&mut conn)
            .await
            .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_proposal).collect()
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

pub async fn load_events(db_path: &Path) -> Result<Vec<MemoryEvent>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows = sqlx::query("SELECT * FROM memory_events ORDER BY created_at ASC, event_id ASC")
        .fetch_all(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_event).collect()
}

pub async fn load_projection_state(db_path: &Path) -> Result<ProjectionState, MemoryError> {
    let mut conn = connect(db_path).await?;
    let row = sqlx::query(
        "SELECT stale, warning, last_rebuild_at FROM memory_projection_state WHERE singleton = 1",
    )
    .fetch_one(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(projection_state(
        row.get::<i64, _>("stale") != 0,
        row.get::<Option<String>, _>("warning"),
        row.get::<Option<String>, _>("last_rebuild_at"),
    ))
}

pub async fn save_projection_state(
    db_path: &Path,
    state: &ProjectionState,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query(
        "UPDATE memory_projection_state SET stale = ?, warning = ?, last_rebuild_at = ? WHERE singleton = 1",
    )
        .bind(if state.stale { 1 } else { 0 })
        .bind(&state.warning)
        .bind(&state.last_rebuild_at)
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    Ok(())
}

#[cfg(test)]
pub async fn drop_search_index_for_tests(db_path: &Path) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    conn.execute("DROP TABLE IF EXISTS memory_records_fts")
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
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
pub async fn delete_proposal_for_tests(
    db_path: &Path,
    proposal_id: &str,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query("DELETE FROM memory_proposals WHERE proposal_id = ?")
        .bind(proposal_id)
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

fn row_to_scope(row: &sqlx::sqlite::SqliteRow) -> Result<MemoryScope, MemoryError> {
    let scope_kind = row.get::<String, _>("scope_kind");
    let instance_id = row.get::<String, _>("instance_id");
    let workspace_root = row.get::<Option<String>, _>("workspace_root");

    match scope_kind.as_str() {
        "global_main" => Ok(MemoryScope::GlobalMain { instance_id }),
        "workspace" => Ok(MemoryScope::Workspace {
            instance_id,
            workspace_root: workspace_root.unwrap_or_default(),
        }),
        _ => Err(MemoryError(format!("unknown scope_kind: {scope_kind}"))),
    }
}

async fn ensure_search_index_table(conn: &mut SqliteConnection) -> Result<bool, MemoryError> {
    match conn
        .execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_records_fts
             USING fts5(memory_id UNINDEXED, content, normalized_content, localized_summary)",
        )
        .await
    {
        Ok(_) => Ok(true),
        Err(error) => {
            let message = error.to_string();
            if message.contains("fts5") || message.contains("no such module") {
                Ok(false)
            } else {
                Err(MemoryError(message))
            }
        }
    }
}

async fn search_index_table_exists(conn: &mut SqliteConnection) -> Result<bool, MemoryError> {
    let row = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type IN ('table', 'virtual table') AND name = 'memory_records_fts'",
    )
    .fetch_optional(conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(row.is_some())
}

fn row_to_record(row: sqlx::sqlite::SqliteRow) -> Result<MemoryRecord, MemoryError> {
    Ok(MemoryRecord {
        memory_id: row.get("memory_id"),
        record_version: row.get("record_version"),
        memory_type: parse_memory_type(&row.get::<String, _>("memory_type"))?,
        status: parse_memory_status(&row.get::<String, _>("status"))?,
        permanence: parse_memory_permanence(&row.get::<String, _>("permanence"))?,
        scope: row_to_scope(&row)?,
        content: row.get("content"),
        content_language: row.get("content_language"),
        normalized_content: row.get("normalized_content"),
        normalized_language: row.get("normalized_language"),
        localized_summary: row.get("localized_summary"),
        source: parse_memory_source(&row.get::<String, _>("source"))?,
        evidence_json: row.get("evidence_json"),
        state_key: row.get("state_key"),
        state_version: row.get("state_version"),
        current_state: row.get("current_state"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_proposal(row: sqlx::sqlite::SqliteRow) -> Result<MemoryProposal, MemoryError> {
    Ok(MemoryProposal {
        proposal_id: row.get("proposal_id"),
        memory_type: parse_memory_type(&row.get::<String, _>("memory_type"))?,
        scope: row_to_scope(&row)?,
        source: parse_memory_source(&row.get::<String, _>("source"))?,
        status: parse_proposal_status(&row.get::<String, _>("status"))?,
        content: row.get("content"),
        content_language: row.get("content_language"),
        normalized_content: row.get("normalized_content"),
        normalized_language: row.get("normalized_language"),
        localized_summary: row.get("localized_summary"),
        created_at: row.get("created_at"),
    })
}

fn row_to_event(row: sqlx::sqlite::SqliteRow) -> Result<MemoryEvent, MemoryError> {
    Ok(MemoryEvent {
        event_id: row.get("event_id"),
        event_kind: parse_event_kind(&row.get::<String, _>("event_kind"))?,
        memory_id: row.get("memory_id"),
        proposal_id: row.get("proposal_id"),
        scope: row_to_scope(&row)?,
        actor: row.get("actor"),
        reason: row.get("reason"),
        evidence_json: row.get("evidence_json"),
        created_at: row.get("created_at"),
    })
}

fn memory_type_name(value: &MemoryType) -> &'static str {
    match value {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

fn memory_status_name(value: &MemoryStatus) -> &'static str {
    match value {
        MemoryStatus::Active => "active",
        MemoryStatus::Superseded => "superseded",
        MemoryStatus::Invalidated => "invalidated",
        MemoryStatus::Archived => "archived",
        MemoryStatus::Forgotten => "forgotten",
    }
}

fn memory_permanence_name(value: &MemoryPermanence) -> &'static str {
    match value {
        MemoryPermanence::Standard => "standard",
        MemoryPermanence::Permanent => "permanent",
    }
}

fn memory_source_name(value: &MemorySource) -> &'static str {
    match value {
        MemorySource::UserExplicit => "user_explicit",
        MemorySource::UserCorrection => "user_correction",
        MemorySource::AssistantSummary => "assistant_summary",
        MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

fn proposal_status_name(value: &MemoryProposalStatus) -> &'static str {
    match value {
        MemoryProposalStatus::Open => "open",
        MemoryProposalStatus::Accepted => "accepted",
        MemoryProposalStatus::Rejected => "rejected",
    }
}

fn event_kind_name(value: &MemoryEventKind) -> &'static str {
    match value {
        MemoryEventKind::Accepted => "accepted",
        MemoryEventKind::Proposed => "proposed",
        MemoryEventKind::Rejected => "rejected",
        MemoryEventKind::Corrected => "corrected",
        MemoryEventKind::Archived => "archived",
        MemoryEventKind::Forgotten => "forgotten",
    }
}

fn parse_memory_type(value: &str) -> Result<MemoryType, MemoryError> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(MemoryError(format!("unknown memory_type: {value}"))),
    }
}

fn parse_memory_status(value: &str) -> Result<MemoryStatus, MemoryError> {
    match value {
        "active" => Ok(MemoryStatus::Active),
        "superseded" => Ok(MemoryStatus::Superseded),
        "invalidated" => Ok(MemoryStatus::Invalidated),
        "archived" => Ok(MemoryStatus::Archived),
        "forgotten" => Ok(MemoryStatus::Forgotten),
        _ => Err(MemoryError(format!("unknown memory_status: {value}"))),
    }
}

fn parse_memory_permanence(value: &str) -> Result<MemoryPermanence, MemoryError> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(MemoryError(format!("unknown memory_permanence: {value}"))),
    }
}

fn parse_memory_source(value: &str) -> Result<MemorySource, MemoryError> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(MemoryError(format!("unknown memory_source: {value}"))),
    }
}

fn parse_proposal_status(value: &str) -> Result<MemoryProposalStatus, MemoryError> {
    match value {
        "open" => Ok(MemoryProposalStatus::Open),
        "accepted" => Ok(MemoryProposalStatus::Accepted),
        "rejected" => Ok(MemoryProposalStatus::Rejected),
        _ => Err(MemoryError(format!("unknown proposal_status: {value}"))),
    }
}

fn parse_event_kind(value: &str) -> Result<MemoryEventKind, MemoryError> {
    match value {
        "accepted" => Ok(MemoryEventKind::Accepted),
        "proposed" => Ok(MemoryEventKind::Proposed),
        "rejected" => Ok(MemoryEventKind::Rejected),
        "corrected" => Ok(MemoryEventKind::Corrected),
        "archived" => Ok(MemoryEventKind::Archived),
        "forgotten" => Ok(MemoryEventKind::Forgotten),
        _ => Err(MemoryError(format!("unknown event_kind: {value}"))),
    }
}

use crate::types::MemorySource;
