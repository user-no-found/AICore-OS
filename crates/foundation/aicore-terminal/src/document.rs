use crate::diagnostics::{Diagnostic, WarningDiagnostic};
use crate::summary::{RunSummary, StepSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Logo,
    Panel {
        title: String,
        body: String,
    },
    KeyValue(Vec<(String, String)>),
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    Diagnostic(Diagnostic),
    Markdown(String),
    Json(String),
    StructuredJson {
        event: String,
        payload: String,
    },
    Toml(String),
    Text(String),
    WarningSummary {
        warnings: Vec<WarningDiagnostic>,
        limit: usize,
    },
    FinalSummary(RunSummary),
    RunStarted(String),
    StepStarted(String),
    StepFinished(StepSummary),
    Warning(WarningDiagnostic),
    RunFinished(RunSummary),
}

impl Block {
    pub fn logo() -> Self {
        Self::Logo
    }

    pub fn panel(title: &str, body: &str) -> Self {
        Self::Panel {
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    pub fn key_value(rows: Vec<(&str, &str)>) -> Self {
        Self::KeyValue(
            rows.into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
    }

    pub fn table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> Self {
        Self::Table {
            headers: headers.into_iter().map(ToString::to_string).collect(),
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(ToString::to_string).collect())
                .collect(),
        }
    }

    pub fn diagnostic(diagnostic: Diagnostic) -> Self {
        Self::Diagnostic(diagnostic)
    }

    pub fn markdown(markdown: &str) -> Self {
        Self::Markdown(markdown.to_string())
    }

    pub fn json(json: &str) -> Self {
        Self::Json(json.to_string())
    }

    pub fn structured_json(event: &str, payload: &str) -> Self {
        Self::StructuredJson {
            event: event.to_string(),
            payload: payload.to_string(),
        }
    }

    pub fn toml(toml: &str) -> Self {
        Self::Toml(toml.to_string())
    }

    pub fn text(text: &str) -> Self {
        Self::Text(text.to_string())
    }

    pub fn warning_summary(warnings: Vec<WarningDiagnostic>, limit: usize) -> Self {
        Self::WarningSummary { warnings, limit }
    }

    pub fn final_summary(summary: RunSummary) -> Self {
        Self::FinalSummary(summary)
    }

    pub fn run_started(name: &str) -> Self {
        Self::RunStarted(name.to_string())
    }

    pub fn step_started(name: &str) -> Self {
        Self::StepStarted(name.to_string())
    }

    pub fn step_finished(summary: StepSummary) -> Self {
        Self::StepFinished(summary)
    }

    pub fn warning(warning: WarningDiagnostic) -> Self {
        Self::Warning(warning)
    }

    pub fn run_finished(summary: RunSummary) -> Self {
        Self::RunFinished(summary)
    }
}
