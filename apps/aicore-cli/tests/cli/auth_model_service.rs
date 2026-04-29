use super::support::*;

#[test]
fn auth_list_reads_real_config_root() {
    let root = temp_root("auth-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["auth", "list", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("认证池（local direct）"));
    assert!(stdout.contains("auth.dummy.main"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("auth.openrouter.main"));
    assert!(stdout.contains("provider：openrouter"));
    assert!(stdout.contains("kind：api-key"));
    assert!(stdout.contains("enabled：true"));
    assert!(stdout.contains("capabilities：chat, vision"));
    assert!(stdout.contains("secret：configured"));
    assert!(stdout.contains("execution_path：local_direct"));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
}

#[test]
fn cli_auth_list_rich_uses_terminal_table_or_panel() {
    let root = temp_root("auth-list-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["auth", "list", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 认证池（local direct）"));
    assert!(stdout.contains("auth.dummy.main"));
    assert!(stdout.contains("secret：configured"));
}

#[test]
fn cli_auth_list_json_outputs_valid_json() {
    let root = temp_root("auth-list-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["auth", "list", "--local"],
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
    assert_eq!(event["operation"], "auth.list");
    assert!(stdout.contains("auth.dummy.main"));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_auth_list_does_not_expose_secret_material() {
    let root = temp_root("auth-list-no-secret-material");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["auth", "list", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("secret：configured"));
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://"));
}

#[test]
fn cli_auth_list_does_not_print_secret_ref() {
    let root = temp_root("auth-list-no-secret-ref");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["auth", "list", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("secret_ref"));
    assert!(!stdout.contains("secret://auth.openrouter.main"));
}

#[test]
fn model_show_reads_real_config_root() {
    let root = temp_root("model-show-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["model", "show", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例模型配置（local direct）"));
    assert!(stdout.contains("instance：global-main"));
    assert!(stdout.contains("primary auth_ref：auth.dummy.main"));
    assert!(stdout.contains("primary model：dummy/default-chat"));
    assert!(stdout.contains("fallback auth_ref：auth.openrouter.main"));
    assert!(stdout.contains("fallback model：openai/gpt-5"));
    assert!(stdout.contains("execution_path：local_direct"));
}

#[test]
fn cli_model_show_rich_uses_terminal_panel() {
    let root = temp_root("model-show-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["model", "show", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 实例模型配置（local direct）"));
    assert!(stdout.contains("primary auth_ref：auth.dummy.main"));
}

#[test]
fn cli_model_show_json_outputs_valid_json() {
    let root = temp_root("model-show-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["model", "show", "--local"],
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
    assert!(stdout.contains("model.show"));
    assert!(!stdout.contains("实例模型配置："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn service_list_reads_real_config_root() {
    let root = temp_root("service-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["service", "list", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("服务角色配置（local direct）"));
    assert!(stdout.contains("memory_dreamer mode：inherit_instance"));
    assert!(stdout.contains("evolution_reviewer mode：disabled"));
    assert!(stdout.contains("search mode：explicit"));
    assert!(stdout.contains("search auth_ref：auth.openrouter.search"));
    assert!(stdout.contains("search model：perplexity/sonar"));
    assert!(stdout.contains("execution_path：local_direct"));
}

#[test]
fn cli_service_list_rich_uses_terminal_panel_or_table() {
    let root = temp_root("service-list-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["service", "list", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 服务角色配置（local direct）"));
    assert!(stdout.contains("memory_dreamer mode：inherit_instance"));
    assert!(stdout.contains("search auth_ref：auth.openrouter.search"));
}

#[test]
fn cli_service_list_json_outputs_valid_json() {
    let root = temp_root("service-list-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["service", "list", "--local"],
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
    assert!(stdout.contains("service.list"));
    assert!(!stdout.contains("服务角色配置："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn auth_list_fails_when_config_missing() {
    let root = temp_root("auth-list-missing");
    let output = run_cli_with_config_root(&["auth", "list", "--local"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少认证池配置，请先运行 config init。"));
}

#[test]
fn model_show_fails_when_runtime_missing() {
    let root = temp_root("model-show-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["model", "show", "--local"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn service_list_fails_when_services_missing() {
    let root = temp_root("service-list-missing");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");

    let output = run_cli_with_config_root(&["service", "list", "--local"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少服务角色配置，请先运行 config init。"));
}

#[test]
fn auth_list_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["auth", "list"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("auth.list"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn model_show_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["model", "show"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("model.show"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn service_list_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["service", "list"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("service.list"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}
