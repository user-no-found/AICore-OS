use std::fs;

use aicore_event::{EventGetRequest, EventQueryRequest, EventReader, EventWriter};

use crate::tests::{
    get_sample_event, open_store, sample_envelope, sample_envelope_with_optionals, temp_db_path,
    write_sample_event,
};

#[test]
fn write_event_succeeds_and_get_returns_equivalent_envelope() {
    let path = temp_db_path("store-write-get");
    let store = open_store(&path);

    write_sample_event(&store);
    let response = get_sample_event(&store);

    assert_eq!(response.event, Some(sample_envelope()));
}

#[test]
fn optional_fields_roundtrip() {
    let path = temp_db_path("store-optional-fields");
    let store = open_store(&path);
    let envelope = sample_envelope_with_optionals();

    store.write(&envelope).expect("write should succeed");

    let response = store
        .get(&EventGetRequest::new("evt.002"))
        .expect("get should succeed");

    assert_eq!(response.event, Some(envelope));
}

#[test]
fn get_missing_event_returns_none() {
    let path = temp_db_path("store-missing-event");
    let store = open_store(&path);

    let response = store
        .get(&EventGetRequest::new("evt.missing"))
        .expect("get should succeed");

    assert!(response.event.is_none());
}

#[test]
fn query_returns_structured_not_implemented_error() {
    let path = temp_db_path("store-query-unsupported");
    let store = open_store(&path);

    let error = store
        .query(&EventQueryRequest::new())
        .expect_err("query should be unsupported");

    let message = error.to_string();
    assert!(message.contains("unsupported"));
    assert!(message.contains("not_implemented_yet"));
}

#[test]
fn aicore_event_does_not_depend_on_rusqlite() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let cargo_toml = fs::read_to_string(manifest_dir.join("../aicore-event/Cargo.toml"))
        .expect("aicore-event Cargo.toml should read");

    assert!(!cargo_toml.contains("rusqlite"));
}

#[test]
fn business_crates_do_not_depend_on_aicore_event_sqlite() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.join("../../..");

    for relative in [
        "apps/aicore/Cargo.toml",
        "apps/aicore-cli/Cargo.toml",
        "crates/kernel/aicore-kernel/Cargo.toml",
        "crates/memory/aicore-memory/Cargo.toml",
        "crates/provider/aicore-provider/Cargo.toml",
        "crates/agent/aicore-agent/Cargo.toml",
        "crates/tools/aicore-tools/Cargo.toml",
    ] {
        let cargo_toml = fs::read_to_string(workspace_root.join(relative))
            .expect("business crate Cargo.toml should read");
        assert!(
            !cargo_toml.contains("aicore-event-sqlite"),
            "{relative} should not depend on aicore-event-sqlite",
        );
    }
}
