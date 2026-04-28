use serde_json::json;

use crate::diagnostics::{Diagnostic, WarningDiagnostic};
use crate::document::{Block, Document};
use crate::redaction::safe_text;
use crate::summary::{RunSummary, StepSummary};

pub fn render_json_lines(document: &Document) -> String {
    let mut lines = Vec::new();
    for block in &document.blocks {
        match block {
            Block::RunStarted(name) => {
                lines.push(json_event("run.started", json!({ "name": name })))
            }
            Block::StepStarted(name) => {
                lines.push(json_event("step.started", json!({ "name": name })))
            }
            Block::StepFinished(summary) => lines.push(json_event(
                "step.finished",
                json!({ "summary": safe_step_summary(summary) }),
            )),
            Block::Warning(warning) => lines.push(json_event(
                "warning",
                json!({ "warning": safe_warning_json(warning) }),
            )),
            Block::WarningSummary { warnings, .. } => {
                for warning in warnings {
                    lines.push(json_event(
                        "warning",
                        json!({ "warning": safe_warning_json(warning) }),
                    ));
                }
            }
            Block::RunFinished(summary) | Block::FinalSummary(summary) => lines.push(json_event(
                "run.finished",
                json!({ "summary": safe_run_summary(summary) }),
            )),
            Block::Logo => lines.push(json_event("block.logo", json!({ "enabled": false }))),
            Block::Panel { title, body } => lines.push(json_event(
                "block.panel",
                json!({ "title": safe_text(title), "body": safe_text(body) }),
            )),
            Block::KeyValue(rows) => lines.push(json_event(
                "block.key_value",
                json!({ "rows": safe_rows(rows) }),
            )),
            Block::Table { headers, rows } => lines.push(json_event(
                "block.table",
                json!({ "headers": safe_string_vec(headers), "rows": safe_table_rows(rows) }),
            )),
            Block::Diagnostic(diagnostic) => lines.push(json_event(
                "diagnostic",
                json!({ "diagnostic": safe_diagnostic_json(diagnostic) }),
            )),
            Block::Markdown(markdown) => lines.push(json_event(
                "block.markdown",
                json!({ "markdown": safe_text(markdown) }),
            )),
            Block::Json(source) => lines.push(json_event(
                "block.json",
                json!({ "json": safe_text(source) }),
            )),
            Block::StructuredJson { event, payload } => lines.push(json_event(
                event,
                serde_json::from_str(payload)
                    .map(safe_json_value)
                    .unwrap_or_else(|_| json!({ "raw": safe_text(payload) })),
            )),
            Block::Toml(source) => lines.push(json_event(
                "block.toml",
                json!({ "toml": safe_text(source) }),
            )),
            Block::Text(text) => {
                lines.push(json_event("block.text", json!({ "text": safe_text(text) })))
            }
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn json_event(kind: &str, payload: serde_json::Value) -> String {
    serde_json::to_string(&json!({
        "schema": "aicore.terminal.v1",
        "event": kind,
        "payload": payload,
    }))
    .unwrap_or_else(|_| "{\"schema\":\"aicore.terminal.v1\",\"event\":\"error\"}".to_string())
}

fn safe_json_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::String(value) => json!(safe_text(&value)),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(safe_json_value).collect())
        }
        serde_json::Value::Object(values) => serde_json::Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (safe_text(&key), safe_json_value(value)))
                .collect(),
        ),
        value => value,
    }
}

fn safe_string_vec(values: &[String]) -> Vec<String> {
    values.iter().map(|value| safe_text(value)).collect()
}

fn safe_rows(rows: &[(String, String)]) -> Vec<(String, String)> {
    rows.iter()
        .map(|(key, value)| (safe_text(key), safe_text(value)))
        .collect()
}

fn safe_table_rows(rows: &[Vec<String>]) -> Vec<Vec<String>> {
    rows.iter().map(|row| safe_string_vec(row)).collect()
}

fn safe_diagnostic_json(diagnostic: &Diagnostic) -> serde_json::Value {
    json!({
        "severity": diagnostic.severity,
        "code": diagnostic.code.as_ref().map(|value| safe_text(value)),
        "message": safe_text(&diagnostic.message),
        "path": diagnostic.path.as_ref().map(|value| safe_text(value)),
        "line": diagnostic.line,
        "column": diagnostic.column,
        "help": diagnostic.help.as_ref().map(|value| safe_text(value)),
    })
}

fn safe_warning_json(warning: &WarningDiagnostic) -> serde_json::Value {
    json!({
        "step": safe_text(&warning.step),
        "message": safe_text(&warning.message),
        "path": warning.path.as_ref().map(|value| safe_text(value)),
        "line": warning.line,
        "column": warning.column,
        "source": warning.source,
        "raw_lines": warning.raw_lines.iter().map(|value| safe_text(value)).collect::<Vec<_>>(),
    })
}

fn safe_step_summary(summary: &StepSummary) -> serde_json::Value {
    json!({
        "name": safe_text(&summary.name),
        "status": summary.status,
        "warning_count": summary.warning_count,
    })
}

fn safe_run_summary(summary: &RunSummary) -> serde_json::Value {
    json!({
        "name": safe_text(&summary.name),
        "status": summary.status,
        "step_count": summary.step_count,
        "warning_count": summary.warning_count,
    })
}
