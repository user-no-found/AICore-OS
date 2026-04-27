use std::collections::BTreeMap;

use aicore_terminal::{WarningDiagnostic, WarningSource};

pub fn parse_warnings(step: &str, stdout: &str, stderr: &str) -> Vec<WarningDiagnostic> {
    let mut warnings = Vec::new();
    let mut last_warning_index = None;
    let combined = format!("{stdout}\n{stderr}");

    for line in combined.lines() {
        let trimmed = line.trim_start();
        if let Some(message) = trimmed.strip_prefix("warning:") {
            warnings.push(
                WarningDiagnostic::new(step, message.trim())
                    .with_source(WarningSource::RustcRendered),
            );
            last_warning_index = Some(warnings.len() - 1);
            continue;
        }

        if let Some((path, line_number, column)) = parse_location_line(trimmed) {
            if let Some(index) = last_warning_index {
                warnings[index].path = Some(path);
                warnings[index].line = Some(line_number);
                warnings[index].column = Some(column);
                warnings[index].raw_lines.push(line.to_string());
            }
            continue;
        }

        if trimmed.contains("warning:") || trimmed.starts_with("warning[") {
            let message = trimmed
                .split_once("warning:")
                .map(|(_, message)| message.trim())
                .unwrap_or(trimmed);
            warnings.push(
                WarningDiagnostic::new(step, message).with_source(WarningSource::TextScanner),
            );
            last_warning_index = Some(warnings.len() - 1);
        }
    }

    dedupe_warnings(warnings)
}

fn parse_location_line(line: &str) -> Option<(String, u32, u32)> {
    let location = line.strip_prefix("-->")?.trim();
    let mut parts = location.rsplitn(3, ':');
    let column = parts.next()?.parse::<u32>().ok()?;
    let line_number = parts.next()?.parse::<u32>().ok()?;
    let path = parts.next()?.trim().to_string();
    Some((path, line_number, column))
}

fn dedupe_warnings(warnings: Vec<WarningDiagnostic>) -> Vec<WarningDiagnostic> {
    let mut by_fingerprint = BTreeMap::new();
    for warning in warnings {
        by_fingerprint
            .entry(warning.fingerprint())
            .or_insert(warning);
    }
    by_fingerprint.into_values().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_warning_parser_detects_warning_line() {
        let warnings = parse_warnings(
            "cargo test",
            "",
            "warning: unused variable: `value`\n  --> src/lib.rs:10:5\n",
        );

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, "unused variable: `value`");
    }

    #[test]
    fn cargo_warning_parser_attaches_location() {
        let warnings = parse_warnings(
            "cargo test",
            "",
            "warning: unused import\n  --> crates/demo/src/lib.rs:7:9\n",
        );

        assert_eq!(warnings[0].path.as_deref(), Some("crates/demo/src/lib.rs"));
        assert_eq!(warnings[0].line, Some(7));
        assert_eq!(warnings[0].column, Some(9));
    }
}
