use super::support::*;

#[test]
fn provider_smoke_reads_real_config_root() {
    let root = temp_root("provider-smoke-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke（local direct）"));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("auth_ref：auth.dummy.main"));
    assert!(stdout.contains("model：dummy/default-chat"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider name：dummy"));
    assert!(stdout.contains("adapter：dummy"));
    assert!(stdout.contains("api mode：dummy"));
    assert!(stdout.contains("engine：dummy"));
    assert!(stdout.contains("engine status：available"));
    assert!(stdout.contains("provider response：skipped"));
    assert!(stdout.contains("runtime output：通过"));
    assert!(stdout.contains("live_call：false"));
    assert!(stdout.contains("sdk_live_call：false"));
    assert!(stdout.contains("network_used：false"));
    assert!(stdout.contains("execution_path：local_direct"));
    assert!(!stdout.contains("secret://"));
    assert!(!stdout.contains("credential_lease_ref"));
}

#[test]
fn cli_provider_smoke_rich_uses_terminal_panel() {
    let root = temp_root("provider-smoke-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["provider", "smoke", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Provider Smoke（local direct）"));
    assert!(stdout.contains("provider：dummy"));
    assert!(!stdout.contains("secret://"));
}

#[test]
fn cli_provider_smoke_plain_has_no_ansi() {
    let root = temp_root("provider-smoke-plain-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["provider", "smoke", "--local"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke（local direct）"));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_provider_smoke_json_outputs_valid_json() {
    let root = temp_root("provider-smoke-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["provider", "smoke", "--local"],
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
    assert_eq!(event["operation"], "provider.smoke");
    assert_eq!(event["fields"]["live_call"], "false");
    assert_eq!(event["fields"]["sdk_live_call"], "false");
    assert_eq!(event["fields"]["network_used"], "false");
    assert!(!stdout.contains("Provider Smoke："));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains("secret://"));
}

#[test]
fn cli_provider_smoke_no_color_has_no_ansi() {
    let root = temp_root("provider-smoke-no-color-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["provider", "smoke", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke（local direct）"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_agent_smoke_runs() {
    let root = temp_root("agent-smoke-runs");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output =
        run_cli_with_config_root(&["agent", "smoke", "--local", "agent smoke request"], &root);

    assert!(output.status.success());
}

#[test]
fn cli_agent_smoke_outputs_chinese_status() {
    let root = temp_root("agent-smoke-chinese-status");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["agent", "smoke", "--local", "继续实现"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Loop（local direct）"));
    assert!(stdout.contains("status：通过"));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("runtime output：已追加"));
}

#[test]
fn cli_agent_smoke_reports_memory_prompt_provider_runtime_status() {
    let root = temp_root("agent-smoke-status-lines");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "agent loop memory context",
    );

    let output = run_cli_with_config_root(&["agent", "smoke", "--local", "agent loop"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("memory pack："));
    assert!(stdout.contains("prompt builder：通过"));
    assert!(stdout.contains("outcome：completed"));
    assert!(stdout.contains("ingress source：cli"));
    assert!(stdout.contains("provider invoked：yes"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider name：dummy"));
    assert!(stdout.contains("assistant output present：yes"));
    assert!(stdout.contains("failure stage：<none>"));
    assert!(stdout.contains("runtime output：已追加"));
    assert!(stdout.contains("event count："));
    assert!(stdout.contains("queue len：0"));
}

#[test]
fn cli_agent_smoke_does_not_print_prompt() {
    let root = temp_root("agent-smoke-no-prompt");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "sensitive prompt context should stay internal",
    );

    let output = run_cli_with_config_root(&["agent", "smoke", "--local", "please answer"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("SYSTEM:"));
    assert!(!stdout.contains("CURRENT USER REQUEST:"));
    assert!(!stdout.contains("sensitive prompt context should stay internal"));
}

#[test]
fn cli_agent_smoke_rich_uses_terminal_summary() {
    let root = temp_root("agent-smoke-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "smoke", "--local", "hello"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Agent Loop"));
    assert!(stdout.contains("outcome：completed"));
    assert!(!stdout.contains("SYSTEM:"));
}

#[test]
fn cli_agent_smoke_plain_has_no_ansi() {
    let root = temp_root("agent-smoke-plain-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "smoke", "--local", "hello"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Loop（local direct）"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_agent_session_smoke_runs() {
    let root = temp_root("agent-session-smoke-runs");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &[
            "agent",
            "session-smoke",
            "--local",
            "第一轮请求",
            "第二轮请求",
        ],
        &root,
    );

    assert!(output.status.success());
}

#[test]
fn cli_agent_session_smoke_rich_uses_terminal_summary() {
    let root = temp_root("agent-session-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "session-smoke", "--local", "first", "second"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Agent Session（local direct）"));
    assert!(stdout.contains("turn 1 outcome：completed"));
    assert!(stdout.contains("turn 2 outcome：completed"));
    assert!(!stdout.contains("SYSTEM:"));
}

#[test]
fn cli_agent_session_smoke_plain_has_no_ansi() {
    let root = temp_root("agent-session-plain-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["agent", "session-smoke", "--local", "first", "second"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Session"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_agent_session_smoke_outputs_chinese_summary() {
    let root = temp_root("agent-session-smoke-summary");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &[
            "agent",
            "session-smoke",
            "--local",
            "第一轮请求",
            "第二轮请求",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Session"));
    assert!(stdout.contains("status：通过"));
    assert!(stdout.contains("conversation："));
    assert!(stdout.contains("turns：2"));
    assert!(stdout.contains("completed all inputs：yes"));
    assert!(stdout.contains("stop reason：<none>"));
    assert!(stdout.contains("latest outcome：completed"));
    assert!(stdout.contains("conversation status：idle"));
    assert!(stdout.contains("turn 1 outcome：completed"));
    assert!(stdout.contains("turn 1 provider invoked：yes"));
    assert!(stdout.contains("turn 1 assistant output present：yes"));
    assert!(stdout.contains("turn 1 failure stage：<none>"));
    assert!(stdout.contains("turn 2 outcome：completed"));
}

#[test]
fn cli_agent_session_smoke_does_not_print_prompt() {
    let root = temp_root("agent-session-smoke-no-prompt");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &[
            "agent",
            "session-smoke",
            "--local",
            "第一轮请求",
            "第二轮请求",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("SYSTEM:"));
    assert!(!stdout.contains("CURRENT USER REQUEST:"));
}

#[test]
fn cli_agent_session_summary_consumes_public_surface() {
    let root = temp_root("agent-session-public-surface");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "session raw memory should stay internal",
    );

    let output = run_cli_with_config_root(
        &[
            "agent",
            "session-smoke",
            "--local",
            "第一轮请求",
            "第二轮请求",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Session"));
    assert!(stdout.contains("status：通过"));
    assert!(stdout.contains("turn 1 assistant output present：yes"));
    assert!(!stdout.contains("session raw memory should stay internal"));
    assert!(!stdout.contains("SYSTEM:"));
    assert!(!stdout.contains("CURRENT USER REQUEST:"));
}

#[test]
fn provider_smoke_fails_when_auth_missing() {
    let root = temp_root("provider-smoke-missing-auth");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少认证池配置，请先运行 config init。"));
}

#[test]
fn cli_agent_smoke_provider_resolve_failure_prints_chinese_error() {
    let root = temp_root("agent-smoke-missing-auth");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.someone.else"
provider = "openrouter"
kind = "api_key"
secret_ref = "secret://auth.someone.else"
capabilities = ["chat"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["agent", "smoke", "--local", "需要失败"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("配置命令失败"));
    assert!(stderr.contains("Agent Turn 失败"));
    assert!(stderr.contains("provider_resolve"));
}

#[test]
fn cli_agent_smoke_non_chat_auth_prints_clear_error() {
    let root = temp_root("agent-smoke-non-chat-auth");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.search.only"
provider = "dummy"
kind = "api_key"
secret_ref = "secret://auth.search.only"
capabilities = ["search"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.search.only"
primary_model = "dummy/default-chat"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["agent", "smoke", "--local", "non chat auth"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("配置命令失败"));
    assert!(stderr.contains("Agent Turn 失败"));
    assert!(stderr.contains("provider_resolve"));
    assert!(stderr.contains("chat capability"));
    assert!(!stderr.contains("secret://"));
}

#[test]
fn cli_provider_smoke_reports_dummy_or_boundary_state_clearly() {
    let root = temp_root("provider-smoke-dummy-clear");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider name：dummy"));
    assert!(stdout.contains("adapter：dummy"));
    assert!(stdout.contains("api mode：dummy"));
    assert!(stdout.contains("engine：dummy"));
    assert!(stdout.contains("engine status：available"));
}

#[test]
fn provider_smoke_local_readonly_does_not_invoke_real_provider() {
    let root = temp_root("provider-smoke-local-readonly-no-live-call");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.openrouter.main"
provider = "openrouter"
kind = "api_key"
secret_ref = "secret://auth.openrouter.main"
capabilities = ["chat"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke（local direct）"));
    assert!(stdout.contains("provider：openrouter"));
    assert!(stdout.contains("live_call：false"));
    assert!(stdout.contains("sdk_live_call：false"));
    assert!(stdout.contains("network_used：false"));
    assert!(stdout.contains("provider response：skipped"));
    assert!(!stdout.contains("secret://"));
}

#[test]
fn cli_provider_capability_failure_does_not_print_secret_ref() {
    let root = temp_root("provider-smoke-non-chat-auth");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.search.only"
provider = "dummy"
kind = "api_key"
secret_ref = "secret://auth.search.only"
capabilities = ["search"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.search.only"
primary_model = "dummy/default-chat"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("配置命令失败"));
    assert!(stderr.contains("provider 解析错误"));
    assert!(stderr.contains("chat capability"));
    assert!(!stderr.contains("secret://"));
}

#[test]
fn cli_agent_smoke_real_provider_unavailable_prints_chinese_error() {
    let root = temp_root("agent-smoke-real-provider-unavailable");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.openrouter.main"
provider = "openrouter"
kind = "api_key"
secret_ref = "secret://auth.openrouter.main"
capabilities = ["chat"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(
        &["agent", "smoke", "--local", "需要 provider gate 失败"],
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("配置命令失败"));
    assert!(stderr.contains("Agent Turn 失败"));
    assert!(stderr.contains("provider_invoke"));
    assert!(stderr.contains("Provider"));
    assert!(!stderr.contains("secret://"));
}

#[test]
fn provider_smoke_fails_when_runtime_missing() {
    let root = temp_root("provider-smoke-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke", "--local"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}
