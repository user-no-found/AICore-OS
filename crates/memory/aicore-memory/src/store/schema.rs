use super::*;
use crate::store::transaction::connect;

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

#[cfg(test)]
pub async fn drop_search_index_for_tests(db_path: &Path) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    conn.execute("DROP TABLE IF EXISTS memory_records_fts")
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}

pub(super) async fn ensure_search_index_table(
    conn: &mut SqliteConnection,
) -> Result<bool, MemoryError> {
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

pub(super) async fn search_index_table_exists(
    conn: &mut SqliteConnection,
) -> Result<bool, MemoryError> {
    let row = sqlx::query(
        "SELECT name FROM sqlite_master WHERE type IN ('table', 'virtual table') AND name = 'memory_records_fts'",
    )
    .fetch_optional(conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(row.is_some())
}
