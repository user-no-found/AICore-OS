use std::io::IsTerminal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    pub is_tty: bool,
}

impl TerminalCapabilities {
    pub fn stdout() -> Self {
        Self {
            is_tty: std::io::stdout().is_terminal(),
        }
    }
}
