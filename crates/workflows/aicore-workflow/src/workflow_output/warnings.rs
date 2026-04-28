use aicore_terminal::{
    Block, Document, TerminalConfig, TerminalMode, WarningDiagnostic, render_document, safe_text,
};

use super::format::{accent, label_style, warning};
use super::panels::render_panel;

pub fn render_warnings(warnings: Vec<WarningDiagnostic>, config: &TerminalConfig) -> String {
    if warnings.is_empty() {
        return String::new();
    }

    if config.mode == TerminalMode::Json {
        let blocks = warnings
            .into_iter()
            .take(20)
            .map(|warning| Block::warning(warning_for_json(&warning)))
            .collect::<Vec<_>>();
        return render_document(&Document::new(blocks), config);
    }

    let mut lines = vec![warning_summary_count_line(warnings.len(), config)];
    for (index, warning) in warnings.iter().take(20).enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.extend(render_warning_block(index + 1, warning, config));
    }
    if warnings.len() > 20 {
        lines.push(format!("... 还有 {} 条 warning", warnings.len() - 20));
    }
    render_panel("Warnings", &lines.join("\n"), config)
}

pub fn warning_for_json(warning: &WarningDiagnostic) -> WarningDiagnostic {
    let surface = parse_warning_surface(warning);
    let mut raw_lines = vec![format!("message: {}", surface.message)];
    if !surface.paths.is_empty() {
        raw_lines.push("paths:".to_string());
        raw_lines.extend(surface.paths.iter().map(|path| format!("- {path}")));
    }
    if let Some(current) = &surface.current {
        raw_lines.push(format!("current: {current}"));
    }
    if let Some(expected) = &surface.expected {
        raw_lines.push(format!("expected: {expected}"));
    }
    if let Some(fix) = &surface.fix {
        raw_lines.push(format!("fix: {fix}"));
    }
    if let Some(persist) = &surface.persist {
        raw_lines.push(format!("persist: {persist}"));
    }
    raw_lines.extend(
        surface
            .details
            .iter()
            .map(|detail| format!("detail: {detail}")),
    );

    WarningDiagnostic {
        step: warning.step.clone(),
        message: surface.message,
        path: warning.path.clone(),
        line: warning.line,
        column: warning.column,
        source: warning.source.clone(),
        raw_lines,
    }
}

fn warning_summary_count_line(count: usize, config: &TerminalConfig) -> String {
    if config.mode == TerminalMode::Rich {
        render_warning_field("Warnings", &count.to_string(), config)
    } else {
        format!("Warnings: {count} scanned this run")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WarningSurface {
    message: String,
    paths: Vec<String>,
    current: Option<String>,
    expected: Option<String>,
    fix: Option<String>,
    persist: Option<String>,
    details: Vec<String>,
}

fn render_warning_block(
    index: usize,
    warning: &WarningDiagnostic,
    config: &TerminalConfig,
) -> Vec<String> {
    let surface = parse_warning_surface(warning);
    if config.mode == TerminalMode::Rich {
        let mut lines = vec![
            accent(&format!("#{index} {}", safe_text(&warning.step)), config),
            render_warning_field("Level", &warning_level_text(config), config),
            render_warning_field("Message", &surface.message, config),
        ];
        if !surface.paths.is_empty() {
            lines.push(render_warning_field("Paths", "", config));
            lines.extend(
                surface
                    .paths
                    .iter()
                    .map(|path| format!("  - {}", safe_text(path))),
            );
        }
        if let Some(current) = surface.current {
            lines.push(render_warning_field("Current", &current, config));
        }
        if let Some(expected) = surface.expected {
            lines.push(render_warning_field("Expected", &expected, config));
        }
        if let Some(fix) = surface.fix {
            lines.push(render_warning_field("Fix", &fix, config));
        }
        if let Some(persist) = surface.persist {
            lines.push(render_warning_field("Persist", "", config));
            lines.extend(
                split_persist_command(&persist)
                    .into_iter()
                    .map(|line| format!("  {}", safe_text(&line))),
            );
        }
        for detail in surface.details {
            lines.push(render_warning_field("Detail", &detail, config));
        }
        return lines;
    }

    let mut lines = vec![
        format!("[WARN] {}", safe_text(&warning.step)),
        format!("message: {}", safe_text(&surface.message)),
    ];
    if !surface.paths.is_empty() {
        lines.push("paths:".to_string());
        lines.extend(
            surface
                .paths
                .iter()
                .map(|path| format!("- {}", safe_text(path))),
        );
    }
    if let Some(current) = surface.current {
        lines.push(format!("current: {}", safe_text(&current)));
    }
    if let Some(expected) = surface.expected {
        lines.push(format!("expected: {}", safe_text(&expected)));
    }
    if let Some(fix) = surface.fix {
        lines.push(format!("fix: {}", safe_text(&fix)));
    }
    if let Some(persist) = surface.persist {
        lines.push(format!("persist: {}", safe_text(&persist)));
    }
    for detail in surface.details {
        lines.push(format!("detail: {}", safe_text(&detail)));
    }
    lines
}

fn render_warning_field(key: &str, value: &str, config: &TerminalConfig) -> String {
    let label = format!("{:<8}", safe_text(key));
    if value.is_empty() {
        if config.mode == TerminalMode::Rich {
            format!("{} :", label_style(&label, config))
        } else {
            format!("{} :", label.trim_end())
        }
    } else if config.mode == TerminalMode::Rich {
        format!("{} : {}", label_style(&label, config), value)
    } else {
        format!("{} : {}", label.trim_end(), safe_text(value))
    }
}

fn warning_level_text(config: &TerminalConfig) -> String {
    if config.mode == TerminalMode::Rich {
        warning("! WARN", config)
    } else {
        "[WARN]".to_string()
    }
}

fn split_persist_command(value: &str) -> Vec<String> {
    value
        .split_once(" >> ")
        .map(|(command, target)| vec![command.to_string(), format!(">> {target}")])
        .unwrap_or_else(|| vec![value.to_string()])
}

fn parse_warning_surface(warning: &WarningDiagnostic) -> WarningSurface {
    let lines = warning
        .message
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let first = lines.first().copied().unwrap_or("");

    if first.contains("~/.aicore/bin 当前不在 PATH") {
        let mut paths = Vec::new();
        let mut fix = None;
        let mut persist = None;
        let mut details = Vec::new();
        for line in lines.iter().skip(1) {
            if let Some(path) = line.strip_prefix("- ") {
                paths.push(path.to_string());
            } else if let Some(value) = line.strip_prefix("临时生效命令：") {
                fix = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("重新加载命令：") {
                fix = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("建议加入 shell rc：") {
                persist = Some(value.to_string());
            } else if !line.ends_with('：') {
                details.push((*line).to_string());
            }
        }
        return WarningSurface {
            message: first.trim_end_matches('。').to_string() + "。",
            paths,
            current: None,
            expected: None,
            fix,
            persist,
            details,
        };
    }

    if first.contains("检测到命令 shadowing") {
        let mut current = None;
        let mut expected = None;
        let mut fix = None;
        let mut details = Vec::new();
        for line in lines.iter().skip(1) {
            if line.contains("指向") {
                current = backtick_values(line).get(1).cloned();
            } else if line.contains("位于") {
                expected = backtick_values(line).first().cloned();
            } else if line.starts_with("请将") {
                fix = Some("将 $HOME/.aicore/bin 放到 PATH 前面".to_string());
            } else {
                details.push((*line).to_string());
            }
        }
        return WarningSurface {
            message: "检测到命令 shadowing".to_string(),
            paths: Vec::new(),
            current,
            expected,
            fix,
            persist: None,
            details,
        };
    }

    WarningSurface {
        message: first.to_string(),
        paths: Vec::new(),
        current: warning.path.clone(),
        expected: None,
        fix: None,
        persist: None,
        details: lines
            .iter()
            .skip(1)
            .map(|line| (*line).to_string())
            .collect(),
    }
}

fn backtick_values(line: &str) -> Vec<String> {
    line.split('`')
        .enumerate()
        .filter_map(|(index, part)| {
            if index % 2 == 1 {
                Some(part.to_string())
            } else {
                None
            }
        })
        .collect()
}
