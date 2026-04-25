use std::process::Command;

#[test]
fn renders_minimal_system_status_by_default() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore"))
        .output()
        .expect("aicore binary should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("AICore OS"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(stdout.contains("主实例工作目录："));
    assert!(stdout.contains("主实例状态目录："));
    assert!(stdout.contains("组件数量："));
    assert!(stdout.contains("实例数量："));
    assert!(stdout.contains("Runtime：global-main/main"));
}
