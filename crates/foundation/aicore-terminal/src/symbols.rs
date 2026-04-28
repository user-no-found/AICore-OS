use serde::Serialize;

use crate::config::{SymbolMode, TerminalConfig};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSymbols {
    pub ok: &'static str,
    pub warn: &'static str,
    pub failed: &'static str,
    pub running: &'static str,
    pub info: &'static str,
    pub skipped: &'static str,
}

impl StatusSymbols {
    pub fn unicode() -> Self {
        Self {
            ok: "✓",
            warn: "⚠",
            failed: "✗",
            running: "[RUNNING]",
            info: "•",
            skipped: "–",
        }
    }

    pub fn ascii() -> Self {
        Self {
            ok: "[OK]",
            warn: "[WARN]",
            failed: "[FAILED]",
            running: "[RUNNING]",
            info: "[INFO]",
            skipped: "[SKIPPED]",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    Warn,
    Failed,
    Running,
    Info,
    Skipped,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Failed => "FAILED",
            Self::Running => "RUNNING",
            Self::Info => "INFO",
            Self::Skipped => "SKIPPED",
        }
    }
}

pub(crate) fn symbols_for(config: &TerminalConfig) -> StatusSymbols {
    match config.symbols {
        SymbolMode::Unicode => StatusSymbols::unicode(),
        SymbolMode::Ascii => StatusSymbols::ascii(),
    }
}
