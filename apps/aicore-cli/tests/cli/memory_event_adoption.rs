use std::fs;
use std::process::Command;

use aicore_event::{EventGetRequest, EventReader};
use aicore_event_sqlite::SqliteEventStore;
use aicore_foundation::{InstanceId, ensure_instance_layout, resolve_instance_for_cwd};
use aicore_memory::{MemoryKernel, MemoryPaths};

use super::support::{
    MemoryTrigger, assert_has_json_event, assert_json_lines, result_event, run_cli_with_env,
    run_component_with_payload, runtime_home, seed_open_proposal, seed_rule_based_proposal,
    temp_root,
};

#[test]
fn memory_remember_default_path_unchanged() {
    let home = runtime_home("memory-remember-unchanged");
    let output = run_cli_with_env(
        &["memory", "remember", "remember unchanged test"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                home.join("config").to_str().expect("config root utf-8"),
            ),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核写入调用"));
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("in-process fallback：false"));
}

#[test]
fn cli_kernel_invoke_write_memory_remember_unknown_recommended_action_json() {
    let home = runtime_home("kernel-write-remember-unknown-json");
    let output = run_cli_with_env(
        &[
            "kernel",
            "invoke-write",
            "memory.remember",
            "trigger_unknown",
        ],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                home.join("config").to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result = result_event(&events);
    assert_eq!(result["payload"]["operation"], "memory.remember");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_outcome"],
        "unknown"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["recommended_action"],
        "query_memory_fact_source_before_retry"
    );
    assert!(
        result["payload"]["result"]["fields"]["recommended_action_message"]
            .as_str()
            .unwrap()
            .contains("memory status")
    );
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn cli_kernel_invoke_write_memory_remember_unknown_recommended_action_human() {
    let home = runtime_home("kernel-write-remember-unknown-human");
    let output = run_cli_with_env(
        &[
            "kernel",
            "invoke-write",
            "memory.remember",
            "trigger_unknown",
        ],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                home.join("config").to_str().expect("config root utf-8"),
            ),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核写入调用"));
    assert!(stdout.contains("write_state"));
    assert!(stdout.contains("不确定"));
    assert!(stdout.contains("memory status"));
    assert!(stdout.contains("不要直接重复提交"));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_remember_kernel_native_json_outputs_structured_fields() {
    let home = runtime_home("memory-remember-kernel-native-json");
    let output = run_cli_with_env(
        &["memory", "remember", "kernel native remember test"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                home.join("config").to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "kernel.invocation.result");
    let result = result_event(&events);
    assert_eq!(result["payload"]["operation"], "memory.remember");
    assert_eq!(result["payload"]["status"], "completed");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["write_outcome"],
        "applied"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["kernel_invocation_path"],
        "binary"
    );
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn direct_memory_write_commands_remain_compatible() {
    let home = runtime_home("direct-memory-write-compatible");
    let root = home.join("config");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：direct accept",
    );

    let remember = run_cli_with_env(
        &["memory", "remember", "direct remember"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
        ],
    );
    assert!(remember.status.success());

    let accept = run_cli_with_env(
        &["memory", "accept", &proposal_id],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
        ],
    );
    assert!(accept.status.success());

    let reject_proposal = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：direct reject",
    );
    let reject = run_cli_with_env(
        &["memory", "reject", &reject_proposal],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
        ],
    );
    assert!(reject.status.success());
}

#[test]
fn memory_remember_component_success_writes_event_ledger() {
    let root = temp_root("m56-memory-remember-event");
    let output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": "m56 remember event" }),
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");

    assert_eq!(value["fields"]["event_recorded"], "true");
    assert_eq!(value["fields"]["event_write_status"], "recorded");
    let memory_id = value["fields"]["memory_id"]
        .as_str()
        .expect("memory_id should exist");
    let event_id = value["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");

    let store = open_event_store(&event_db_path_for_root(&root));
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.event_type, "memory.remembered");
    assert_eq!(event.subject_type, "memory_record");
    assert_eq!(event.subject_id, memory_id);
    assert_eq!(event.summary, "memory record created");
    assert_eq!(event.retention_class.as_str(), "transient_30d");
    assert_eq!(event.source_instance.as_str(), "global-main");
    assert_eq!(event.source_component.as_str(), "aicore-memory");
}

#[test]
fn memory_accept_component_success_writes_event_ledger() {
    let root = temp_root("m56-memory-accept-event");
    let proposal_id = seed_open_proposal(
        &root,
        aicore_memory::MemoryType::Core,
        "m56 accept proposal",
    );

    let output = run_component_with_payload(
        "__component-memory-accept-stdio",
        "memory.accept",
        serde_json::json!({ "proposal_id": proposal_id }),
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["fields"]["event_recorded"], "true");
    assert_eq!(value["fields"]["event_write_status"], "recorded");

    let event_id = value["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");
    let store = open_event_store(&event_db_path_for_root(&root));
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.event_type, "memory.proposal.accepted");
    assert_eq!(event.subject_type, "memory_proposal");
    assert_eq!(event.subject_id, proposal_id);
    assert_eq!(event.summary, "memory proposal accepted");
}

#[test]
fn memory_reject_component_success_writes_event_ledger() {
    let root = temp_root("m56-memory-reject-event");
    let proposal_id = seed_open_proposal(
        &root,
        aicore_memory::MemoryType::Core,
        "m56 reject proposal",
    );

    let output = run_component_with_payload(
        "__component-memory-reject-stdio",
        "memory.reject",
        serde_json::json!({ "proposal_id": proposal_id }),
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["fields"]["event_recorded"], "true");
    assert_eq!(value["fields"]["event_write_status"], "recorded");

    let event_id = value["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");
    let store = open_event_store(&event_db_path_for_root(&root));
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.event_type, "memory.proposal.rejected");
    assert_eq!(event.subject_type, "memory_proposal");
    assert_eq!(event.subject_id, proposal_id);
    assert_eq!(event.summary, "memory proposal rejected");
}

#[test]
fn memory_event_adoption_has_no_raw_leak() {
    let root = temp_root("m56-no-raw-leak");
    let sensitive = "raw_payload secret token api_key cookie full_prompt memory_content";
    let remember_output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": sensitive }),
        &root,
    );
    assert!(remember_output.status.success());
    let remember_stdout = String::from_utf8(remember_output.stdout).expect("stdout utf-8");
    assert_no_sensitive_markers(&remember_stdout);

    let proposal_id = seed_open_proposal(&root, aicore_memory::MemoryType::Core, sensitive);
    let accept_output = run_component_with_payload(
        "__component-memory-accept-stdio",
        "memory.accept",
        serde_json::json!({ "proposal_id": proposal_id }),
        &root,
    );
    assert!(accept_output.status.success());
    let accept_stdout = String::from_utf8(accept_output.stdout).expect("stdout utf-8");
    assert_no_sensitive_markers(&accept_stdout);

    let value: serde_json::Value =
        serde_json::from_str(accept_stdout.trim()).expect("component output should be json");
    let event_id = value["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");
    let store = open_event_store(&event_db_path_for_root(&root));
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.evidence_ref, None);
    assert_eq!(event.payload_ref, None);
    assert_no_sensitive_markers(&event.summary);
    assert_no_sensitive_markers(&serde_json::to_string(&event).expect("event should serialize"));
    for tag in &event.tag_set.tags {
        assert_no_sensitive_markers(tag.as_str());
    }
}

#[test]
fn memory_event_write_failure_does_not_rollback_main_business() {
    let root = temp_root("m56-event-write-failure");
    seed_mismatched_event_store(&root);

    let output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": "event failure remember proof" }),
        &root,
    );

    assert!(
        output.status.success(),
        "main business should still succeed"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");

    assert_eq!(value["fields"]["write_applied"], "true");
    assert_eq!(value["fields"]["event_recorded"], "false");
    assert_eq!(value["fields"]["event_write_status"], "failed");
    assert!(value["fields"]["event_error_code"].is_string());
    assert!(value["fields"].get("event_id").is_none());

    let search = super::support::run_cli_with_config_root(
        &[
            "memory",
            "search",
            "event failure remember proof",
            "--local",
        ],
        &root,
    );
    assert!(search.status.success());
    let search_stdout = String::from_utf8(search.stdout).expect("stdout should be utf-8");
    assert!(search_stdout.contains("event failure remember proof"));
}

#[test]
fn memory_business_failures_do_not_write_success_events() {
    let root = temp_root("m56-business-failure-no-event");

    let remember = super::support::run_cli_with_config_root_and_env(
        &["memory", "remember", "", "--local"],
        &root.join("config"),
        &[("AICORE_TERMINAL", "json")],
    );
    assert!(!remember.status.success());

    let accept_missing = run_component_with_payload(
        "__component-memory-accept-stdio",
        "memory.accept",
        serde_json::json!({}),
        &root,
    );
    assert!(accept_missing.status.success());
    let accept_value: serde_json::Value =
        serde_json::from_str(String::from_utf8(accept_missing.stdout).unwrap().trim())
            .expect("component output should be json");
    assert_eq!(accept_value["status"], "failed");
    assert_eq!(accept_value["fields"]["event_recorded"], "false");
    assert_eq!(accept_value["fields"]["event_write_status"], "skipped");

    let reject_unknown = run_component_with_payload(
        "__component-memory-reject-stdio",
        "memory.reject",
        serde_json::json!({ "proposal_id": "prop_missing" }),
        &root,
    );
    assert!(reject_unknown.status.success());
    let reject_value: serde_json::Value =
        serde_json::from_str(String::from_utf8(reject_unknown.stdout).unwrap().trim())
            .expect("component output should be json");
    assert_eq!(reject_value["status"], "failed");
    assert_eq!(reject_value["fields"]["event_recorded"], "false");
    assert_eq!(reject_value["fields"]["event_write_status"], "skipped");

    let event_db = event_db_path_for_root(&root.join("config"));
    if event_db.exists() {
        let store = open_event_store(&event_db);
        assert!(
            store
                .get(&EventGetRequest::new("evt.memory.remembered.unknown"))
                .unwrap()
                .event
                .is_none(),
            "no success event should be materialized for failed business writes"
        );
    }
}

#[test]
fn memory_event_query_remains_unsupported() {
    let root = temp_root("m56-query-unsupported");
    let output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": "query unsupported check" }),
        &root,
    );
    assert!(output.status.success());

    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8(output.stdout).unwrap().trim())
            .expect("component output should be json");
    let event_id = value["fields"]["event_id"].as_str().unwrap();
    let store = open_event_store(&event_db_path_for_root(&root));
    let error = store
        .query(&aicore_event::EventQueryRequest::new())
        .expect_err("query should remain unsupported");
    assert!(error.to_string().contains("not_implemented_yet"));
    assert!(
        store
            .get(&EventGetRequest::new(event_id))
            .unwrap()
            .event
            .is_some(),
        "get should still work"
    );
}

#[test]
fn workspace_memory_remember_uses_workspace_scope_without_config_override() {
    let home = runtime_home("m6p1-workspace-memory-scope-home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).expect("workspace should create");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "remember", "workspace scoped memory", "--local"])
        .current_dir(&workspace)
        .env("HOME", &home)
        .env("AICORE_TERMINAL", "json")
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    let workspace_memory_root = workspace.join(".aicore").join("memory");
    let kernel =
        MemoryKernel::open(MemoryPaths::new(&workspace_memory_root)).expect("kernel should open");
    let record = kernel
        .records()
        .into_iter()
        .find(|record| record.content == "workspace scoped memory")
        .expect("workspace memory should exist");

    match &record.scope {
        aicore_memory::MemoryScope::Workspace {
            instance_id,
            workspace_root,
        } => {
            assert_ne!(instance_id, "global-main");
            assert_eq!(workspace_root, &workspace.display().to_string());
        }
        other => panic!("expected workspace scope, got {other:?}"),
    }
}

#[test]
fn workspace_memory_event_source_instance_uses_workspace_instance_without_config_override() {
    let home = runtime_home("m6p1-workspace-event-source-home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).expect("workspace should create");
    let binding = resolve_instance_for_cwd(&workspace, &home).expect("workspace should resolve");
    ensure_instance_layout(&binding).expect("layout should create");
    let expected_instance = binding.instance_id.as_str().to_string();

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "remember", "workspace event source", "--local"])
        .current_dir(&workspace)
        .env("HOME", &home)
        .env("AICORE_TERMINAL", "json")
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let result: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("local direct output should be json");
    assert_eq!(result["fields"]["event_recorded"], "true");
    let event_id = result["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");
    let store = SqliteEventStore::open(
        workspace
            .join(".aicore")
            .join("events")
            .join("events.sqlite"),
        &binding.instance_id,
    )
    .expect("workspace event store should open");
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.source_instance.as_str(), expected_instance);
    assert_ne!(event.source_instance.as_str(), "global-main");
}

#[test]
fn legacy_config_root_event_source_instance_remains_global_main() {
    let root = temp_root("m6p1-legacy-event-source");
    let output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": "legacy event source" }),
        &root,
    );

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8(output.stdout).unwrap().trim())
            .expect("component output should be json");
    let event_id = value["fields"]["event_id"]
        .as_str()
        .expect("event_id should exist");
    let store = open_event_store(&event_db_path_for_root(&root));
    let event = store
        .get(&EventGetRequest::new(event_id))
        .expect("event get should succeed")
        .event
        .expect("event should exist");

    assert_eq!(event.source_instance.as_str(), "global-main");
}

#[test]
fn legacy_config_root_event_write_failure_still_uses_global_main_guard() {
    let root = temp_root("m6p1-legacy-event-guard");
    seed_mismatched_event_store(&root);

    let output = run_component_with_payload(
        "__component-memory-remember-stdio",
        "memory.remember",
        serde_json::json!({ "content": "legacy mismatch still fails" }),
        &root,
    );

    assert!(output.status.success());
    let value: serde_json::Value =
        serde_json::from_str(String::from_utf8(output.stdout).unwrap().trim())
            .expect("component output should be json");

    assert_eq!(value["fields"]["write_applied"], "true");
    assert_eq!(value["fields"]["event_recorded"], "false");
    assert_eq!(value["fields"]["event_write_status"], "failed");
    assert!(value["fields"].get("event_id").is_none());
}

fn event_db_path_for_root(root: &std::path::Path) -> std::path::PathBuf {
    root.join("instances")
        .join("global-main")
        .join("events")
        .join("events.sqlite")
}

fn open_event_store(path: &std::path::Path) -> SqliteEventStore {
    SqliteEventStore::open(path, &InstanceId::global_main()).expect("event store should open")
}

fn seed_mismatched_event_store(root: &std::path::Path) {
    let path = event_db_path_for_root(root);
    let instance = InstanceId::new("workspace-other").expect("valid instance id");
    let store = SqliteEventStore::open(&path, &instance).expect("mismatched store should open");
    drop(store);
}

fn assert_no_sensitive_markers(text: &str) {
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
