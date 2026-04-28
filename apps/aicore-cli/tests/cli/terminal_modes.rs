use super::support::*;

#[test]
fn cli_kernel_invoke_process_smoke_outputs_chinese_summary() {
    let home = temp_root("kernel-process-smoke-chinese");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_process_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核组件进程调用 Smoke"));
    assert!(stdout.contains("invocation：completed"));
    assert!(stdout.contains("invocation mode：local_process"));
    assert!(stdout.contains("transport：stdio_jsonl"));
    assert!(stdout.contains("handler kind：local_process"));
    assert!(stdout.contains("spawned process：true"));
    assert!(stdout.contains("event generated：true"));
    assert!(stdout.contains("ledger appended：true"));
    assert!(stdout.contains("result kind：component.process.smoke"));
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(stdout.contains("只验证 local process boundary"));
}

#[test]
fn cli_kernel_invoke_process_smoke_json_outputs_structured_result() {
    let home = temp_root("kernel-process-smoke-json");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_process_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
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
        .expect("structured process result event should exist");

    assert_eq!(
        result_event["payload"]["operation"],
        "component.process.smoke"
    );
    assert_eq!(result_event["payload"]["handler"]["kind"], "local_process");
    assert_eq!(
        result_event["payload"]["handler"]["invocation_mode"],
        "local_process"
    );
    assert_eq!(
        result_event["payload"]["handler"]["transport"],
        "stdio_jsonl"
    );
    assert_eq!(result_event["payload"]["handler"]["spawned_process"], true);
    assert_eq!(
        result_event["payload"]["result"]["kind"],
        "component.process.smoke"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["ipc"],
        "stdio_jsonl"
    );
    assert_eq!(
        result_event["payload"]["result"]["fields"]["kernel_invocation_path"],
        "binary"
    );
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_kernel_invoke_process_smoke_writes_process_metadata_to_ledger() {
    let home = temp_root("kernel-process-smoke-ledger");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_process_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
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
    assert!(ledger.contains("\"handler_kind\":\"local_process\""));
    assert!(ledger.contains("\"spawned_process\":true"));
    assert!(ledger.contains("\"transport\":\"stdio_jsonl\""));
    assert!(!ledger.contains("secret_ref"));
    assert!(!ledger.contains("raw provider"));
}

#[test]
fn cli_invoke_process_smoke_uses_installed_kernel_runtime_binary() {
    let home = temp_root("kernel-process-smoke-installed-binary");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_kernel_runtime_binary_fixture(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel invocation path：binary"));
    assert!(stdout.contains("handler kind：local_process"));
    assert!(stdout.contains("spawned process：true"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(stdout.contains("process smoke handled by installed kernel runtime binary"));
}

#[test]
fn missing_kernel_runtime_binary_blocks_process_smoke_without_fallback() {
    let home = temp_root("kernel-process-smoke-missing-kernel");
    seed_global_runtime_metadata(&home);
    seed_foundation_runtime_binary(&home);
    seed_process_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("kernel_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("invocation：completed"));
    assert!(!stdout.contains("handler executed：true"));
}

#[test]
fn missing_foundation_runtime_binary_blocks_process_smoke_without_fallback() {
    let home = temp_root("kernel-process-smoke-missing-foundation");
    seed_global_runtime_metadata(&home);
    seed_kernel_runtime_binary_fixture(&home);
    seed_process_smoke_manifest(&home);

    let output = run_cli_with_env(
        &["kernel", "invoke-process-smoke", "component.process.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("foundation_runtime_binary_missing"));
    assert!(stdout.contains("in-process fallback：false"));
    assert!(!stdout.contains("invocation：completed"));
    assert!(!stdout.contains("handler executed：true"));
}
#[test]
fn renders_runtime_smoke_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["runtime", "smoke"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Runtime Smoke："));
    assert!(stdout.contains("CLI 场景："));
    assert!(stdout.contains("接收决策：StartTurn"));
    assert!(stdout.contains("账本消息数：2"));
    assert!(stdout.contains("输出目标：active-views"));
    assert!(stdout.contains("投递身份：active-views"));
    assert!(stdout.contains("External Origin 场景："));
    assert!(stdout.contains("输出目标：origin"));
    assert!(stdout.contains("投递身份：external:feishu:chat-1"));
    assert!(stdout.contains("Follow 场景："));
    assert!(stdout.contains("输出目标：followed-external"));
    assert!(stdout.contains("投递身份：external:feishu:chat-2"));
}

#[test]
fn cli_runtime_smoke_rich_uses_terminal_panel() {
    let output = run_cli_with_env(&["runtime", "smoke"], &[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Runtime Smoke"));
    assert!(stdout.contains("CLI 场景"));
    assert!(stdout.contains("接收决策：StartTurn"));
    assert!(stdout.contains("Follow 场景"));
}

#[test]
fn cli_runtime_smoke_plain_has_no_ansi() {
    let output = run_cli_with_env(&["runtime", "smoke"], &[("AICORE_TERMINAL", "plain")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Runtime Smoke："));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_runtime_smoke_json_outputs_valid_json() {
    let output = run_cli_with_env(&["runtime", "smoke"], &[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("StartTurn"));
    assert!(stdout.contains("followed-external"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn renders_config_smoke_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "smoke"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置 Smoke Test："));
    assert!(stdout.contains("默认配置文件：通过"));
    assert!(stdout.contains("认证池保存/读取：通过"));
    assert!(stdout.contains("实例运行配置保存/读取：通过"));
    assert!(stdout.contains("服务角色配置保存/读取：通过"));
    assert!(stdout.contains("配置校验：通过"));
}

#[test]
fn cli_config_smoke_rich_uses_terminal_panel() {
    let output = run_cli_with_env(&["config", "smoke"], &[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 配置 Smoke Test"));
    assert!(stdout.contains("默认配置文件：通过"));
    assert!(stdout.contains("配置校验：通过"));
}

#[test]
fn cli_config_smoke_plain_has_no_ansi() {
    let output = run_cli_with_env(&["config", "smoke"], &[("AICORE_TERMINAL", "plain")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置 Smoke Test："));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_config_smoke_json_outputs_valid_json() {
    let output = run_cli_with_env(&["config", "smoke"], &[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("默认配置文件"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_config_smoke_no_color_has_no_ansi() {
    let output = run_cli_with_env(
        &["config", "smoke"],
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置 Smoke Test"));
    assert!(!stdout.contains("\u{1b}["));
}
