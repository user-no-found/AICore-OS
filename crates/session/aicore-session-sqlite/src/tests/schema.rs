use rusqlite::Connection;

use aicore_foundation::InstanceId;

use crate::SqliteSessionStore;
use crate::tests::{open_store, temp_store_path};

const EXPECTED_TABLES: &[&str] = &[
    "approval_responses",
    "approvals",
    "control_events",
    "instance_runtime_state",
    "ledger_meta",
    "ledger_writes",
    "messages",
    "pending_inputs",
    "sessions",
    "turns",
];

#[test]
fn open_initializes_all_ten_tables() {
    let path = temp_store_path("schema-all-tables");
    let _store = open_store(path.db_path());
    let conn = Connection::open(path.db_path()).expect("sqlite db should open");

    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
        )
        .expect("statement should prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query should execute")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("table names should collect");

    assert_eq!(tables, EXPECTED_TABLES);
}

#[test]
fn ledger_meta_is_initialized_with_expected_values() {
    let path = temp_store_path("schema-meta");
    let _store = open_store(path.db_path());
    let conn = Connection::open(path.db_path()).expect("sqlite db should open");

    let (schema_version, store_kind, instance_id): (i64, String, String) = conn
        .query_row(
            "SELECT schema_version, store_kind, instance_id FROM ledger_meta LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("meta row should exist");

    assert_eq!(schema_version, 2);
    assert_eq!(store_kind, "sqlite_session_ledger");
    assert_eq!(instance_id, InstanceId::global_main().as_str());
}

#[test]
fn future_schema_version_fails_structurally() {
    let path = temp_store_path("schema-future-version");
    {
        let store = open_store(path.db_path());
        drop(store);
    }

    let conn = Connection::open(path.db_path()).expect("sqlite db should open");
    conn.execute("UPDATE ledger_meta SET schema_version = 3", [])
        .expect("schema version update should succeed");
    drop(conn);

    let error = match SqliteSessionStore::open(path.db_path(), &InstanceId::global_main()) {
        Ok(_) => panic!("future schema version should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("schema version"));
}

#[test]
fn wrong_instance_id_fails_structurally() {
    let path = temp_store_path("schema-wrong-instance");
    {
        let store = open_store(path.db_path());
        drop(store);
    }

    let conn = Connection::open(path.db_path()).expect("sqlite db should open");
    conn.execute("UPDATE ledger_meta SET instance_id = 'workspace-other'", [])
        .expect("instance id update should succeed");
    drop(conn);

    let error = match SqliteSessionStore::open(path.db_path(), &InstanceId::global_main()) {
        Ok(_) => panic!("wrong instance id should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("instance id"));
}

#[test]
fn wrong_store_kind_fails_structurally() {
    let path = temp_store_path("schema-wrong-kind");
    {
        let store = open_store(path.db_path());
        drop(store);
    }

    let conn = Connection::open(path.db_path()).expect("sqlite db should open");
    conn.execute(
        "UPDATE ledger_meta SET store_kind = 'event_ledger_sqlite'",
        [],
    )
    .expect("store kind update should succeed");
    drop(conn);

    let error = match SqliteSessionStore::open(path.db_path(), &InstanceId::global_main()) {
        Ok(_) => panic!("wrong store kind should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("ledger kind"));
}

#[test]
fn schema_does_not_contain_forbidden_raw_or_secret_columns() {
    let schema = include_str!("../schema.rs").to_lowercase();
    for forbidden in super::FORBIDDEN_FIELDS {
        assert!(
            !schema.contains(forbidden),
            "forbidden schema token leaked: {forbidden}"
        );
    }
}

#[test]
fn unique_session_turn_seq_is_enforced() {
    let path = temp_store_path("schema-uniq-seq");
    let _store = open_store(path.db_path());
    let conn = Connection::open(path.db_path()).expect("sqlite db should open");

    // Insert session first
    conn.execute(
        "INSERT INTO sessions (session_id, title, status, created_at, updated_at)
         VALUES ('sess-1', 'Test', 'active', 1, 1)",
        [],
    )
    .expect("session insert should succeed");

    // Insert first turn
    conn.execute(
        "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
         VALUES ('turn-1', 'sess-1', 1, 'active', 1)",
        [],
    )
    .expect("first turn insert should succeed");

    // Duplicate turn_seq should fail
    let result = conn.execute(
        "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
         VALUES ('turn-2', 'sess-1', 1, 'active', 1)",
        [],
    );
    assert!(
        result.is_err(),
        "duplicate turn_seq should violate UNIQUE(session_id, turn_seq)"
    );
}

#[test]
fn foreign_key_session_cascade_delete_is_enforced() {
    let path = temp_store_path("schema-fk-cascade");
    let _store = open_store(path.db_path());
    let conn = Connection::open(path.db_path()).expect("sqlite db should open");

    conn.execute(
        "INSERT INTO sessions (session_id, title, status, created_at, updated_at)
         VALUES ('sess-1', 'Test', 'active', 1, 1)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
         VALUES ('turn-1', 'sess-1', 1, 'active', 1)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO messages (message_id, session_id, turn_id, kind, content, created_at)
         VALUES ('msg-1', 'sess-1', 'turn-1', 'user', 'hello', 1)",
        [],
    )
    .unwrap();

    // Delete session should cascade to turns and messages
    conn.execute("DELETE FROM sessions WHERE session_id = 'sess-1'", [])
        .expect("cascade delete should succeed");

    let turn_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM turns WHERE session_id = 'sess-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(turn_count, 0, "cascade delete should remove turns");

    let msg_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE session_id = 'sess-1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(msg_count, 0, "cascade delete should remove messages");
}

#[test]
fn status_enum_check_is_enforced() {
    let path = temp_store_path("schema-check-enum");
    let _store = open_store(path.db_path());
    let conn = Connection::open(path.db_path()).expect("sqlite db should open");

    // Invalid status for sessions
    let result = conn.execute(
        "INSERT INTO sessions (session_id, title, status, created_at, updated_at)
         VALUES ('sess-1', 'Test', 'invalid_status', 1, 1)",
        [],
    );
    assert!(
        result.is_err(),
        "invalid session status should violate CHECK"
    );

    // Invalid status for turns
    conn.execute(
        "INSERT INTO sessions (session_id, title, status, created_at, updated_at)
         VALUES ('sess-1', 'Test', 'active', 1, 1)",
        [],
    )
    .unwrap();
    let result = conn.execute(
        "INSERT INTO turns (turn_id, session_id, turn_seq, status, started_at)
         VALUES ('turn-1', 'sess-1', 1, 'invalid', 1)",
        [],
    );
    assert!(result.is_err(), "invalid turn status should violate CHECK");
}
