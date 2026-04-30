use super::support::*;

#[test]
fn renders_config_path_command() {
    let root = temp_root("config-path");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置路径"));
    assert!(stdout.contains(&format!("root：{}", root.display())));
    assert!(stdout.contains(&format!("auth.toml：{}", root.join("auth.toml").display())));
    assert!(stdout.contains(&format!(
        "services.toml：{}",
        root.join("services.toml").display()
    )));
    assert!(stdout.contains(&format!("instances：{}", root.join("instances").display())));
    assert!(stdout.contains(&format!(
        "global-main runtime：{}",
        root.join("instances")
            .join("global-main")
            .join("runtime.toml")
            .display()
    )));
}

#[test]
fn cli_config_path_rich_uses_terminal_panel() {
    let root = temp_root("config-path-rich-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "path"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 配置路径"));
    assert!(stdout.contains(&format!("root：{}", root.display())));
}

#[test]
fn cli_config_path_plain_has_no_ansi() {
    let root = temp_root("config-path-plain-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "path"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置路径"));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_config_path_json_outputs_valid_json() {
    let root = temp_root("config-path-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "path"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(events.iter().any(|event| event["event"] == "block.panel"));
    assert!(!stdout.contains("配置路径："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_config_path_no_color_has_no_ansi() {
    let root = temp_root("config-path-no-color-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "path"],
        &root,
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置路径"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn config_path_uses_default_home_root_without_override() {
    let home = temp_root("config-path-home");
    let expected_root = home.join(".aicore").join("config");
    fs::create_dir_all(&home).expect("home should create");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .current_dir(&home)
        .env("HOME", &home)
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(&format!("root：{}", expected_root.display())));
}

#[test]
fn config_path_uses_workspace_instance_root_without_override() {
    let home = temp_root("config-path-workspace-home");
    let workspace = home.join("project");
    fs::create_dir_all(&workspace).expect("workspace should create");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .current_dir(&workspace)
        .env("HOME", &home)
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(&format!(
        "root：{}",
        workspace.join(".aicore").join("config").display()
    )));
    assert!(workspace.join(".aicore").join("soul.md").exists());
    assert!(workspace.join(".gitignore").exists());
    let gitignore =
        fs::read_to_string(workspace.join(".gitignore")).expect("gitignore should read");
    assert!(gitignore.contains(".aicore/"));
}

#[test]
fn config_path_uses_ancestor_workspace_marker_without_override() {
    let home = temp_root("config-path-workspace-ancestor-home");
    let workspace = home.join("project");
    let nested = workspace.join("src/bin");
    fs::create_dir_all(workspace.join(".aicore")).expect("workspace marker should create");
    fs::create_dir_all(&nested).expect("nested cwd should create");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .current_dir(&nested)
        .env("HOME", &home)
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(&format!(
        "root：{}",
        workspace.join(".aicore").join("config").display()
    )));
}

#[test]
fn cli_config_init_rich_uses_terminal_panel() {
    let root = temp_root("config-init-rich-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "init"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 配置初始化"));
    assert!(stdout.contains("auth.toml：已创建"));
}

#[test]
fn cli_config_init_json_outputs_valid_json() {
    let root = temp_root("config-init-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["config", "init"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert!(events.iter().any(|event| event["event"] == "block.panel"));
    assert!(!stdout.contains("配置初始化："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn config_init_creates_real_config_files_under_override_root() {
    let root = temp_root("config-init");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert!(root.join("auth.toml").exists());
    assert!(root.join("services.toml").exists());
    assert!(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml")
            .exists()
    );
}

#[test]
fn config_init_does_not_overwrite_existing_files() {
    let root = temp_root("config-init-no-overwrite");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(root.join("auth.toml"), "sentinel-auth").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "sentinel-services")
        .expect("services.toml should be writable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        "sentinel-runtime",
    )
    .expect("runtime.toml should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(root.join("auth.toml")).expect("auth.toml should remain readable"),
        "sentinel-auth"
    );
    assert_eq!(
        fs::read_to_string(root.join("services.toml"))
            .expect("services.toml should remain readable"),
        "sentinel-services"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("instances")
                .join("global-main")
                .join("runtime.toml")
        )
        .expect("runtime.toml should remain readable"),
        "sentinel-runtime"
    );
}

#[test]
fn cli_config_validate_rich_uses_terminal_panel() {
    let root = temp_root("config-validate-rich-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["config", "validate", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 配置校验（local direct）"));
    assert!(stdout.contains("runtime_config_present：true"));
}

#[test]
fn cli_config_validate_json_outputs_valid_json() {
    let root = temp_root("config-validate-json-terminal");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root_and_env(
        &["config", "validate", "--local"],
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
    assert!(stdout.contains("config.validate"));
    assert!(!stdout.contains("配置校验："));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn config_validate_accepts_initialized_config() {
    let root = temp_root("config-validate-ok");

    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate", "--local"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置校验（local direct）"));
    assert!(stdout.contains("auth_pool_present：true"));
    assert!(stdout.contains("runtime_config_present：true"));
    assert!(stdout.contains("service_profiles_present：true"));
    assert!(stdout.contains("execution_path：local_direct"));
}

#[test]
fn config_validate_fails_when_runtime_missing() {
    let root = temp_root("config-validate-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate", "--local"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn config_validate_kernel_native_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核只读调用"));
    assert!(stdout.contains("config.validate"));
    assert!(stdout.contains("kernel invocation path"));
    assert!(stdout.contains("binary"));
    assert!(stdout.contains("in-process fallback"));
    assert!(stdout.contains("false"));
}

#[test]
fn config_smoke_still_uses_temp_demo_root() {
    let root = temp_root("config-smoke-real-root");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "smoke"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert!(!root.exists());
}
