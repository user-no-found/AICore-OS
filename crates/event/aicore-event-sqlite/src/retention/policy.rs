use aicore_event::{EventStatus, RetentionClass};
use aicore_foundation::{AicoreError, AicoreResult, Timestamp};
use rusqlite::Connection;

use crate::error::sqlite_schema_error;

use super::types::{RetentionPlan, RetentionSkip, RetentionSkipReason};

const THIRTY_DAYS_MILLIS: u128 = 30 * 24 * 60 * 60 * 1000;
const ONE_EIGHTY_DAYS_MILLIS: u128 = 180 * 24 * 60 * 60 * 1000;

#[derive(Debug)]
pub(crate) struct RetentionRecord {
    pub(crate) event_id: String,
    pub(crate) recorded_at_millis: u128,
    pub(crate) retention_class: String,
    pub(crate) status: Option<String>,
    pub(crate) source_instance: String,
}

#[derive(Debug)]
enum RetentionDecision {
    Compact,
    Delete,
    Skip(RetentionSkipReason),
}

pub(crate) fn build_plan(
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

pub(crate) fn load_retention_records(conn: &Connection) -> AicoreResult<Vec<RetentionRecord>> {
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
