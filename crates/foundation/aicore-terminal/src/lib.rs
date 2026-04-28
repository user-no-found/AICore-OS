mod capabilities;
mod config;
mod diagnostics;
mod document;
mod redaction;
mod render;
mod sanitize;
mod summary;
mod symbols;
#[cfg(test)]
mod tests;
mod width;

pub use capabilities::TerminalCapabilities;
pub use config::{
    ColorMode, LogoMode, ProgressMode, SymbolMode, TerminalConfig, TerminalEnv, TerminalMode,
};
pub use diagnostics::{Diagnostic, Severity, WarningDiagnostic, WarningSource};
pub use document::{Block, Document};
pub use redaction::{redact_text, safe_text};
pub use render::render_document;
pub use sanitize::sanitize_text;
pub use summary::{RunSummary, StepSummary};
pub use symbols::{Status, StatusSymbols};
pub use width::display_width;
