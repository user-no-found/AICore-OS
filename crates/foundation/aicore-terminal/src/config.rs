use std::collections::BTreeMap;

use serde::Serialize;

use crate::capabilities::TerminalCapabilities;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalEnv {
    values: BTreeMap<String, String>,
}

impl TerminalEnv {
    pub fn current() -> Self {
        Self {
            values: std::env::vars().collect(),
        }
    }

    pub fn from_pairs<const N: usize>(pairs: [(&str, &str); N]) -> Self {
        Self {
            values: pairs
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn is_truthy(&self, key: &str) -> bool {
        matches!(self.get(key), Some("1" | "true" | "TRUE" | "yes" | "YES"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalMode {
    Rich,
    Plain,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogoMode {
    Compact,
    Full,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolMode {
    Unicode,
    Ascii,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalConfig {
    pub mode: TerminalMode,
    pub color: ColorMode,
    pub logo: LogoMode,
    pub symbols: SymbolMode,
    pub progress: ProgressMode,
    pub verbose: bool,
    pub deny_warnings: bool,
}

impl TerminalConfig {
    pub fn current() -> Self {
        Self::from_env_and_capabilities(&TerminalEnv::current(), TerminalCapabilities::stdout())
    }

    pub fn from_env_and_capabilities(
        env: &TerminalEnv,
        capabilities: TerminalCapabilities,
    ) -> Self {
        let ci = env.is_truthy("CI");
        let requested_mode = env.get("AICORE_TERMINAL").unwrap_or("auto");
        let mode = match requested_mode {
            "rich" => TerminalMode::Rich,
            "plain" => TerminalMode::Plain,
            "json" => TerminalMode::Json,
            _ if capabilities.is_tty && !ci => TerminalMode::Rich,
            _ => TerminalMode::Plain,
        };

        let color = if mode == TerminalMode::Json || env.is_truthy("NO_COLOR") {
            ColorMode::Never
        } else {
            match env.get("AICORE_COLOR").unwrap_or("auto") {
                "always" => ColorMode::Always,
                "never" => ColorMode::Never,
                _ => ColorMode::Auto,
            }
        };

        let logo = if mode == TerminalMode::Json {
            LogoMode::Off
        } else {
            match env.get("AICORE_LOGO").unwrap_or("compact") {
                "full" => LogoMode::Full,
                "off" => LogoMode::Off,
                _ => LogoMode::Compact,
            }
        };

        let symbols = match env.get("AICORE_SYMBOLS") {
            Some("unicode") => SymbolMode::Unicode,
            Some("ascii") => SymbolMode::Ascii,
            _ if mode == TerminalMode::Rich && !ci => SymbolMode::Unicode,
            _ => SymbolMode::Ascii,
        };

        let progress = if mode == TerminalMode::Json || env.is_truthy("AICORE_VERBOSE") {
            ProgressMode::Never
        } else {
            match env.get("AICORE_PROGRESS").unwrap_or("auto") {
                "always" => ProgressMode::Always,
                "never" => ProgressMode::Never,
                _ => ProgressMode::Auto,
            }
        };

        Self {
            mode,
            color,
            logo,
            symbols,
            progress,
            verbose: env.is_truthy("AICORE_VERBOSE"),
            deny_warnings: env.is_truthy("AICORE_WORKFLOW_DENY_WARNINGS"),
        }
    }

    pub fn use_ansi(&self) -> bool {
        match self.color {
            ColorMode::Always => self.mode != TerminalMode::Json,
            ColorMode::Never => false,
            ColorMode::Auto => self.mode == TerminalMode::Rich,
        }
    }

    pub fn rich_for_tests() -> Self {
        Self {
            mode: TerminalMode::Rich,
            color: ColorMode::Never,
            logo: LogoMode::Compact,
            symbols: SymbolMode::Unicode,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }

    pub fn plain_for_tests() -> Self {
        Self {
            mode: TerminalMode::Plain,
            color: ColorMode::Never,
            logo: LogoMode::Off,
            symbols: SymbolMode::Ascii,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }

    pub fn json_for_tests() -> Self {
        Self {
            mode: TerminalMode::Json,
            color: ColorMode::Never,
            logo: LogoMode::Off,
            symbols: SymbolMode::Ascii,
            progress: ProgressMode::Never,
            verbose: false,
            deny_warnings: false,
        }
    }
}
