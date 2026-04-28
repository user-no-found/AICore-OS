use super::support::*;

#[test]
fn cli_kernel_invoke_smoke_runs_registered_handler() {
    let home = temp_root("kernel-invoke-existing");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "memory.search"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核调用 Smoke"));
    assert!(stdout.contains("invocation：completed"));
    assert!(stdout.contains("route：routed"));
    assert!(stdout.contains("component：aicore-cli"));
    assert!(stdout.contains("capability：memory.search"));
    assert!(stdout.contains("handler executed：true"));
    assert!(stdout.contains("event generated：true"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger path："));
    assert!(stdout.contains("invocation-ledger.jsonl"));
    assert!(stdout.contains("ledger records：5"));
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    assert!(ledger.contains("\"stage\":\"accepted\""));
    assert!(ledger.contains("\"stage\":\"invocation_completed\""));
}

#[test]
fn cli_kernel_invoke_smoke_reports_missing_handler() {
    let home = temp_root("kernel-invoke-missing-handler");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("provider.smoke", "provider.smoke")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "provider.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核调用失败"));
    assert!(stdout.contains("failure stage：handler_lookup"));
    assert!(stdout.contains("missing handler"));
    assert!(stdout.contains("handler executed：false"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger records：4"));
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    assert!(ledger.contains("\"stage\":\"handler_lookup_failed\""));
    assert!(ledger.contains("\"stage\":\"invocation_failed\""));
}

#[test]
fn cli_kernel_invoke_smoke_json_outputs_valid_json() {
    let home = temp_root("kernel-invoke-json");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "memory.search"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("memory.search"));
    assert!(stdout.contains("handler executed"));
    assert!(stdout.contains("ledger appended"));
    assert!(stdout.contains("ledger records"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_kernel_invoke_smoke_reports_ledger_appended() {
    let home = temp_root("kernel-invoke-ledger-appended");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "memory.search"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger records：5"));
    assert!(stdout.contains("invocation-ledger.jsonl"));
}

#[test]
fn cli_kernel_invoke_smoke_json_reports_ledger_status() {
    let home = temp_root("kernel-invoke-ledger-json-status");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "memory.search"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("ledger appended"));
    assert!(stdout.contains("true"));
    assert!(stdout.contains("ledger records"));
    assert!(stdout.contains("5"));
}

#[test]
fn cli_kernel_invoke_smoke_repeated_operation_writes_distinct_invocation_ids() {
    let home = temp_root("kernel-invoke-distinct-invocation-id");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    for _ in 0..2 {
        let output = run_cli_with_env(
            &["kernel", "invoke-smoke", "memory.search"],
            &[("HOME", home.to_str().expect("home path should be utf-8"))],
        );
        assert!(output.status.success());
    }

    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    let ids = ledger
        .lines()
        .map(|record| extract_json_string(record, "invocation_id"))
        .collect::<Vec<_>>();

    assert_eq!(ids.len(), 10);
    assert_eq!(ids[0], ids[4]);
    assert_eq!(ids[5], ids[9]);
    assert_ne!(ids[0], ids[5]);
    assert_ne!(ids[0], "invoke.memory.search");
    assert_ne!(ids[5], "invoke.memory.search");
}

#[test]
fn cli_kernel_invoke_smoke_outputs_chinese_summary() {
    let home = temp_root("kernel-invoke-chinese");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-smoke", "memory.search"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核调用 Smoke"));
    assert!(stdout.contains("说明：只执行 in-process smoke handler"));
    assert!(stdout.contains("不启动组件进程"));
}

#[test]
fn kernel_readonly_handler_routes_before_execute() {
    let home = temp_root("kernel-readonly-routes-before-execute");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("route：routed"));
    assert!(stdout.contains("operation：runtime.status"));
    assert!(stdout.contains("component：aicore"));
    assert!(stdout.contains("capability：runtime.status"));
    assert!(stdout.contains("handler executed：true"));
}

#[test]
fn kernel_readonly_handler_executes_through_invocation_runtime() {
    let home = temp_root("kernel-readonly-executes");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("invocation：completed"));
    assert!(stdout.contains("kernel runtime binary"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(stdout.contains("result summary："));
    assert!(stdout.contains("foundation installed：yes"));
    assert!(stdout.contains("kernel installed：yes"));
    assert!(stdout.contains("manifest count：1"));
    assert!(stdout.contains("capability count：2"));
}

#[test]
fn kernel_readonly_handler_writes_invocation_ledger_records() {
    let home = temp_root("kernel-readonly-ledger-records");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger records：5"));

    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    assert_eq!(
        ledger_stages(&ledger),
        vec![
            "accepted",
            "route_decision_made",
            "handler_executed",
            "event_generated",
            "invocation_completed"
        ]
    );
}

#[test]
fn kernel_readonly_handler_records_share_invocation_id() {
    let home = temp_root("kernel-readonly-shared-invocation");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    let ids = ledger
        .lines()
        .map(|record| extract_json_string(record, "invocation_id"))
        .collect::<Vec<_>>();

    assert_eq!(ids.len(), 5);
    assert!(ids.iter().all(|id| id == &ids[0]));
    assert_ne!(ids[0], "invoke.runtime.status");
}

#[test]
fn kernel_readonly_handler_result_does_not_expose_raw_payload() {
    let home = temp_root("kernel-readonly-no-sensitive-dump");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");

    assert!(!stdout.contains("KernelInvocationEnvelope"));
    assert!(!stdout.contains("payload"));
    assert!(!stdout.contains("secret_ref"));
    assert!(!ledger.contains("KernelInvocationEnvelope"));
    assert!(!ledger.contains("payload"));
    assert!(!ledger.contains("secret_ref"));
}

#[test]
fn kernel_readonly_handler_failure_records_invocation_failed() {
    let home = temp_root("kernel-readonly-failure-records");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("missing.handler.smoke", "missing.handler.smoke")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "missing.handler.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用失败"));
    assert!(stdout.contains("failure stage：handler_lookup"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger records：4"));

    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");
    assert_eq!(
        ledger_stages(&ledger),
        vec![
            "accepted",
            "route_decision_made",
            "handler_lookup_failed",
            "invocation_failed"
        ]
    );
}

#[test]
fn cli_kernel_invoke_readonly_outputs_chinese_summary() {
    let home = temp_root("kernel-readonly-chinese");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("kernel runtime binary"));
    assert!(stdout.contains("protocol：stdio_jsonl"));
    assert!(stdout.contains("protocol version：aicore.kernel.runtime_binary.stdio_jsonl.v1"));
    assert!(stdout.contains("runtime contract：kernel.runtime.v1"));
    assert!(stdout.contains("binary health：ok"));
}

#[test]
fn cli_kernel_invoke_readonly_json_outputs_valid_json() {
    let home = temp_root("kernel-readonly-json");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "kernel.invocation.result");
    assert!(stdout.contains("runtime.status"));
    assert!(stdout.contains("\"ledger\""));
    assert!(stdout.contains("\"appended\":true"));
    assert!(stdout.contains("kernel_invocation_path"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_kernel_invoke_readonly_json_contains_structured_result_fields() {
    let home = temp_root("kernel-readonly-json-structured-result");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result_event = events
        .iter()
        .find(|event| event["event"] == "kernel.invocation.result")
        .expect("structured result event should exist");

    assert_eq!(result_event["payload"]["operation"], "runtime.status");
    assert_eq!(result_event["payload"]["result"]["kind"], "runtime.status");
    assert_eq!(
        result_event["payload"]["result"]["fields"]["foundation_installed"],
        "yes"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["kernel_installed"],
        "yes"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["manifest_count"],
        "1"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["capability_count"],
        "2"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["kernel_invocation_path"],
        "binary"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["protocol"],
        "stdio_jsonl"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["protocol_version"],
        "aicore.kernel.runtime_binary.stdio_jsonl.v1"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["runtime_binary_contract_version"],
        "kernel.runtime.v1"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["binary_health"],
        "ok"
    );
}

#[test]
fn cli_kernel_invoke_readonly_json_does_not_require_parsing_human_body() {
    let home = temp_root("kernel-readonly-json-not-body");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result_event = events
        .iter()
        .find(|event| event["event"] == "kernel.invocation.result")
        .expect("structured result event should exist");

    assert!(result_event["payload"]["result"].is_object());
    assert!(result_event["payload"]["result"]["fields"].is_object());
    assert_ne!(
        result_event["payload"]["result"]["fields"],
        serde_json::Value::Null
    );
}

#[test]
fn cli_kernel_invoke_readonly_result_does_not_expose_secret_ref() {
    let home = temp_root("kernel-readonly-json-no-secret-ref");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let ledger_path = home
        .join(".aicore")
        .join("state")
        .join("kernel")
        .join("invocation-ledger.jsonl");
    let ledger = fs::read_to_string(ledger_path).expect("ledger should be written");

    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("credential_lease_ref"));
    assert!(!ledger.contains("secret_ref"));
    assert!(!ledger.contains("credential_lease_ref"));
}

#[test]
fn cli_kernel_invoke_readonly_reports_ledger_status() {
    let home = temp_root("kernel-readonly-ledger-status");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("ledger path："));
    assert!(stdout.contains("ledger records：5"));
}

#[test]
fn kernel_invoke_readonly_fails_when_kernel_runtime_binary_missing() {
    let home = temp_root("kernel-readonly-missing-kernel-binary");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用失败"));
    assert!(stdout.contains("failure stage：kernel_runtime_binary_missing"));
    assert!(stdout.contains("protocol：stdio_jsonl"));
    assert!(stdout.contains("kernel runtime health：missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("invocation：completed"));
}

#[test]
fn kernel_invoke_readonly_fails_when_foundation_runtime_binary_missing() {
    let home = temp_root("kernel-readonly-missing-foundation-binary");
    seed_global_runtime_metadata(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用失败"));
    assert!(stdout.contains("failure stage：foundation_runtime_binary_missing"));
    assert!(stdout.contains("protocol：stdio_jsonl"));
    assert!(stdout.contains("foundation runtime health：missing"));
    assert!(stdout.contains("in-process fallback：false"));
}

#[test]
fn app_public_path_does_not_silently_fallback_to_in_process_kernel() {
    let home = temp_root("kernel-readonly-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_route_manifest(
        &home,
        "aicore.toml",
        "aicore",
        &[("runtime.status", "runtime.status")],
    );

    let output = run_cli_with_env(
        &["kernel", "invoke-readonly", "runtime.status"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(!stdout.contains("handler executed：true"));
    assert!(!stdout.contains("first-party in-process adapter：true"));
}
