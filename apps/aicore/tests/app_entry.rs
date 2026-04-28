use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

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

#[test]
fn aicore_entry_reports_global_runtime_status() {
    let home = temp_home("runtime-status");
    create_runtime_status_fixture(&home);
    let output = run_aicore_with_env(&[
        ("HOME", home.to_str().expect("utf8 home")),
        ("PATH", "/usr/bin:/bin"),
        ("AICORE_TERMINAL", "plain"),
    ]);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid utf-8");
    assert!(stdout.contains("global root"));
    assert!(stdout.contains(home.join(".aicore").display().to_string().as_str()));
    assert!(stdout.contains("foundation installed"));
    assert!(stdout.contains("yes"));
    assert!(stdout.contains("kernel installed"));
    assert!(stdout.contains("contract version"));
    assert!(stdout.contains("kernel.runtime.v1"));
    assert!(stdout.contains("manifest count"));
    assert!(stdout.contains("capability count"));
    assert!(stdout.contains("event ledger"));
    assert!(stdout.contains("bin path status"));
}

fn create_runtime_status_fixture(home: &std::path::Path) {
    let foundation = home.join(".aicore/runtime/foundation");
    let kernel = home.join(".aicore/runtime/kernel");
    let manifests = home.join(".aicore/share/manifests");
    let kernel_state = home.join(".aicore/state/kernel");
    std::fs::create_dir_all(&foundation).expect("foundation runtime dir");
    std::fs::create_dir_all(&kernel).expect("kernel runtime dir");
    std::fs::create_dir_all(&manifests).expect("manifests dir");
    std::fs::create_dir_all(&kernel_state).expect("kernel state dir");
    std::fs::write(foundation.join("install.toml"), "status = \"installed\"\n")
        .expect("foundation install metadata");
    std::fs::write(
        kernel.join("version.toml"),
        "runtime_version = \"0.1.0\"\ncontract_version = \"kernel.runtime.v1\"\n",
    )
    .expect("kernel version metadata");
    std::fs::write(kernel.join("install.toml"), "status = \"installed\"\n")
        .expect("kernel install metadata");
    std::fs::write(kernel.join("capabilities.toml"), "capability_count = 3\n")
        .expect("kernel capabilities metadata");
    std::fs::write(manifests.join("aicore.toml"), "component_id = \"aicore\"\n").expect("manifest");
}

fn temp_home(name: &str) -> std::path::PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("aicore-app-{name}-{}-{unique}", std::process::id()));
    std::fs::create_dir_all(&path).expect("create temp home");
    path
}
