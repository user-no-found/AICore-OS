use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    MemoryPermanence, MemoryType, assert_has_json_event, assert_json_lines,
    run_cli_with_config_root, run_cli_with_config_root_and_env, run_cli_with_env,
    seed_foundation_runtime_binary, seed_global_runtime_metadata,
    seed_kernel_runtime_binary_fixture, seed_memory_read_manifests, seed_memory_record, temp_root,
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
        vec!["memory", "status", "--local"],
        vec!["memory", "search", "direct", "--local"],
        vec!["memory", "proposals", "--local"],
        vec!["memory", "audit", "--local"],
        vec!["memory", "wiki", "--local"],
        vec!["memory", "wiki", "core", "--local"],
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

#[test]
fn memory_status_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "status"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("memory.status"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn memory_search_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "search", "test"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("memory.search"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn memory_proposals_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "proposals"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("memory.proposals"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
}

#[test]
fn memory_audit_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "audit"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("memory.audit"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
}

#[test]
fn memory_wiki_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["memory", "wiki"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("memory.wiki"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
}

#[test]
fn cli_memory_status_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-status-local-rich");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Status（local direct）"));
    assert!(stdout.contains("instance: global-main"));
    assert!(!stdout.contains("Memory Status："));
}

#[test]
fn cli_memory_status_local_json_outputs_valid_json() {
    let root = temp_root("memory-status-local-json");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.status");
    assert_eq!(event["fields"]["operation"], "memory.status");
    assert!(!stdout.contains("Memory Status："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_search_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-search-local-rich");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "local search test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "search", "local", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆搜索（local direct）"));
    assert!(stdout.contains("local search test"));
    assert!(!stdout.contains("记忆搜索："));
}

#[test]
fn cli_memory_search_local_json_outputs_valid_json() {
    let root = temp_root("memory-search-local-json");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "local json search",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "search", "json", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.search");
    assert_eq!(event["fields"]["operation"], "memory.search");
    assert!(!stdout.contains("记忆搜索："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_search_local_with_filters() {
    let root = temp_root("memory-search-local-filters");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "filter shared core",
    );
    let _ = seed_memory_record(
        &root,
        MemoryType::Decision,
        MemoryPermanence::Standard,
        "filter shared decision",
    );

    let output = run_cli_with_config_root(
        &[
            "memory", "search", "filter", "--type", "decision", "--local",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("[decision]"));
    assert!(!stdout.contains("[core]"));
}

#[test]
fn cli_memory_search_local_filter_at_end() {
    let root = temp_root("memory-search-local-filter-end");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "end filter core",
    );

    let output = run_cli_with_config_root(
        &["memory", "search", "end", "--limit", "1", "--local"],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("end filter core"));
}

#[test]
fn cli_memory_proposals_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-proposals-local-rich");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "proposals", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Proposals（local direct）"));
    assert!(!stdout.contains("Memory Proposals："));
}

#[test]
fn cli_memory_proposals_local_json_outputs_valid_json() {
    let root = temp_root("memory-proposals-local-json");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "proposals", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.proposals");
    assert_eq!(event["fields"]["operation"], "memory.proposals");
    assert!(!stdout.contains("Memory Proposals："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_audit_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-audit-local-rich");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "audit", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Audit（local direct）"));
    assert!(stdout.contains("status: ok"));
    assert!(!stdout.contains("Memory Audit："));
}

#[test]
fn cli_memory_audit_local_json_outputs_valid_json() {
    let root = temp_root("memory-audit-local-json");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["memory", "audit", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.audit");
    assert_eq!(event["fields"]["operation"], "memory.audit");
    assert!(!stdout.contains("Memory Audit："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_wiki_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-wiki-local-rich");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki local test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆 Wiki Projection（local direct）"));
    assert!(stdout.contains("- page: index"));
    assert!(!stdout.contains("记忆 Wiki Projection："));
}

#[test]
fn cli_memory_wiki_local_json_outputs_valid_json() {
    let root = temp_root("memory-wiki-local-json");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki local json",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.wiki");
    assert_eq!(event["fields"]["operation"], "memory.wiki");
    assert!(!stdout.contains("记忆 Wiki Projection："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_wiki_page_local_rich_uses_terminal_panel() {
    let root = temp_root("memory-wiki-page-local-rich");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki page local test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "core", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆 Wiki Projection（local direct）"));
    assert!(stdout.contains("- page: core"));
    assert!(!stdout.contains("记忆 Wiki Projection："));
}

#[test]
fn cli_memory_wiki_page_local_json_outputs_valid_json() {
    let root = temp_root("memory-wiki-page-local-json");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki page local json",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "core", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    let event = events
        .iter()
        .find(|event| event["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.wiki_page");
    assert_eq!(event["fields"]["operation"], "memory.wiki_page");
    assert!(!stdout.contains("记忆 Wiki Projection："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_wiki_page_local_rejects_path_traversal() {
    let root = temp_root("memory-wiki-page-local-traversal");
    let output = run_cli_with_config_root(&["memory", "wiki", "../../secret", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("不允许读取任意 Wiki 路径"));
}

#[test]
fn cli_memory_wiki_page_local_position_before_page() {
    let root = temp_root("memory-wiki-page-local-pos");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki pos test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "--local", "core"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: core"));
    assert!(stdout.contains("local direct"));
}
