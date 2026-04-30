use std::time::{SystemTime, UNIX_EPOCH};

use aicore_event::{EventStatus, RetentionClass};
use aicore_foundation::{AicoreError, AicoreResult, Timestamp};
use rusqlite::{Connection, Transaction, params};

use crate::error::{sqlite_schema_error, sqlite_write_error};
use crate::store::SqliteEventStore;

const THIRTY_DAYS_MILLIS: u128 = 30 * 24 * 60 * 60 * 1000;
const ONE_EIGHTY_DAYS_MILLIS: u128 = 180 * 24 * 60 * 60 * 1000;
const COMPRESSED_SUMMARY: &str = "compressed_event_record";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetentionSkipReason {
    Protected,
    TooNew,
    Uncompacted,
    InvalidClass,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionSkip {
    pub event_id: String,
    pub reason: RetentionSkipReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionPlan {
    pub scanned: usize,
    pub eligible_for_compaction: usize,
    pub eligible_for_delete: usize,
    pub protected_skipped: usize,
    pub too_new_skipped: usize,
    pub uncompacted_skipped: usize,
    pub invalid_class_skipped: usize,
    pub compaction_candidate_event_ids: Vec<String>,
    pub delete_candidate_event_ids: Vec<String>,
    pub skipped: Vec<RetentionSkip>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetentionApplyResult {
    pub run_id: String,
    pub scanned: usize,
    pub eligible_for_compaction: usize,
    pub compacted: usize,
    pub eligible_for_delete: usize,
    pub deleted: usize,
    pub protected_skipped: usize,
    pub too_new_skipped: usize,
    pub uncompacted_skipped: usize,
    pub invalid_class_skipped: usize,
    pub failed: usize,
    pub compacted_event_ids: Vec<String>,
    pub deleted_event_ids: Vec<String>,
    pub skipped: Vec<RetentionSkip>,
}

#[derive(Debug)]
struct RetentionRecord {
    event_id: String,
    recorded_at_millis: u128,
    retention_class: String,
    status: Option<String>,
    source_instance: String,
}

#[derive(Debug)]
enum RetentionDecision {
    Compact,
    Delete,
    Skip(RetentionSkipReason),
}

impl SqliteEventStore {
    pub fn plan_retention(&self, now: Timestamp) -> AicoreResult<RetentionPlan> {
        let connection = self.lock_connection()?;
        let records = load_retention_records(&connection)?;

        build_plan(&records, now, self.instance_id())
    }

    pub fn apply_retention(&self, now: Timestamp) -> AicoreResult<RetentionApplyResult> {
        let run_id = format!("run.{}", current_run_nonce());
        self.apply_retention_internal(now, &run_id)
    }

    #[cfg(test)]
    pub(crate) fn apply_retention_with_run_id(
        &self,
        now: Timestamp,
        run_id: &str,
    ) -> AicoreResult<RetentionApplyResult> {
        self.apply_retention_internal(now, run_id)
    }

    fn apply_retention_internal(
        &self,
        now: Timestamp,
        run_id: &str,
    ) -> AicoreResult<RetentionApplyResult> {
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction().map_err(sqlite_write_error)?;
        let records = load_retention_records(&tx)?;
        let plan = build_plan(&records, now, self.instance_id())?;

        let mut compacted_event_ids = Vec::new();
        for event_id in &plan.compaction_candidate_event_ids {
            compact_event(&tx, event_id)?;
            compacted_event_ids.push(event_id.clone());
        }

        let mut deleted_event_ids = Vec::new();
        for event_id in &plan.delete_candidate_event_ids {
            delete_event(&tx, event_id)?;
            deleted_event_ids.push(event_id.clone());
        }

        insert_compaction_run(
            &tx,
            run_id,
            now,
            plan.scanned,
            compacted_event_ids.len(),
            deleted_event_ids.len(),
        )?;
        tx.commit().map_err(sqlite_write_error)?;

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
}

fn build_plan(
    records: &[RetentionRecord],
    now: Timestamp,
    instance_id: &str,
) -> AicoreResult<RetentionPlan> {
    let mut plan = RetentionPlan {
        scanned: records.len(),
        ..RetentionPlan::default()
    };

    for record in records {
        match classify_record(record, now, instance_id)? {
            RetentionDecision::Compact => {
                plan.eligible_for_compaction += 1;
                plan.compaction_candidate_event_ids
                    .push(record.event_id.clone());
            }
            RetentionDecision::Delete => {
                plan.eligible_for_delete += 1;
                plan.delete_candidate_event_ids
                    .push(record.event_id.clone());
            }
            RetentionDecision::Skip(reason) => {
                match reason {
                    RetentionSkipReason::Protected => plan.protected_skipped += 1,
                    RetentionSkipReason::TooNew => plan.too_new_skipped += 1,
                    RetentionSkipReason::Uncompacted => plan.uncompacted_skipped += 1,
                    RetentionSkipReason::InvalidClass => plan.invalid_class_skipped += 1,
                }
                plan.skipped.push(RetentionSkip {
                    event_id: record.event_id.clone(),
                    reason,
                });
            }
        }
    }

    Ok(plan)
}

fn load_retention_records(conn: &Connection) -> AicoreResult<Vec<RetentionRecord>> {
    let mut stmt = conn
        .prepare(
            "SELECT event_id, recorded_at, retention_class, status, source_instance
             FROM events
             ORDER BY recorded_at, event_id",
        )
        .map_err(sqlite_schema_error)?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(sqlite_schema_error)?;

    rows.map(|row| {
        let (event_id, recorded_at, retention_class, status, source_instance) =
            row.map_err(sqlite_schema_error)?;
        let recorded_at_millis = recorded_at.parse::<u128>().map_err(|_| {
            AicoreError::InvalidState(format!(
                "invalid recorded_at in retention scan for {event_id}: {recorded_at}"
            ))
        })?;
        Ok(RetentionRecord {
            event_id,
            recorded_at_millis,
            retention_class,
            status,
            source_instance,
        })
    })
    .collect()
}

fn classify_record(
    record: &RetentionRecord,
    now: Timestamp,
    instance_id: &str,
) -> AicoreResult<RetentionDecision> {
    if record.source_instance != instance_id {
        return Err(AicoreError::Conflict(format!(
            "event source_instance mismatch during retention scan: expected {instance_id}, got {}",
            record.source_instance
        )));
    }

    let Some(retention_class) = parse_retention_class(&record.retention_class) else {
        return Ok(RetentionDecision::Skip(RetentionSkipReason::InvalidClass));
    };

    if matches!(
        retention_class,
        RetentionClass::Durable | RetentionClass::AuditPinned
    ) {
        return Ok(RetentionDecision::Skip(RetentionSkipReason::Protected));
    }

    let age = now.unix_millis().saturating_sub(record.recorded_at_millis);
    if age < THIRTY_DAYS_MILLIS {
        return Ok(RetentionDecision::Skip(RetentionSkipReason::TooNew));
    }

    match parse_status(record.status.as_deref())? {
        EventStatus::Compressed => {
            if age >= ONE_EIGHTY_DAYS_MILLIS {
                Ok(RetentionDecision::Delete)
            } else {
                Ok(RetentionDecision::Skip(RetentionSkipReason::TooNew))
            }
        }
        EventStatus::Recorded => Ok(RetentionDecision::Compact),
        EventStatus::Expired | EventStatus::Invalid => {
            Ok(RetentionDecision::Skip(RetentionSkipReason::Uncompacted))
        }
    }
}

fn parse_retention_class(value: &str) -> Option<RetentionClass> {
    match value {
        "ephemeral" => Some(RetentionClass::Ephemeral),
        "transient_30d" => Some(RetentionClass::Transient30d),
        "summary_180d" => Some(RetentionClass::Summary180d),
        "durable" => Some(RetentionClass::Durable),
        "audit_pinned" => Some(RetentionClass::AuditPinned),
        "needs_review" => Some(RetentionClass::NeedsReview),
        "invalid" => None,
        _ => None,
    }
}

fn parse_status(value: Option<&str>) -> AicoreResult<EventStatus> {
    match value.unwrap_or("recorded") {
        "recorded" => Ok(EventStatus::Recorded),
        "compressed" => Ok(EventStatus::Compressed),
        "expired" => Ok(EventStatus::Expired),
        "invalid" => Ok(EventStatus::Invalid),
        other => Err(AicoreError::InvalidState(format!(
            "unknown event status in retention scan: {other}"
        ))),
    }
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

fn current_run_nonce() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}
