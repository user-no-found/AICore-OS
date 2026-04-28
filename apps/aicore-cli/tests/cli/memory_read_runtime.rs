use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    MemoryPermanence, MemoryType, assert_has_json_event, assert_json_lines,
    run_cli_with_config_root, run_cli_with_env, seed_foundation_runtime_binary,
    seed_global_runtime_metadata, seed_kernel_runtime_binary_fixture, seed_memory_read_manifests,
    seed_memory_record, temp_root,
};

#[test]
fn memory_read_components_output_single_jsonl_result() {
    let root = temp_root("component-memory-read-jsonl");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "component memory read content",
    );

    for (command, operation, field) in [
        (
            "__component-memory-status-stdio",
            "memory.status",
            "record_count",
        ),
        (
            "__component-memory-search-stdio",
            "memory.search",
            "result_count",
        ),
        (
            "__component-memory-proposals-stdio",
            "memory.proposals",
            "proposal_count",
        ),
        ("__component-memory-audit-stdio", "memory.audit", "ok"),
        ("__component-memory-wiki-stdio", "memory.wiki", "pages"),
        (
            "__component-memory-wiki-page-stdio",
            "memory.wiki_page",
            "markdown",
        ),
    ] {
        let output = run_component(command, operation, &root);

        assert!(output.status.success(), "{operation} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert_eq!(stdout.lines().count(), 1);
        assert!(!stdout.contains('╭'));
        assert!(!stdout.contains("\u{1b}["));
        assert!(!stdout.contains("secret_ref"));

        let value: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("component output should be json");
        assert_eq!(value["result_kind"], operation);
        assert_eq!(value["fields"]["operation"], operation);
        assert!(value["fields"][field].is_string());
        assert_eq!(value["fields"]["kernel_invocation_path"], "binary");
    }
}

#[test]
fn cli_kernel_invoke_readonly_memory_read_json_outputs_structured_fields() {
    for (operation, args, component_id, field) in [
        (
            "memory.status",
            Vec::<&str>::new(),
            "aicore-memory-status",
            "record_count",
        ),
        (
            "memory.search",
            vec!["test"],
            "aicore-memory-search",
            "result_count",
        ),
        (
            "memory.proposals",
            Vec::<&str>::new(),
            "aicore-memory-proposals",
            "proposal_count",
        ),
        (
            "memory.audit",
            Vec::<&str>::new(),
            "aicore-memory-audit",
            "ok",
        ),
        (
            "memory.wiki",
            Vec::<&str>::new(),
            "aicore-memory-wiki",
            "pages",
        ),
        (
            "memory.wiki_page",
            vec!["core"],
            "aicore-memory-wiki-page",
            "markdown",
        ),
    ] {
        let home = runtime_home(&format!(
            "kernel-readonly-{}-json",
            operation.replace('.', "-")
        ));
        let mut command_args = vec!["kernel", "invoke-readonly", operation];
        command_args.extend(args);

        let output = run_cli_with_env(
            &command_args,
            &[
                ("HOME", home.to_str().expect("home path should be utf-8")),
                ("AICORE_TERMINAL", "json"),
            ],
        );

        assert!(output.status.success(), "{operation} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let events = assert_json_lines(&stdout);
        assert_has_json_event(&events, "kernel.invocation.result");
        let result = events
            .iter()
            .find(|event| event["event"] == "kernel.invocation.result")
            .expect("result event should exist");

        assert_eq!(result["payload"]["operation"], operation);
        assert_eq!(result["payload"]["route"]["component_id"], component_id);
        assert_eq!(result["payload"]["handler"]["kind"], "local_process");
        assert_eq!(result["payload"]["handler"]["spawned_process"], true);
        assert_eq!(result["payload"]["result"]["kind"], operation);
        assert!(result["payload"]["result"]["fields"][field].is_string());
        assert_eq!(
            result["payload"]["result"]["fields"]["kernel_invocation_path"],
            "binary"
        );
        assert!(!stdout.contains("secret_ref"));
    }
}

#[test]
fn memory_search_component_preserves_filters() {
    let root = temp_root("component-memory-search-filters");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "filter shared memory core",
    );
    let _ = seed_memory_record(
        &root,
        MemoryType::Decision,
        MemoryPermanence::Standard,
        "filter shared memory decision",
    );

    let output = run_component_with_payload(
        "__component-memory-search-stdio",
        "memory.search",
        serde_json::json!({
            "query": "decision",
            "type": "decision",
            "limit": 1
        }),
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["fields"]["filters"]["type"], "decision");
    assert_eq!(value["fields"]["result_count"], "1");
    let results = value["fields"]["results"].as_str().expect("results string");
    assert!(results.contains("decision"));
    assert!(!results.contains("filter shared memory core"));
}

#[test]
fn memory_search_component_rejects_invalid_filter_payload() {
    let root = temp_root("component-memory-search-invalid-filter");
    let output = run_component_with_payload(
        "__component-memory-search-stdio",
        "memory.search",
        serde_json::json!({
            "query": "test",
            "invalid_filter": "--unknown"
        }),
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("不支持的 memory.search filter"));
}

#[test]
fn memory_wiki_page_component_rejects_path_traversal() {
    let root = temp_root("component-memory-wiki-traversal");
    let output = run_component_with_payload(
        "__component-memory-wiki-page-stdio",
        "memory.wiki_page",
        serde_json::json!({ "page": "../../secret" }),
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("不允许读取任意 Wiki 路径"));
}

#[test]
fn cli_kernel_invoke_readonly_memory_read_has_no_in_process_fallback() {
    let home = temp_root("kernel-readonly-memory-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_memory_read_manifests(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "memory.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn direct_memory_read_commands_remain_compatible() {
    let root = temp_root("direct-memory-read-compatible");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "direct memory read compatibility",
    );

    for args in [
        vec!["memory", "status"],
        vec!["memory", "search", "direct"],
        vec!["memory", "proposals"],
        vec!["memory", "audit"],
        vec!["memory", "wiki"],
        vec!["memory", "wiki", "core"],
    ] {
        let output = run_cli_with_config_root(&args, &root);
        assert!(output.status.success(), "{args:?} should succeed");
    }
}

fn runtime_home(name: &str) -> std::path::PathBuf {
    let home = temp_root(name);
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_memory_read_manifests(&home);
    home
}

fn run_component(command: &str, operation: &str, root: &std::path::Path) -> std::process::Output {
    let payload = match operation {
        "memory.search" => serde_json::json!({ "query": "component" }),
        "memory.wiki_page" => serde_json::json!({ "page": "core" }),
        _ => serde_json::json!({}),
    };
    run_component_with_payload(command, operation, payload, root)
}

fn run_component_with_payload(
    command: &str,
    operation: &str,
    payload: serde_json::Value,
    root: &std::path::Path,
) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg(command)
        .env("AICORE_CONFIG_ROOT", root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("component handler should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be open")
        .write_all(local_ipc_request(operation, payload).as_bytes())
        .expect("request should be writable");
    child
        .wait_with_output()
        .expect("component handler should finish")
}

fn local_ipc_request(operation: &str, payload: serde_json::Value) -> String {
    serde_json::json!({
        "schema_version": "aicore.local_ipc.invocation.v1",
        "protocol": "stdio_jsonl",
        "protocol_version": "aicore.local_ipc.stdio_jsonl.v1",
        "invocation_id": format!("invoke.test.{operation}"),
        "trace_id": "trace.test",
        "instance_id": "global-main",
        "operation": operation,
        "payload": payload,
        "route": {
            "component_id": format!("aicore-{}", operation.replace('.', "-").replace('_', "-")),
            "app_id": "aicore-cli",
            "capability_id": operation,
            "contract_version": "kernel.app.v1"
        }
    })
    .to_string()
        + "\n"
}
