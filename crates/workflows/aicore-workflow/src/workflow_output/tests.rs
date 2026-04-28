use std::path::PathBuf;

use aicore_terminal::{
    Status, TerminalCapabilities, TerminalConfig, TerminalEnv, WarningDiagnostic,
};

use crate::cargo_runner::CommandReport;
use crate::shell_integration::{ShellPathBootstrapResult, ShellPathBootstrapStatus};

use super::warnings::warning_for_json;
use super::*;

#[test]
fn workflow_report_includes_logo_header_in_rich_mode() {
    let output = render_run_started_for_tests("kernel", &TerminalConfig::rich_for_tests());

    assert!(output.contains("AICore OS"));
    assert!(output.contains("Workflow"));
    assert!(output.contains("Mode"));
    assert!(output.contains("Root"));
    assert!(output.contains("Target"));
    assert!(output.contains("Warnings"));
    assert!(output.contains('╭'));
    assert!(output.contains('╰'));
}

#[test]
fn workflow_report_includes_step_cards_and_final_summary() {
    let output = render_finished_for_tests(
        "kernel",
        Status::Ok,
        2,
        0,
        &TerminalConfig::plain_for_tests(),
    );

    assert!(output.contains("Summary"));
    assert!(output.contains("Workflow"));
    assert!(output.contains("Status"));
    assert!(output.contains("Steps"));
    assert!(output.contains("Warnings  0 scanned this run"));
}

#[test]
fn workflow_rich_summary_renders_panel() {
    let output =
        render_finished_for_tests("core", Status::Ok, 8, 0, &TerminalConfig::rich_for_tests());

    assert!(output.contains("Summary"));
    assert!(output.contains("Workflow"));
    assert!(output.contains("core"));
    assert!(output.contains("✓ OK"));
    assert!(output.contains("8 total"));
    assert!(output.contains("0 scanned this run"));
}

#[test]
fn workflow_rich_output_renders_step_table() {
    let output = render_workflow_steps_for_tests(&TerminalConfig::rich_for_tests());

    assert!(output.contains("Workflow Steps"));
    assert!(output.contains("foundation"));
    assert!(output.contains("kernel"));
    assert!(output.contains("fmt"));
    assert!(output.contains("install"));
}

#[test]
fn workflow_plain_output_renders_step_table() {
    let output = render_workflow_steps_for_tests(&TerminalConfig::plain_for_tests());

    assert!(output.contains("Workflow Steps"));
    assert!(output.contains("#  Layer"));
    assert!(output.contains("foundation"));
}

#[test]
fn workflow_warning_summary_keeps_structured_fields() {
    let warnings = vec![WarningDiagnostic::new(
        "install",
        "~/.aicore/bin 当前不在 PATH。\n当前安装的二进制路径：\n- /home/demo/.aicore/bin/aicore\n重新加载命令：source ~/.bashrc && hash -r",
    )];

    let output = render_warnings_for_tests(warnings, &TerminalConfig::plain_for_tests());

    assert!(output.contains("Warnings"));
    assert!(output.contains("message: ~/.aicore/bin 当前不在 PATH。"));
    assert!(output.contains("- /home/demo/.aicore/bin/aicore"));
    assert!(output.contains("fix: source ~/.bashrc && hash -r"));
}

#[test]
fn workflow_warning_summary_limits_output() {
    let warnings = (0..25)
        .map(|index| WarningDiagnostic::new("test", &format!("warning {index}")))
        .collect::<Vec<_>>();

    let output = render_warnings_for_tests(warnings, &TerminalConfig::plain_for_tests());

    assert!(output.contains("Warnings"));
    assert!(output.contains("warning 0"));
    assert!(output.contains("warning 19"));
    assert!(output.contains("还有 5 条 warning"));
}

#[test]
fn workflow_command_report_is_suppressed_without_verbose_or_force() {
    let report = CommandReport {
        command: "cargo test".to_string(),
        stdout: "ok".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        duration: std::time::Duration::from_millis(50),
        warnings: Vec::new(),
    };

    let output =
        render_command_report_for_tests(&report, false, false, &TerminalConfig::plain_for_tests());
    assert!(output.is_empty());
}

#[test]
fn workflow_command_report_renders_raw_output_when_forced() {
    let report = CommandReport {
        command: "cargo test".to_string(),
        stdout: "ok".to_string(),
        stderr: "warn".to_string(),
        exit_code: Some(1),
        duration: std::time::Duration::from_millis(50),
        warnings: Vec::new(),
    };

    let output =
        render_command_report_for_tests(&report, false, true, &TerminalConfig::plain_for_tests());
    assert!(output.contains("cargo test"));
    assert!(output.contains("stdout:"));
    assert!(output.contains("stderr:"));
}

#[test]
fn workflow_json_mode_emits_run_and_step_events() {
    let config = TerminalConfig::json_for_tests();
    let run_started = render_run_started_for_tests("kernel", &config);
    let finished = render_finished_for_tests("kernel", Status::Ok, 2, 0, &config);

    assert!(run_started.contains("\"event\":\"run.started\""));
    assert!(finished.contains("\"event\":\"run.finished\""));
}

#[test]
fn workflow_json_warning_surface_flattens_structured_warning() {
    let warning = WarningDiagnostic::new(
        "install",
        "检测到命令 shadowing：\n当前 shell 的 `aicore` 指向 `/usr/bin/aicore`。\n新安装的 AICore OS 位于 `/home/demo/.aicore/bin/aicore`。\n请将 `$HOME/.aicore/bin` 放到 PATH 前面，或清理旧的 `/usr/bin/aicore`。",
    );

    let json_warning = warning_for_json(&warning);

    assert_eq!(json_warning.message, "检测到命令 shadowing");
    assert!(
        json_warning
            .raw_lines
            .iter()
            .any(|line| line.contains("current: /usr/bin/aicore"))
    );
    assert!(
        json_warning
            .raw_lines
            .iter()
            .any(|line| line.contains("expected: /home/demo/.aicore/bin/aicore"))
    );
}

#[test]
fn workflow_shell_bootstrap_panel_renders() {
    let result = ShellPathBootstrapResult {
        status: ShellPathBootstrapStatus::Updated,
        shell: "/bin/bash".to_string(),
        rc_file: Some(PathBuf::from("/home/demo/.bashrc")),
        bin_path: PathBuf::from("/home/demo/.aicore/bin"),
        action: "updated".to_string(),
        reload: Some("source ~/.bashrc".to_string()),
        rollback: None,
        message: Some("done".to_string()),
    };

    let output = render_shell_path_bootstrap(&result, &TerminalConfig::plain_for_tests());
    assert!(output.contains("Shell PATH Bootstrap"));
    assert!(output.contains("/home/demo/.bashrc"));
    assert!(output.contains("source ~/.bashrc"));
}

#[test]
fn workflow_deny_warnings_error_is_stable() {
    assert!(deny_warnings_error(0).is_none());
    let error = deny_warnings_error(3).expect("deny warning should error");
    assert!(error.contains("AICORE_WORKFLOW_DENY_WARNINGS=1"));
    assert!(error.contains("Warnings 3"));
}

#[test]
fn workflow_output_uses_terminal_config_current_for_factory() {
    let env = TerminalEnv::from_pairs([("CI", "1"), ("AICORE_TERMINAL", "plain")]);
    let config =
        TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: false });
    assert_eq!(config.mode, aicore_terminal::TerminalMode::Plain);
}
