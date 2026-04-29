use std::io::Write;
use std::process::{Command, Stdio};

use super::support::{
    MemoryTrigger, assert_has_json_event, assert_json_lines, run_cli_with_config_root,
    run_cli_with_config_root_and_env, run_cli_with_env, seed_foundation_runtime_binary,
    seed_global_runtime_metadata, seed_kernel_runtime_binary_fixture, seed_memory_write_manifests,
    seed_rule_based_proposal, temp_root,
};

#[test]
fn memory_write_components_output_single_jsonl_result() {
    let root = temp_root("component-memory-write-jsonl");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：component write",
    );

    for (command, operation, payload, field) in [
        (
            "__component-memory-remember-stdio",
            "memory.remember",
            serde_json::json!({ "content": "component remember write" }),
            "memory_id",
        ),
        (
            "__component-memory-accept-stdio",
            "memory.accept",
            serde_json::json!({ "proposal_id": proposal_id }),
            "proposal_id",
        ),
    ] {
        let output = run_component_with_payload(command, operation, payload, &root);

        assert!(output.status.success(), "{operation} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert_eq!(stdout.lines().count(), 1);
        assert!(!stdout.contains('╭'));
        assert!(!stdout.contains("\u{1b}["));
        assert!(!stdout.contains("component remember write"));
        assert!(!stdout.contains("secret_ref"));

        let value: serde_json::Value =
            serde_json::from_str(stdout.trim()).expect("component output should be json");
        assert_eq!(value["result_kind"], operation);
        assert_eq!(value["fields"]["operation"], operation);
        assert_eq!(value["fields"]["write_applied"], "true");
        assert_eq!(value["fields"]["audit_closed"], "true");
        assert_eq!(value["fields"]["write_outcome"], "applied");
        assert_eq!(value["fields"]["idempotency"], "not_guaranteed");
        assert!(value["fields"][field].is_string());
        assert_eq!(value["fields"]["kernel_invocation_path"], "binary");
    }
}

#[test]
fn memory_reject_component_outputs_single_jsonl_result() {
    let root = temp_root("component-memory-reject-jsonl");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：component reject",
    );

    let output = run_component_with_payload(
        "__component-memory-reject-stdio",
        "memory.reject",
        serde_json::json!({ "proposal_id": proposal_id }),
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert_eq!(stdout.lines().count(), 1);
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("component output should be json");
    assert_eq!(value["fields"]["operation"], "memory.reject");
    assert_eq!(value["fields"]["write_applied"], "true");
    assert_eq!(value["fields"]["audit_closed"], "true");
    assert_eq!(value["fields"]["proposal_id"], proposal_id);
    assert!(!stdout.contains("component reject"));
}

#[test]
fn cli_kernel_invoke_write_memory_remember_json_outputs_structured_fields() {
    let home = runtime_home("kernel-write-memory-remember-json");
    let output = run_cli_with_env(
        &[
            "kernel",
            "invoke-write",
            "memory.remember",
            "kernel write remembered content",
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
    assert_has_json_event(&events, "kernel.invocation.result");
    let result = result_event(&events);

    assert_eq!(result["payload"]["operation"], "memory.remember");
    assert_eq!(
        result["payload"]["route"]["component_id"],
        "aicore-memory-remember"
    );
    assert_eq!(result["payload"]["handler"]["kind"], "local_process");
    assert_eq!(result["payload"]["handler"]["spawned_process"], true);
    assert_eq!(result["payload"]["result"]["kind"], "memory.remember");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["audit_closed"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["write_outcome"],
        "applied"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["idempotency"],
        "not_guaranteed"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["content_present"],
        "true"
    );
    assert!(!stdout.contains("kernel write remembered content"));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn cli_kernel_invoke_write_memory_accept_reject_json_outputs_structured_fields() {
    let home = runtime_home("kernel-write-memory-accept-reject-json");
    let root = home.join("config");
    let accept_proposal =
        seed_rule_based_proposal(&root, MemoryTrigger::ExplicitRemember, "记住：accept write");
    let reject_proposal =
        seed_rule_based_proposal(&root, MemoryTrigger::ExplicitRemember, "记住：reject write");

    for (operation, proposal_id, component_id, status) in [
        (
            "memory.accept",
            accept_proposal.as_str(),
            "aicore-memory-accept",
            "accepted",
        ),
        (
            "memory.reject",
            reject_proposal.as_str(),
            "aicore-memory-reject",
            "rejected",
        ),
    ] {
        let output = run_cli_with_env(
            &["kernel", "invoke-write", operation, proposal_id],
            &[
                ("HOME", home.to_str().expect("home path should be utf-8")),
                (
                    "AICORE_CONFIG_ROOT",
                    root.to_str().expect("config root utf-8"),
                ),
                ("AICORE_TERMINAL", "json"),
            ],
        );

        assert!(output.status.success(), "{operation} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let events = assert_json_lines(&stdout);
        let result = result_event(&events);
        assert_eq!(result["payload"]["operation"], operation);
        assert_eq!(result["payload"]["route"]["component_id"], component_id);
        assert_eq!(result["payload"]["handler"]["kind"], "local_process");
        assert_eq!(
            result["payload"]["result"]["fields"]["write_applied"],
            "true"
        );
        assert_eq!(
            result["payload"]["result"]["fields"]["audit_closed"],
            "true"
        );
        assert_eq!(
            result["payload"]["result"]["fields"]["write_outcome"],
            "applied"
        );
        assert_eq!(
            result["payload"]["result"]["fields"]["proposal_id"],
            proposal_id
        );
        assert_eq!(result["payload"]["result"]["fields"]["status"], status);
        assert!(!stdout.contains("secret_ref"));
        assert!(!stdout.contains("accept write"));
        assert!(!stdout.contains("reject write"));
    }
}

#[test]
fn memory_write_invalid_inputs_are_structured_failure() {
    let home = runtime_home("kernel-write-memory-invalid");
    let root = home.join("config");

    for args in [
        vec!["kernel", "invoke-write", "memory.remember", ""],
        vec!["kernel", "invoke-write", "memory.accept", "prop_missing"],
        vec!["kernel", "invoke-write", "memory.reject", "prop_missing"],
    ] {
        let output = run_cli_with_env(
            &args,
            &[
                ("HOME", home.to_str().expect("home path should be utf-8")),
                (
                    "AICORE_CONFIG_ROOT",
                    root.to_str().expect("config root utf-8"),
                ),
                ("AICORE_TERMINAL", "json"),
            ],
        );

        assert!(!output.status.success(), "{args:?} should fail");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let events = assert_json_lines(&stdout);
        let result = result_event(&events);
        assert_eq!(result["payload"]["status"], "failed");
        assert_eq!(
            result["payload"]["result"]["fields"]["write_applied"],
            "false"
        );
        assert_eq!(
            result["payload"]["result"]["fields"]["write_outcome"],
            "failed"
        );
        assert!(!stdout.contains("secret_ref"));
    }
}

#[test]
fn cli_kernel_invoke_write_memory_has_no_in_process_fallback() {
    let home = temp_root("kernel-write-memory-no-fallback");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_memory_write_manifests(&home);

    let output = run_cli_with_env(
        &[
            "kernel",
            "invoke-write",
            "memory.remember",
            "no fallback write",
        ],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn memory_accept_kernel_native_default_path() {
    let home = runtime_home("memory-accept-kernel-native-default");
    let root = home.join("config");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：kernel native accept test",
    );

    let output = run_cli_with_env(
        &["memory", "accept", &proposal_id],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_reject_kernel_native_default_path() {
    let home = runtime_home("memory-reject-kernel-native-default");
    let root = home.join("config");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：kernel native reject test",
    );

    let output = run_cli_with_env(
        &["memory", "reject", &proposal_id],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_accept_local_direct_human_path() {
    let root = temp_root("memory-accept-local-direct-human");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local direct accept test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "accept", &proposal_id, "--local"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已接受（local direct）"));
    assert!(stdout.contains("execution_path: local_direct"));
    assert!(stdout.contains("kernel_invocation_path: not_used"));
    assert!(stdout.contains("ledger_appended: false"));
    assert!(stdout.contains("proposal: "));
    assert!(stdout.contains("memory: mem_"));
}

#[test]
fn memory_reject_local_direct_human_path() {
    let root = temp_root("memory-reject-local-direct-human");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local direct reject test",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "reject", &proposal_id, "--local"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已拒绝（local direct）"));
    assert!(stdout.contains("execution_path: local_direct"));
    assert!(stdout.contains("kernel_invocation_path: not_used"));
    assert!(stdout.contains("ledger_appended: false"));
    assert!(stdout.contains("proposal: "));
}

#[test]
fn memory_accept_kernel_native_json_path() {
    let home = runtime_home("memory-accept-kernel-native-json");
    let root = home.join("config");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：kernel native accept json",
    );

    let output = run_cli_with_env(
        &["memory", "accept", &proposal_id],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result = result_event(&events);
    assert_eq!(result["payload"]["operation"], "memory.accept");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["audit_closed"],
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
fn memory_reject_kernel_native_json_path() {
    let home = runtime_home("memory-reject-kernel-native-json");
    let root = home.join("config");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：kernel native reject json",
    );

    let output = run_cli_with_env(
        &["memory", "reject", &proposal_id],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result = result_event(&events);
    assert_eq!(result["payload"]["operation"], "memory.reject");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "true"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["kernel_invocation_path"],
        "binary"
    );
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_accept_local_direct_json_path() {
    let root = temp_root("memory-accept-local-direct-json");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local direct accept json",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "accept", &proposal_id, "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(events.iter().any(|e| e["event"] == "direct.command.result"));
    let event = events
        .iter()
        .find(|e| e["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.accept");
    assert_eq!(event["success"], true);
    assert_eq!(event["execution_path"], "local_direct");
    assert_eq!(event["kernel_invocation_path"], "not_used");
    assert_eq!(event["ledger_appended"], false);
    assert_eq!(event["fields"]["operation"], "memory.accept");
    assert_eq!(event["fields"]["write_applied"], "true");
    assert_eq!(event["fields"]["kernel_invocation_path"], "binary");
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_reject_local_direct_json_path() {
    let root = temp_root("memory-reject-local-direct-json");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local direct reject json",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "reject", &proposal_id, "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(events.iter().any(|e| e["event"] == "direct.command.result"));
    let event = events
        .iter()
        .find(|e| e["event"] == "direct.command.result")
        .expect("direct.command.result should exist");
    assert_eq!(event["operation"], "memory.reject");
    assert_eq!(event["success"], true);
    assert_eq!(event["execution_path"], "local_direct");
    assert_eq!(event["kernel_invocation_path"], "not_used");
    assert_eq!(event["ledger_appended"], false);
    assert_eq!(event["fields"]["operation"], "memory.reject");
    assert_eq!(event["fields"]["write_applied"], "true");
    assert_eq!(event["fields"]["kernel_invocation_path"], "binary");
    assert!(!stdout.contains("secret_ref"));
}

#[test]
fn memory_accept_invalid_proposal_id_structured_failure() {
    let home = runtime_home("memory-accept-invalid-json");
    let root = home.join("config");

    let output = run_cli_with_env(
        &["memory", "accept", "prop_missing"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result = result_event(&events);
    assert_eq!(result["payload"]["status"], "failed");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "false"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["write_outcome"],
        "failed"
    );
}

#[test]
fn memory_reject_invalid_proposal_id_structured_failure() {
    let home = runtime_home("memory-reject-invalid-json");
    let root = home.join("config");

    let output = run_cli_with_env(
        &["memory", "reject", "prop_missing"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            (
                "AICORE_CONFIG_ROOT",
                root.to_str().expect("config root utf-8"),
            ),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    let result = result_event(&events);
    assert_eq!(result["payload"]["status"], "failed");
    assert_eq!(
        result["payload"]["result"]["fields"]["write_applied"],
        "false"
    );
    assert_eq!(
        result["payload"]["result"]["fields"]["write_outcome"],
        "failed"
    );
}

#[test]
fn memory_accept_local_flag_not_polluting_proposal_id() {
    let root = temp_root("memory-accept-local-flag-position");
    let proposal_id_first = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local flag position test first",
    );
    let proposal_id_second = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：local flag position test second",
    );

    for args in [
        vec!["memory", "accept", &proposal_id_first, "--local"],
        vec!["memory", "accept", "--local", &proposal_id_second],
    ] {
        let output = run_cli_with_config_root_and_env(&args, &root, &[("AICORE_TERMINAL", "json")]);
        assert!(output.status.success(), "{args:?} should succeed");
        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        let events = assert_json_lines(&stdout);
        assert!(
            events.iter().any(|e| e["event"] == "direct.command.result"),
            "{args:?} should emit direct.command.result"
        );
    }
}

#[test]
fn memory_remember_default_path_unchanged() {
    let root = temp_root("memory-remember-unchanged");

    let output =
        run_cli_with_config_root(&["memory", "remember", "remember unchanged test"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆已写入"));
    assert!(stdout.contains("id: mem_"));
    assert!(stdout.contains("type: core"));
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

fn runtime_home(name: &str) -> std::path::PathBuf {
    let home = temp_root(name);
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_memory_write_manifests(&home);
    home
}

fn result_event(events: &[serde_json::Value]) -> &serde_json::Value {
    events
        .iter()
        .find(|event| event["event"] == "kernel.invocation.result")
        .expect("result event should exist")
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
