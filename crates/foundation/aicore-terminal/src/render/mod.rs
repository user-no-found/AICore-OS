mod human;
mod json;

use crate::config::{TerminalConfig, TerminalMode};
use crate::document::Document;

pub fn render_document(document: &Document, config: &TerminalConfig) -> String {
    match config.mode {
        TerminalMode::Json => json::render_json_lines(document),
        TerminalMode::Rich => human::render_human(document, config, true),
        TerminalMode::Plain => human::render_human(document, config, false),
    }
}
