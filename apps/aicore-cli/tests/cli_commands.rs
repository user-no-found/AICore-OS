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
