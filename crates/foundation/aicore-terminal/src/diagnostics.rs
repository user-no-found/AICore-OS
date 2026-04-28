use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: Option<String>,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn warning(code: &str, message: &str) -> Self {
        Self {
            severity: Severity::Warning,
            code: Some(code.to_string()),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            help: None,
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            severity: Severity::Error,
            code: Some(code.to_string()),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            help: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WarningSource {
    CargoDiagnostic,
    RustcRendered,
    Rustdoc,
    BuildScript,
    TextScanner,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WarningDiagnostic {
    pub step: String,
    pub message: String,
    pub path: Option<String>,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub source: WarningSource,
    pub raw_lines: Vec<String>,
}

impl WarningDiagnostic {
    pub fn new(step: &str, message: &str) -> Self {
        Self {
            step: step.to_string(),
            message: message.to_string(),
            path: None,
            line: None,
            column: None,
            source: WarningSource::TextScanner,
            raw_lines: vec![message.to_string()],
        }
    }

    pub fn with_source(mut self, source: WarningSource) -> Self {
        self.source = source;
        self
    }

    pub fn with_location(mut self, path: &str, line: u32, column: u32) -> Self {
        self.path = Some(path.to_string());
        self.line = Some(line);
        self.column = Some(column);
        self
    }

    pub fn fingerprint(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}",
            self.step,
            self.path.as_deref().unwrap_or("-"),
            self.line
                .map(|line| line.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.column
                .map(|column| column.to_string())
                .unwrap_or_else(|| "-".to_string()),
            normalize_warning_message(&self.message)
        )
    }
}

fn normalize_warning_message(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
