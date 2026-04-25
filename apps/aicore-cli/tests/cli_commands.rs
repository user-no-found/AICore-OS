use std::{fs, path::PathBuf, process::Command};

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("aicore-cli-p46-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    root
}

fn run_cli_with_config_root(args: &[&str], root: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(args)
        .env("AICORE_CONFIG_ROOT", root)
        .output()
        .expect("aicore-cli should run")
}

#[test]
fn renders_status_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("status")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore CLI"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(stdout.contains("组件数量："));
    assert!(stdout.contains("实例数量："));
    assert!(stdout.contains("Runtime：global-main/main"));
}

#[test]
fn renders_instance_list_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["instance", "list"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例列表："));
    assert!(stdout.contains("global-main"));
    assert!(stdout.contains("global_main"));
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
fn auth_list_reads_real_config_root() {
    let root = temp_root("auth-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["auth", "list"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("认证池："));
    assert!(stdout.contains("auth.openrouter.main"));
    assert!(stdout.contains("provider: openrouter"));
    assert!(stdout.contains("kind: api_key"));
    assert!(stdout.contains("enabled: true"));
    assert!(stdout.contains("capabilities: chat, vision"));
    assert!(stdout.contains("secret_ref: secret://auth.openrouter.main"));
}

#[test]
fn model_show_reads_real_config_root() {
    let root = temp_root("model-show-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["model", "show"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例模型配置："));
    assert!(stdout.contains("instance: global-main"));
    assert!(stdout.contains("primary:"));
    assert!(stdout.contains("auth_ref: auth.openrouter.main"));
    assert!(stdout.contains("model: openai/gpt-5"));
    assert!(stdout.contains("fallback:"));
    assert!(stdout.contains("auth_ref: auth.openai.backup"));
    assert!(stdout.contains("model: gpt-4.1"));
}

#[test]
fn service_list_reads_real_config_root() {
    let root = temp_root("service-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["service", "list"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("服务角色配置："));
    assert!(stdout.contains("memory_dreamer"));
    assert!(stdout.contains("mode: inherit_instance"));
    assert!(stdout.contains("evolution_reviewer"));
    assert!(stdout.contains("mode: disabled"));
    assert!(stdout.contains("search"));
    assert!(stdout.contains("mode: explicit"));
    assert!(stdout.contains("auth_ref: auth.openrouter.search"));
    assert!(stdout.contains("model: perplexity/sonar"));
}

#[test]
fn auth_list_fails_when_config_missing() {
    let root = temp_root("auth-list-missing");
    let output = run_cli_with_config_root(&["auth", "list"], &root);

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

    let output = run_cli_with_config_root(&["model", "show"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn service_list_fails_when_services_missing() {
    let root = temp_root("service-list-missing");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");

    let output = run_cli_with_config_root(&["service", "list"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少服务角色配置，请先运行 config init。"));
}

#[test]
fn provider_smoke_reads_real_config_root() {
    let root = temp_root("provider-smoke-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke："));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("auth_ref：auth.openrouter.main"));
    assert!(stdout.contains("model：openai/gpt-5"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider response：通过"));
    assert!(stdout.contains("runtime output：通过"));
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

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少认证池配置，请先运行 config init。"));
}

#[test]
fn provider_smoke_fails_when_runtime_missing() {
    let root = temp_root("provider-smoke-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

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
    assert!(stdout.contains("配置路径："));
    assert!(stdout.contains(&format!("root: {}", root.display())));
    assert!(stdout.contains(&format!("auth.toml: {}", root.join("auth.toml").display())));
    assert!(stdout.contains(&format!(
        "services.toml: {}",
        root.join("services.toml").display()
    )));
    assert!(stdout.contains(&format!("instances: {}", root.join("instances").display())));
    assert!(stdout.contains(&format!(
        "global-main runtime: {}",
        root.join("instances").join("global-main").join("runtime.toml").display()
    )));
}

#[test]
fn config_path_uses_default_home_root_without_override() {
    let home = temp_root("config-path-home");
    let expected_root = home.join(".aicore").join("config");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .env("HOME", &home)
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(&format!("root: {}", expected_root.display())));
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
fn config_validate_accepts_initialized_config() {
    let root = temp_root("config-validate-ok");

    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置校验："));
    assert!(stdout.contains("认证池：已读取"));
    assert!(stdout.contains("实例运行配置：通过"));
    assert!(stdout.contains("服务角色配置：通过"));
}

#[test]
fn config_validate_fails_when_runtime_missing() {
    let root = temp_root("config-validate-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
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
