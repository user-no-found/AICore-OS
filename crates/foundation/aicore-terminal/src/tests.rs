use crate::*;

#[test]
fn terminal_mode_auto_detects_plain_under_ci() {
    let env = TerminalEnv::from_pairs([("CI", "1")]);
    let config =
        TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: true });

    assert_eq!(config.mode, TerminalMode::Plain);
}

#[test]
fn no_color_disables_ansi() {
    let env = TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")]);
    let config =
        TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: true });

    assert_eq!(config.color, ColorMode::Never);
    assert!(!config.use_ansi());
}

#[test]
fn color_always_emits_ansi_in_rich_mode() {
    let env = TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("AICORE_COLOR", "always")]);
    let config =
        TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: false });
    let document = Document::new(vec![Block::step_finished(StepSummary::new(
        "cargo test",
        Status::Ok,
        0,
    ))]);

    let output = render_document(&document, &config);

    assert!(output.contains("\u{1b}[32m"));
}

#[test]
fn unicode_and_ascii_symbols_render() {
    assert_eq!(StatusSymbols::unicode().ok, "✓");
    assert_eq!(StatusSymbols::ascii().ok, "[OK]");
}

#[test]
fn unicode_running_symbol_is_not_hourglass_emoji() {
    assert_ne!(StatusSymbols::unicode().running, "⏳");
}

#[test]
fn status_color_does_not_paint_entire_step_line() {
    let env = TerminalEnv::from_pairs([("AICORE_TERMINAL", "rich"), ("AICORE_COLOR", "always")]);
    let config =
        TerminalConfig::from_env_and_capabilities(&env, TerminalCapabilities { is_tty: true });
    let document = Document::new(vec![Block::step_finished(StepSummary::new(
        "cargo test",
        Status::Ok,
        0,
    ))]);

    let output = render_document(&document, &config);

    assert!(output.contains("\u{1b}[32m✓ OK\u{1b}[0m"));
    assert!(output.contains("cargo test"));
    assert!(!output.contains("\u{1b}[32m✓ cargo test | Warnings 0\u{1b}[0m"));
}

#[test]
fn panel_renderer_supports_rich_and_plain() {
    let document = Document::new(vec![Block::panel("AICore OS", "Composable")]);

    let rich = render_document(&document, &TerminalConfig::rich_for_tests());
    let plain = render_document(&document, &TerminalConfig::plain_for_tests());

    assert!(rich.contains("╭─ AICore OS"));
    assert!(plain.contains("AICore OS"));
    assert!(!plain.contains('╭'));
}

#[test]
fn rich_panel_wraps_long_body_lines() {
    let document = Document::new(vec![Block::panel(
        "Warnings",
        "fix: echo 'export PATH=\"$HOME/.aicore/bin:$PATH\"' >> ~/.bashrc && then restart the shell before running aicore-cli again",
    )]);

    let rich = render_document(&document, &TerminalConfig::rich_for_tests());
    let widths = rich
        .lines()
        .map(terminal_width_for_test)
        .collect::<Vec<_>>();

    assert!(
        widths.iter().all(|width| *width <= 82),
        "{widths:?}\n{rich}"
    );
    assert!(rich.lines().count() > 3, "long body should wrap\n{rich}");
}

fn terminal_width_for_test(value: &str) -> usize {
    value
        .chars()
        .map(|ch| match ch {
            '╭' | '╮' | '╰' | '╯' | '│' | '─' => 1,
            _ if ch as u32 >= 0x1100 => 2,
            _ => 1,
        })
        .sum()
}

#[test]
fn table_renderer_handles_mixed_chinese_and_english_width() {
    let document = Document::new(vec![Block::table(
        vec!["名称", "Status"],
        vec![vec!["内核", "OK"], vec!["provider", "WARN"]],
    )]);

    let output = render_document(&document, &TerminalConfig::plain_for_tests());

    assert!(output.contains("名称"));
    assert!(output.contains("provider"));
    assert!(output.contains("WARN"));
}

#[test]
fn json_renderer_pretty_prints_json_and_reports_invalid_json() {
    let valid = Document::new(vec![Block::json(r#"{"b":1,"a":true}"#)]);
    let invalid = Document::new(vec![Block::json("{bad json")]);

    let valid_output = render_document(&valid, &TerminalConfig::plain_for_tests());
    let invalid_output = render_document(&invalid, &TerminalConfig::plain_for_tests());

    assert!(valid_output.contains("\n  \"a\": true"));
    assert!(invalid_output.contains("无效 JSON"));
}

#[test]
fn toml_and_markdown_blocks_preserve_content() {
    let document = Document::new(vec![
        Block::toml("[section]\nkey = \"value\""),
        Block::markdown("# 标题\n\n```bash\ncargo core\n```"),
    ]);

    let output = render_document(&document, &TerminalConfig::plain_for_tests());

    assert!(output.contains("[section]"));
    assert!(output.contains("```bash"));
}

#[test]
fn diagnostic_and_warning_summary_render() {
    let warning =
        WarningDiagnostic::new("cargo test", "unused variable").with_location("src/lib.rs", 10, 5);
    let document = Document::new(vec![
        Block::diagnostic(Diagnostic::warning("W0001", "warning message")),
        Block::warning_summary(vec![warning.clone()], 1),
        Block::final_summary(RunSummary::new("kernel", Status::Warn, 2, 1)),
    ]);

    let output = render_document(&document, &TerminalConfig::plain_for_tests());

    assert!(output.contains("warning message"));
    assert!(output.contains("unused variable"));
    assert!(output.contains("Warnings 1"));
}

#[test]
fn sanitizer_and_redaction_strip_unsafe_output() {
    let unsafe_text = "token sk-test-secret \u{1b}[31mred\u{7}";
    let document = Document::new(vec![Block::text(unsafe_text)]);

    let output = render_document(&document, &TerminalConfig::plain_for_tests());

    assert!(!output.contains("sk-test-secret"));
    assert!(!output.contains("\u{1b}"));
    assert!(!output.contains('\u{7}'));
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn json_mode_emits_json_lines_events() {
    let document = Document::new(vec![
        Block::run_started("kernel"),
        Block::run_finished(RunSummary::new("kernel", Status::Ok, 2, 0)),
    ]);

    let output = render_document(&document, &TerminalConfig::json_for_tests());

    for line in output.lines() {
        let value: serde_json::Value = serde_json::from_str(line).expect("valid json line");
        assert_eq!(value["schema"], "aicore.terminal.v1");
    }
}

#[test]
fn structured_json_block_outputs_named_json_event() {
    let document = Document::new(vec![Block::structured_json(
        "kernel.invocation.result",
        r#"{"result":{"fields":{"manifest_count":"3"}}}"#,
    )]);

    let output = render_document(&document, &TerminalConfig::json_for_tests());
    let value: serde_json::Value =
        serde_json::from_str(output.trim()).expect("valid structured json line");

    assert_eq!(value["schema"], "aicore.terminal.v1");
    assert_eq!(value["event"], "kernel.invocation.result");
    assert_eq!(value["payload"]["result"]["fields"]["manifest_count"], "3");
    assert!(!output.contains("block.panel"));
}

#[test]
fn structured_json_block_redacts_secret_like_values() {
    let document = Document::new(vec![Block::structured_json(
        "kernel.invocation.result",
        r#"{"result":{"fields":{"secret_ref":"sk-test-secret"}}}"#,
    )]);

    let output = render_document(&document, &TerminalConfig::json_for_tests());

    assert!(!output.contains("sk-test-secret"));
    assert!(!output.contains("secret_ref"));
    assert!(output.contains("[REDACTED]"));
}

#[test]
fn json_mode_redacts_warning_payload() {
    let warning = WarningDiagnostic::new("cargo test", "leaked sk-test-secret");
    let document = Document::new(vec![Block::warning(warning)]);

    let output = render_document(&document, &TerminalConfig::json_for_tests());

    assert!(!output.contains("sk-test-secret"));
    assert!(output.contains("[REDACTED]"));
}
