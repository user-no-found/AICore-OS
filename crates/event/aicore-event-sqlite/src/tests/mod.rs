mod retention;
mod schema;
mod store;
mod transaction;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use aicore_event::{
    EventEnvelope, EventGetRequest, EventReader, EventStatus, EventTag, EventTagSet, EventWriter,
    RetentionClass,
};
use aicore_foundation::{ComponentId, EventId, InstanceId, Timestamp};
use rusqlite::Connection;

use crate::SqliteEventStore;

fn temp_db_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("aicore-event-sqlite-{name}-{nanos}.sqlite"))
}

fn sample_envelope() -> EventEnvelope {
    EventEnvelope::builder(
        EventId::new("evt.001").expect("valid event id"),
        "memory.remembered",
        Timestamp::from_unix_millis(1_746_000_000_000),
        ComponentId::new("aicore-memory").expect("valid component id"),
        InstanceId::global_main(),
        "memory",
        "memory.001",
        "remembered memory summary",
        RetentionClass::Transient30d,
    )
    .build()
    .expect("sample envelope should build")
}

fn sample_envelope_with_optionals() -> EventEnvelope {
    let tag_set = EventTagSet::new()
        .with_tag(EventTag::new("candidate:durable").expect("valid tag"))
        .with_confirmed(EventTag::new("durable").expect("valid confirmed tag"));

    EventEnvelope::builder(
        EventId::new("evt.002").expect("valid event id"),
        "error.recorded",
        Timestamp::from_unix_millis(1_746_000_100_000),
        ComponentId::new("aicore-cli").expect("valid component id"),
        InstanceId::global_main(),
        "error",
        "error.001",
        "recorded error summary",
        RetentionClass::NeedsReview,
    )
    .schema_version("1.0")
    .recorded_at(Timestamp::from_unix_millis(1_746_000_100_500))
    .correlation_id("corr.001")
    .causation_id("cause.001")
    .invocation_id(aicore_foundation::InvocationId::new("invoke.001").expect("valid invocation"))
    .tag_set(tag_set)
    .evidence_ref("evidence://evt.002")
    .payload_ref("payload://evt.002")
    .redaction_level("summary")
    .visibility(aicore_event::EventVisibility::Instance)
    .status(aicore_event::EventStatus::Recorded)
    .replay_policy(aicore_event::ReplayPolicy::HistoryOnly)
    .build()
    .expect("optional envelope should build")
}

fn sample_envelope_with_retention(
    event_id: &str,
    occurred_at_millis: u128,
    recorded_at_millis: u128,
    retention_class: RetentionClass,
) -> EventEnvelope {
    EventEnvelope::builder(
        EventId::new(event_id).expect("valid event id"),
        "memory.remembered",
        Timestamp::from_unix_millis(occurred_at_millis),
        ComponentId::new("aicore-memory").expect("valid component id"),
        InstanceId::global_main(),
        "memory",
        format!("memory.{event_id}"),
        format!("summary for {event_id}"),
        retention_class,
    )
    .recorded_at(Timestamp::from_unix_millis(recorded_at_millis))
    .status(EventStatus::Recorded)
    .build()
    .expect("retention envelope should build")
}

fn sample_compressed_envelope(
    event_id: &str,
    occurred_at_millis: u128,
    recorded_at_millis: u128,
    retention_class: RetentionClass,
) -> EventEnvelope {
    EventEnvelope::builder(
        EventId::new(event_id).expect("valid event id"),
        "memory.remembered",
        Timestamp::from_unix_millis(occurred_at_millis),
        ComponentId::new("aicore-memory").expect("valid component id"),
        InstanceId::global_main(),
        "memory",
        format!("memory.{event_id}"),
        "compressed_event_record",
        retention_class,
    )
    .recorded_at(Timestamp::from_unix_millis(recorded_at_millis))
    .status(EventStatus::Compressed)
    .build()
    .expect("compressed retention envelope should build")
}

fn open_store(path: &std::path::Path) -> SqliteEventStore {
    SqliteEventStore::open(path, &InstanceId::global_main()).expect("store should open")
}

fn write_sample_event(store: &SqliteEventStore) {
    store
        .write(&sample_envelope())
        .expect("write should succeed");
}

fn get_sample_event(store: &SqliteEventStore) -> aicore_event::EventGetResponse {
    store
        .get(&EventGetRequest::new("evt.001"))
        .expect("get should succeed")
}

fn open_sqlite(path: &std::path::Path) -> Connection {
    Connection::open(path).expect("sqlite db should open")
}

fn count_rows(conn: &Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .expect("count should load")
}
