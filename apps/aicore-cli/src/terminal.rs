use aicore_terminal::{Block, Document, TerminalConfig, render_document};

pub(crate) fn emit_cli_panel(title: &str, rows: Vec<(String, String)>) {
    let body = rows
        .into_iter()
        .map(|(key, value)| format!("{key}：{value}"))
        .collect::<Vec<_>>()
        .join("\n");
    emit_cli_panel_body(title, &body);
}

pub(crate) fn emit_cli_panel_body(title: &str, body: &str) {
    emit_document(Document::new(vec![Block::panel(title, body)]));
}

pub(crate) fn emit_document(document: Document) {
    print!("{}", render_document(&document, &TerminalConfig::current()));
}

pub(crate) fn cli_row(key: impl Into<String>, value: impl Into<String>) -> (String, String) {
    (key.into(), value.into())
}
