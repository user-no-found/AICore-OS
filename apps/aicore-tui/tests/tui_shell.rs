use std::process::Command;

#[test]
fn renders_terminal_ai_interaction_shell() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-tui"))
        .output()
        .expect("aicore-tui should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore TUI"));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("状态栏："));
    assert!(stdout.contains("会话输出区："));
    assert!(stdout.contains("工具与任务事件区："));
    assert!(stdout.contains("输入栏："));
    assert!(stdout.contains("当前模式：终端 AI 交互"));
}
