use aicore_event::{
    EventGetRequest, EventReader, EventStatus, EventTag, EventTagSet, EventWriter, RetentionClass,
};
use aicore_foundation::Timestamp;

use crate::RetentionSkipReason;
use crate::tests::{
    count_rows, open_sqlite, open_store, sample_compressed_envelope,
    sample_envelope_with_optionals, sample_envelope_with_retention, temp_db_path,
};

const DAY_MILLIS: u128 = 24 * 60 * 60 * 1000;
const THIRTY_DAYS: u128 = 30 * DAY_MILLIS;
const ONE_EIGHTY_DAYS: u128 = 180 * DAY_MILLIS;
const BASE_TIME: u128 = 1_800_000_000_000;

#[test]
fn plan_retention_is_side_effect_free_and_uses_recorded_at() {
    let path = temp_db_path("retention-plan-side-effect-free");
    let store = open_store(&path);

    let old_occurred_fresh_recorded = sample_envelope_with_retention(
        "evt.retention.001",
        BASE_TIME - ONE_EIGHTY_DAYS - DAY_MILLIS,
        BASE_TIME - DAY_MILLIS,
        RetentionClass::Transient30d,
    );
    let fresh_occurred_old_recorded = sample_envelope_with_retention(
        "evt.retention.002",
        BASE_TIME - DAY_MILLIS,
        BASE_TIME - THIRTY_DAYS,
        RetentionClass::Transient30d,
    );

    store
        .write(&old_occurred_fresh_recorded)
        .expect("first write should succeed");
    store
        .write(&fresh_occurred_old_recorded)
        .expect("second write should succeed");

    let conn = open_sqlite(&path);
    let before_event_refs = count_rows(&conn, "event_refs");
    let before_compaction_runs = count_rows(&conn, "compaction_runs");
    drop(conn);

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");

    assert_eq!(plan.scanned, 2);
    assert_eq!(plan.eligible_for_compaction, 1);
    assert_eq!(plan.eligible_for_delete, 0);
    assert_eq!(
        plan.compaction_candidate_event_ids,
        vec!["evt.retention.002".to_string()]
    );
    assert!(plan.delete_candidate_event_ids.is_empty());
    assert!(plan.skipped.iter().any(|skip| {
        skip.event_id == "evt.retention.001" && skip.reason == RetentionSkipReason::TooNew
    }));

    let conn = open_sqlite(&path);
    assert_eq!(count_rows(&conn, "event_refs"), before_event_refs);
    assert_eq!(count_rows(&conn, "compaction_runs"), before_compaction_runs);
    let response = store
        .get(&EventGetRequest::new("evt.retention.002"))
        .expect("get should succeed");
    let event = response.event.expect("event should still exist");
    assert_eq!(event.summary, "summary for evt.retention.002");
    assert_eq!(event.status, Some(EventStatus::Recorded));
}

#[test]
fn apply_retention_compacts_recorded_event_and_writes_run_summary() {
    let path = temp_db_path("retention-apply-compact");
    let store = open_store(&path);
    let envelope = sample_envelope_with_optionals();

    store.write(&envelope).expect("write should succeed");

    let result = store
        .apply_retention(Timestamp::from_unix_millis(
            envelope.recorded_at.unix_millis() + THIRTY_DAYS,
        ))
        .expect("apply should succeed");

    assert_eq!(result.scanned, 1);
    assert_eq!(result.compacted, 1);
    assert_eq!(result.deleted, 0);
    assert_eq!(result.compacted_event_ids, vec!["evt.002".to_string()]);
    assert!(result.deleted_event_ids.is_empty());

    let response = store
        .get(&EventGetRequest::new("evt.002"))
        .expect("get should succeed");
    let event = response.event.expect("event should exist");
    assert_eq!(event.summary, "compressed_event_record");
    assert_eq!(event.status, Some(EventStatus::Compressed));
    assert_eq!(event.evidence_ref, None);
    assert_eq!(event.payload_ref, None);
    assert_eq!(event.tag_set.tags, envelope.tag_set.tags);
    assert_eq!(event.tag_set.confirmed, envelope.tag_set.confirmed);

    let conn = open_sqlite(&path);
    assert_eq!(count_rows(&conn, "event_refs"), 0);
    assert_eq!(count_rows(&conn, "compaction_runs"), 1);
    let (scanned, compressed, deleted, error_summary): (i64, i64, i64, Option<String>) = conn
        .query_row(
            "SELECT records_scanned, records_compressed, records_deleted, error_summary FROM compaction_runs LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("compaction run should load");
    assert_eq!(scanned, 1);
    assert_eq!(compressed, 1);
    assert_eq!(deleted, 0);
    assert_eq!(error_summary, None);
}

#[test]
fn recorded_at_boundaries_control_compaction_and_deletion() {
    let path = temp_db_path("retention-recorded-at-boundaries");
    let store = open_store(&path);

    store
        .write(&sample_envelope_with_retention(
            "evt.retention.029",
            BASE_TIME - ONE_EIGHTY_DAYS,
            BASE_TIME - THIRTY_DAYS + 1,
            RetentionClass::Transient30d,
        ))
        .expect("29d23h59m event should write");
    store
        .write(&sample_envelope_with_retention(
            "evt.retention.030",
            BASE_TIME,
            BASE_TIME - THIRTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("30d event should write");
    store
        .write(&sample_envelope_with_retention(
            "evt.retention.030plus",
            BASE_TIME,
            BASE_TIME - THIRTY_DAYS - 1,
            RetentionClass::Transient30d,
        ))
        .expect("30d+1ms event should write");
    store
        .write(&sample_compressed_envelope(
            "evt.retention.179",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS + 1,
            RetentionClass::Transient30d,
        ))
        .expect("179d compressed event should write");
    store
        .write(&sample_compressed_envelope(
            "evt.retention.180",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("180d compressed event should write");
    store
        .write(&sample_compressed_envelope(
            "evt.retention.180plus",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS - 1,
            RetentionClass::Transient30d,
        ))
        .expect("180d+1ms compressed event should write");

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");

    assert!(
        plan.compaction_candidate_event_ids
            .contains(&"evt.retention.030".to_string())
    );
    assert!(
        plan.compaction_candidate_event_ids
            .contains(&"evt.retention.030plus".to_string())
    );
    assert!(
        !plan
            .compaction_candidate_event_ids
            .contains(&"evt.retention.029".to_string())
    );
    assert!(
        plan.delete_candidate_event_ids
            .contains(&"evt.retention.180".to_string())
    );
    assert!(
        plan.delete_candidate_event_ids
            .contains(&"evt.retention.180plus".to_string())
    );
    assert!(
        !plan
            .delete_candidate_event_ids
            .contains(&"evt.retention.179".to_string())
    );
}

#[test]
fn apply_retention_deletes_only_compressed_records_at_180_days() {
    let path = temp_db_path("retention-apply-delete");
    let store = open_store(&path);

    store
        .write(&sample_envelope_with_retention(
            "evt.retention.keep-recorded",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("recorded event should write");
    store
        .write(&sample_compressed_envelope(
            "evt.retention.delete-compressed",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("compressed event should write");

    let result = store
        .apply_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("apply should succeed");

    assert_eq!(result.deleted, 1);
    assert_eq!(
        result.deleted_event_ids,
        vec!["evt.retention.delete-compressed".to_string()]
    );
    assert!(
        result.skipped.iter().any(|skip| {
            skip.event_id == "evt.retention.keep-recorded"
                && skip.reason == RetentionSkipReason::Uncompacted
        }) || result
            .compacted_event_ids
            .contains(&"evt.retention.keep-recorded".to_string())
    );

    let deleted = store
        .get(&EventGetRequest::new("evt.retention.delete-compressed"))
        .expect("get should succeed");
    assert!(deleted.event.is_none());
}

#[test]
fn durable_and_audit_pinned_are_protected() {
    let path = temp_db_path("retention-protected");
    let store = open_store(&path);

    for (event_id, class) in [
        ("evt.retention.durable", RetentionClass::Durable),
        ("evt.retention.audit", RetentionClass::AuditPinned),
    ] {
        store
            .write(&sample_envelope_with_retention(
                event_id,
                BASE_TIME - ONE_EIGHTY_DAYS,
                BASE_TIME - ONE_EIGHTY_DAYS,
                class,
            ))
            .expect("protected event should write");
    }

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");

    assert_eq!(plan.protected_skipped, 2);
    for event_id in ["evt.retention.durable", "evt.retention.audit"] {
        assert!(plan.skipped.iter().any(|skip| {
            skip.event_id == event_id && skip.reason == RetentionSkipReason::Protected
        }));
    }
}

#[test]
fn tags_do_not_control_lifecycle() {
    let path = temp_db_path("retention-tags");
    let store = open_store(&path);
    let tag_set = EventTagSet::new()
        .with_tag(EventTag::new("delete").expect("valid tag"))
        .with_tag(EventTag::new("temp").expect("valid tag"))
        .with_tag(EventTag::new("audit").expect("valid tag"))
        .with_tag(EventTag::new("durable").expect("valid tag"));

    let tagged = aicore_event::EventEnvelope::builder(
        aicore_foundation::EventId::new("evt.retention.tags").expect("valid event id"),
        "memory.remembered",
        Timestamp::from_unix_millis(BASE_TIME),
        aicore_foundation::ComponentId::new("aicore-memory").expect("valid component id"),
        aicore_foundation::InstanceId::global_main(),
        "memory",
        "memory.retention.tags",
        "tagged summary",
        RetentionClass::Transient30d,
    )
    .recorded_at(Timestamp::from_unix_millis(BASE_TIME - THIRTY_DAYS))
    .status(EventStatus::Recorded)
    .tag_set(tag_set)
    .build()
    .expect("tagged event should build");

    store.write(&tagged).expect("write should succeed");

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");

    assert_eq!(
        plan.compaction_candidate_event_ids,
        vec!["evt.retention.tags".to_string()]
    );
    assert_eq!(plan.protected_skipped, 0);
}

#[test]
fn malformed_retention_class_is_fail_closed() {
    let path = temp_db_path("retention-malformed-class");
    let store = open_store(&path);

    store
        .write(&sample_envelope_with_retention(
            "evt.retention.invalid-class",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("write should succeed");

    let conn = open_sqlite(&path);
    conn.execute(
        "UPDATE events SET retention_class = 'corrupted_class' WHERE event_id = 'evt.retention.invalid-class'",
        [],
    )
    .expect("retention class corruption should succeed");
    drop(conn);

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");

    assert_eq!(plan.invalid_class_skipped, 1);
    assert!(plan.skipped.iter().any(|skip| {
        skip.event_id == "evt.retention.invalid-class"
            && skip.reason == RetentionSkipReason::InvalidClass
    }));
    assert!(plan.compaction_candidate_event_ids.is_empty());
    assert!(plan.delete_candidate_event_ids.is_empty());
}

#[test]
fn apply_retention_rolls_back_when_run_insert_fails() {
    let path = temp_db_path("retention-rollback");
    let store = open_store(&path);
    let envelope = sample_envelope_with_optionals();

    store.write(&envelope).expect("write should succeed");

    let conn = open_sqlite(&path);
    conn.execute(
        "INSERT INTO compaction_runs (run_id, started_at, finished_at, status, records_scanned, records_compressed, records_deleted, error_summary)
         VALUES ('run.fixed', '1', '1', 'completed', 0, 0, 0, NULL)",
        [],
    )
    .expect("seed compaction run should succeed");
    drop(conn);

    let error = store
        .apply_retention_with_run_id(
            Timestamp::from_unix_millis(envelope.recorded_at.unix_millis() + THIRTY_DAYS),
            "run.fixed",
        )
        .expect_err("duplicate run id should fail");
    assert!(
        error.to_string().contains("duplicate") || error.to_string().contains("UNIQUE"),
        "unexpected apply rollback error: {error}"
    );

    let response = store
        .get(&EventGetRequest::new("evt.002"))
        .expect("get should succeed");
    let event = response.event.expect("event should still exist");
    assert_eq!(event.summary, envelope.summary);
    assert_eq!(event.status, Some(EventStatus::Recorded));
    assert_eq!(event.evidence_ref, envelope.evidence_ref);
    assert_eq!(event.payload_ref, envelope.payload_ref);

    let conn = open_sqlite(&path);
    assert_eq!(count_rows(&conn, "event_refs"), 2);
    assert_eq!(count_rows(&conn, "compaction_runs"), 1);
}

#[test]
fn dry_run_and_apply_use_same_candidates() {
    let path = temp_db_path("retention-dry-run-apply-consistency");
    let store = open_store(&path);

    store
        .write(&sample_envelope_with_retention(
            "evt.retention.plan-compact",
            BASE_TIME,
            BASE_TIME - THIRTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("compact candidate should write");
    store
        .write(&sample_compressed_envelope(
            "evt.retention.plan-delete",
            BASE_TIME,
            BASE_TIME - ONE_EIGHTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("delete candidate should write");

    let now = Timestamp::from_unix_millis(BASE_TIME);
    let plan = store.plan_retention(now).expect("plan should succeed");
    let result = store.apply_retention(now).expect("apply should succeed");

    assert_eq!(
        plan.compaction_candidate_event_ids,
        result.compacted_event_ids
    );
    assert_eq!(plan.delete_candidate_event_ids, result.deleted_event_ids);
}

#[test]
fn apply_retention_rechecks_candidates_inside_transaction() {
    let path = temp_db_path("retention-apply-recheck");
    let store = open_store(&path);

    store
        .write(&sample_envelope_with_retention(
            "evt.retention.recheck",
            BASE_TIME,
            BASE_TIME - THIRTY_DAYS,
            RetentionClass::Transient30d,
        ))
        .expect("event should write");

    let now = Timestamp::from_unix_millis(BASE_TIME);
    let plan = store.plan_retention(now).expect("plan should succeed");
    assert_eq!(
        plan.compaction_candidate_event_ids,
        vec!["evt.retention.recheck".to_string()]
    );

    let conn = open_sqlite(&path);
    conn.execute(
        "UPDATE events SET retention_class = 'audit_pinned' WHERE event_id = 'evt.retention.recheck'",
        [],
    )
    .expect("retention class update should succeed");
    drop(conn);

    let result = store.apply_retention(now).expect("apply should succeed");
    assert_eq!(result.compacted, 0);
    assert_eq!(result.protected_skipped, 1);
    assert!(result.skipped.iter().any(|skip| {
        skip.event_id == "evt.retention.recheck" && skip.reason == RetentionSkipReason::Protected
    }));
}

#[test]
fn query_remains_unsupported_after_retention_operations() {
    let path = temp_db_path("retention-query-unsupported");
    let store = open_store(&path);

    let _ = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("empty plan should succeed");

    let error = store
        .query(&aicore_event::EventQueryRequest::new())
        .expect_err("query should remain unsupported");
    assert!(error.to_string().contains("not_implemented_yet"));
}

#[test]
fn retention_plan_apply_and_compaction_run_do_not_leak_raw_markers() {
    let path = temp_db_path("retention-no-raw-leak");
    let store = open_store(&path);
    let sensitive_summary =
        "summary carries raw_payload secret token api_key cookie full_prompt memory_content";
    let sensitive_evidence_ref =
        "evidence://raw_payload/secret/token/api_key/cookie/full_prompt/memory_content";
    let sensitive_payload_ref =
        "payload://raw_payload/secret/token/api_key/cookie/full_prompt/memory_content";

    let envelope = aicore_event::EventEnvelope::builder(
        aicore_foundation::EventId::new("evt.retention.safe").expect("valid event id"),
        "memory.remembered",
        Timestamp::from_unix_millis(BASE_TIME),
        aicore_foundation::ComponentId::new("aicore-memory").expect("valid component id"),
        aicore_foundation::InstanceId::global_main(),
        "memory",
        "memory.retention.safe",
        sensitive_summary,
        RetentionClass::Transient30d,
    )
    .recorded_at(Timestamp::from_unix_millis(BASE_TIME - THIRTY_DAYS))
    .status(EventStatus::Recorded)
    .evidence_ref(sensitive_evidence_ref)
    .payload_ref(sensitive_payload_ref)
    .build()
    .expect("sensitive envelope should build");

    store.write(&envelope).expect("write should succeed");

    let plan = store
        .plan_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("plan should succeed");
    let plan_debug = format!("{plan:?}");
    assert_not_contains_sensitive_markers(&plan_debug);

    let result = store
        .apply_retention(Timestamp::from_unix_millis(BASE_TIME))
        .expect("apply should succeed");
    let result_debug = format!("{result:?}");
    assert_not_contains_sensitive_markers(&result_debug);

    let conn = open_sqlite(&path);
    let error_summary: Option<String> = conn
        .query_row(
            "SELECT error_summary FROM compaction_runs LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("compaction run should load");
    if let Some(error_summary) = error_summary {
        assert_not_contains_sensitive_markers(&error_summary);
    }

    let response = store
        .get(&EventGetRequest::new("evt.retention.safe"))
        .expect("get should succeed");
    let event = response.event.expect("compressed event should exist");
    assert_eq!(event.summary, "compressed_event_record");
    assert_eq!(event.evidence_ref, None);
    assert_eq!(event.payload_ref, None);
}

#[test]
fn business_paths_do_not_call_retention_api() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../../..");

    for relative in [
        "apps/aicore/src",
        "apps/aicore-cli/src",
        "crates/kernel/aicore-kernel/src",
        "crates/memory/aicore-memory/src",
        "crates/provider/aicore-provider/src",
        "crates/agent/aicore-agent/src",
        "crates/tools/aicore-tools/src",
    ] {
        scan_directory_forbidden_calls(&workspace_root.join(relative), relative);
    }
}

fn scan_directory_forbidden_calls(path: &std::path::Path, label: &str) {
    if !path.exists() {
        return;
    }

    let entries = std::fs::read_dir(path).expect("directory should read");
    for entry in entries {
        let entry = entry.expect("directory entry should read");
        let entry_path = entry.path();
        if entry_path.is_dir() {
            scan_directory_forbidden_calls(&entry_path, label);
            continue;
        }
        if entry_path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let content = std::fs::read_to_string(&entry_path).expect("source file should read");
        for forbidden in [
            "plan_retention(",
            "apply_retention(",
            "RetentionPlan",
            "RetentionApplyResult",
        ] {
            assert!(
                !content.contains(forbidden),
                "{label} should not reference retention API via {}",
                entry_path.display()
            );
        }
    }
}

fn assert_not_contains_sensitive_markers(text: &str) {
    for marker in [
        "raw_payload",
        "secret",
        "token",
        "api_key",
        "cookie",
        "full_prompt",
        "memory_content",
    ] {
        assert!(
            !text.contains(marker),
            "sensitive marker `{marker}` leaked into `{text}`"
        );
    }
}
