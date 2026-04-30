use std::time::{SystemTime, UNIX_EPOCH};

use aicore_foundation::{AicoreResult, Timestamp};
use rusqlite::{Transaction, params};

use crate::error::sqlite_write_error;

use super::types::{RetentionApplyResult, RetentionPlan};

const COMPRESSED_SUMMARY: &str = "compressed_event_record";

pub(crate) fn apply_plan(
    tx: &Transaction<'_>,
    run_id: &str,
    now: Timestamp,
    plan: RetentionPlan,
) -> AicoreResult<RetentionApplyResult> {
    let mut compacted_event_ids = Vec::new();
    for event_id in &plan.compaction_candidate_event_ids {
        compact_event(tx, event_id)?;
        compacted_event_ids.push(event_id.clone());
    }

    let mut deleted_event_ids = Vec::new();
    for event_id in &plan.delete_candidate_event_ids {
        delete_event(tx, event_id)?;
        deleted_event_ids.push(event_id.clone());
    }

    insert_compaction_run(
        tx,
        run_id,
        now,
        plan.scanned,
        compacted_event_ids.len(),
        deleted_event_ids.len(),
    )?;

    Ok(RetentionApplyResult {
        run_id: run_id.to_string(),
        scanned: plan.scanned,
        eligible_for_compaction: plan.eligible_for_compaction,
        compacted: compacted_event_ids.len(),
        eligible_for_delete: plan.eligible_for_delete,
        deleted: deleted_event_ids.len(),
        protected_skipped: plan.protected_skipped,
        too_new_skipped: plan.too_new_skipped,
        uncompacted_skipped: plan.uncompacted_skipped,
        invalid_class_skipped: plan.invalid_class_skipped,
        failed: 0,
        compacted_event_ids,
        deleted_event_ids,
        skipped: plan.skipped,
    })
}

pub(crate) fn current_run_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn compact_event(tx: &Transaction<'_>, event_id: &str) -> AicoreResult<()> {
    tx.execute(
        "UPDATE events
         SET summary = ?2, status = 'compressed', evidence_ref = NULL, payload_ref = NULL
         WHERE event_id = ?1",
        params![event_id, COMPRESSED_SUMMARY],
    )
    .map_err(sqlite_write_error)?;
    tx.execute(
        "DELETE FROM event_refs WHERE event_id = ?1",
        params![event_id],
    )
    .map_err(sqlite_write_error)?;
    Ok(())
}

fn delete_event(tx: &Transaction<'_>, event_id: &str) -> AicoreResult<()> {
    tx.execute("DELETE FROM events WHERE event_id = ?1", params![event_id])
        .map_err(sqlite_write_error)?;
    Ok(())
}

fn insert_compaction_run(
    tx: &Transaction<'_>,
    run_id: &str,
    now: Timestamp,
    scanned: usize,
    compacted: usize,
    deleted: usize,
) -> AicoreResult<()> {
    let now_text = now.unix_millis().to_string();
    tx.execute(
        "INSERT INTO compaction_runs (
            run_id, started_at, finished_at, status,
            records_scanned, records_compressed, records_deleted, error_summary
         ) VALUES (?1, ?2, ?3, 'completed', ?4, ?5, ?6, NULL)",
        params![
            run_id,
            now_text,
            now_text,
            scanned as i64,
            compacted as i64,
            deleted as i64
        ],
    )
    .map_err(sqlite_write_error)?;
    Ok(())
}
