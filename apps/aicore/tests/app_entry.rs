use std::process::Command;

fn run_aicore_with_env(envs: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aicore"));
    command.env_remove("AICORE_TERMINAL");
    command.env_remove("NO_COLOR");
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("aicore binary should run")
}

fn assert_json_lines(stdout: &str) -> Vec<serde_json::Value> {
    let lines = stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    assert!(!lines.is_empty(), "json mode should emit at least one line");
    lines
        .into_iter()
        .map(|line| serde_json::from_str(line).expect("line should be valid json"))
        .collect()
}

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

#[test]
fn app_aicore_uses_terminal_panel_in_rich_mode() {
    let output = run_aicore_with_env(&[("AICORE_TERMINAL", "rich")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("╭─ AICore OS"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(stdout.contains("Runtime：global-main/main"));
}

#[test]
fn app_aicore_plain_has_no_ansi() {
    let output = run_aicore_with_env(&[("AICORE_TERMINAL", "plain")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("AICore OS"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn app_aicore_json_outputs_valid_json() {
    let output = run_aicore_with_env(&[("AICORE_TERMINAL", "json")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    let events = assert_json_lines(&stdout);
    assert!(
        events
            .iter()
            .any(|event| event.get("event").and_then(|value| value.as_str())
                == Some("block.panel"))
    );
    assert!(stdout.contains("global-main"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn app_aicore_no_color_has_no_ansi() {
    let output = run_aicore_with_env(&[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("AICore OS"));
    assert!(!stdout.contains("\u{1b}["));
}
