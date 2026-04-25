use std::process::Command;

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
fn renders_auth_list_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["auth", "list"])
        .output()
        .expect("aicore-cli should run");

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
fn renders_model_show_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["model", "show"])
        .output()
        .expect("aicore-cli should run");

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
fn renders_service_list_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["service", "list"])
        .output()
        .expect("aicore-cli should run");

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
