use rusqlite::Connection;

use aicore_foundation::InstanceId;

use crate::SqliteEventStore;
use crate::schema;
use crate::tests::{open_store, temp_db_path};

#[test]
fn open_initializes_all_required_tables() {
    let path = temp_db_path("schema-all-tables");
    let _store = open_store(&path);
    let conn = Connection::open(path).expect("sqlite db should open");

    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .expect("statement should prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query should execute")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("table names should collect");

    assert_eq!(
        tables,
        vec![
            "compaction_runs",
            "error_index",
            "event_confirmed_tags",
            "event_refs",
            "event_store_meta",
            "event_tags",
            "events",
            "fix_index",
        ]
    );
}

#[test]
fn event_store_meta_is_initialized_with_expected_values() {
    let path = temp_db_path("schema-meta");
    let _store = open_store(&path);
    let conn = Connection::open(path).expect("sqlite db should open");

    let (schema_version, store_kind, instance_id): (i64, String, String) = conn
        .query_row(
            "SELECT schema_version, store_kind, instance_id FROM event_store_meta LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("meta row should exist");

    assert_eq!(schema_version, 1);
    assert_eq!(store_kind, "sqlite_event_ledger");
    assert_eq!(instance_id, InstanceId::global_main().as_str());
}

#[test]
fn future_schema_version_fails_structurally() {
    let path = temp_db_path("schema-future-version");
    {
        let store = open_store(&path);
        drop(store);
    }

    let conn = Connection::open(&path).expect("sqlite db should open");
    conn.execute("UPDATE event_store_meta SET schema_version = 2", [])
        .expect("schema version update should succeed");
    drop(conn);

    let error = match SqliteEventStore::open(&path, &InstanceId::global_main()) {
        Ok(_) => panic!("future schema version should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("schema version"));
}

#[test]
fn wrong_instance_id_fails_structurally() {
    let path = temp_db_path("schema-wrong-instance");
    {
        let store = open_store(&path);
        drop(store);
    }

    let conn = Connection::open(&path).expect("sqlite db should open");
    conn.execute(
        "UPDATE event_store_meta SET instance_id = 'workspace-other'",
        [],
    )
    .expect("instance id update should succeed");
    drop(conn);

    let error = match SqliteEventStore::open(&path, &InstanceId::global_main()) {
        Ok(_) => panic!("wrong instance id should fail"),
        Err(error) => error,
    };

    assert!(error.to_string().contains("instance id"));
}

#[test]
fn raw_forbidden_fields_are_absent_from_schema() {
    assert!(!schema::schema_contains_forbidden_fields(
        schema::schema_sql()
    ));

    let path = temp_db_path("schema-forbidden-fields");
    let _store = open_store(&path);
    let conn = Connection::open(path).expect("sqlite db should open");

    let schema_sql: String = conn
        .query_row(
            "SELECT group_concat(sql, '\n') FROM sqlite_master WHERE type IN ('table', 'index') AND sql IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .expect("schema sql should load");

    for forbidden in [
        "raw_stdout",
        "raw_stderr",
        "raw_payload",
        "raw_memory_content",
        "raw_provider_request",
        "raw_provider_response",
        "raw_tool_input",
        "raw_tool_output",
        "secret",
        "secret_ref",
        "token",
        "api_key",
        "cookie",
        "full_prompt",
        "full_log",
        "full_backtrace",
        "full_patch",
    ] {
        assert!(
            !schema_sql.contains(forbidden),
            "forbidden field `{forbidden}` leaked into schema"
        );
    }
}
