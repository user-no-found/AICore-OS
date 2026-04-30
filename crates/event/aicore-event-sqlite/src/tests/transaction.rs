use rusqlite::Connection;

use aicore_event::{EventTag, EventWriter};
use aicore_foundation::InstanceId;

use crate::tests::{open_store, sample_envelope, temp_db_path};

#[test]
fn duplicate_event_id_fails_without_partial_rows() {
    let path = temp_db_path("tx-duplicate-event-id");
    let store = open_store(&path);
    let envelope = sample_envelope();

    store.write(&envelope).expect("first write should succeed");
    let error = store
        .write(&envelope)
        .expect_err("duplicate event id should fail");

    assert!(
        error.to_string().contains("duplicate") || error.to_string().contains("UNIQUE"),
        "unexpected duplicate error: {error}"
    );

    let conn = Connection::open(path).expect("sqlite db should open");
    let event_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM events WHERE event_id = 'evt.001'",
            [],
            |row| row.get(0),
        )
        .expect("event count should load");
    let tag_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_tags WHERE event_id = 'evt.001'",
            [],
            |row| row.get(0),
        )
        .expect("tag count should load");
    let confirmed_tag_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_confirmed_tags WHERE event_id = 'evt.001'",
            [],
            |row| row.get(0),
        )
        .expect("confirmed tag count should load");
    let ref_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event_refs WHERE event_id = 'evt.001'",
            [],
            |row| row.get(0),
        )
        .expect("ref count should load");

    assert_eq!(event_count, 1);
    assert_eq!(tag_count, 0);
    assert_eq!(confirmed_tag_count, 0);
    assert_eq!(ref_count, 0);
}

#[test]
fn failed_write_rolls_back_tags_confirmed_tags_and_refs() {
    let path = temp_db_path("tx-rollback");
    let store = open_store(&path);
    let mut envelope = crate::tests::sample_envelope_with_optionals();
    envelope.event_id = aicore_foundation::EventId::new("evt.003").expect("valid event id");
    envelope
        .tag_set
        .tags
        .push(EventTag::new("candidate:durable").expect("valid duplicate tag"));

    let error = store
        .write(&envelope)
        .expect_err("duplicate tag insert should fail");
    assert!(
        error.to_string().contains("duplicate") || error.to_string().contains("UNIQUE"),
        "unexpected rollback error: {error}"
    );

    let conn = Connection::open(path).expect("sqlite db should open");
    for table in ["events", "event_tags", "event_confirmed_tags", "event_refs"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count should load");
        assert_eq!(count, 0, "{table} should have rolled back");
    }
}

#[test]
fn write_rejects_cross_instance_event_without_partial_rows() {
    let path = temp_db_path("tx-cross-instance");
    let store = open_store(&path);
    let mut envelope = sample_envelope();
    envelope.source_instance =
        InstanceId::new("workspace-other").expect("valid workspace instance");

    let error = store
        .write(&envelope)
        .expect_err("cross-instance event should fail");

    let message = error.to_string();
    assert!(
        message.contains("instance") || message.contains("source_instance"),
        "unexpected cross-instance error: {message}"
    );

    let conn = Connection::open(path).expect("sqlite db should open");
    for table in ["events", "event_tags", "event_confirmed_tags", "event_refs"] {
        let count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("count should load");
        assert_eq!(count, 0, "{table} should have no residual rows");
    }
}
