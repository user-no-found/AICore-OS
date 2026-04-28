use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use aicore_foundation::{AicoreClock, SystemClock};

use crate::{KernelInvocationEnvelope, KernelRouteRuntimeOutput, format_contract};

const INVOCATION_LEDGER_SCHEMA: &str = "aicore.kernel.invocation_ledger.v1";

static LEDGER_RECORD_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationLedger {
    path: PathBuf,
    fail_stage: Option<String>,
}

impl KernelInvocationLedger {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            fail_stage: None,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn append(&self, record: &KernelInvocationLedgerRecord) -> Result<(), String> {
        if self.fail_stage.as_deref() == Some(record.stage.as_str()) {
            return Err(format!(
                "failed to append invocation ledger stage {}",
                record.stage
            ));
        }

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建 invocation ledger 目录失败: {error}"))?;
        }
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|error| format!("打开 invocation ledger 失败: {error}"))?;
        writeln!(file, "{}", record.to_json_line())
            .map_err(|error| format!("写入 invocation ledger 失败: {error}"))
    }

    #[cfg(test)]
    pub fn failing_for_test(path: impl AsRef<Path>, stage: impl Into<String>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            fail_stage: Some(stage.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationLedgerRecord {
    pub schema_version: String,
    pub record_id: String,
    pub timestamp: String,
    pub invocation_id: String,
    pub trace_id: String,
    pub instance_id: String,
    pub operation: String,
    pub stage: String,
    pub status: String,
    pub component_id: Option<String>,
    pub app_id: Option<String>,
    pub capability_id: Option<String>,
    pub contract_version: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub handler_kind: Option<String>,
    pub handler_executed: bool,
    pub event_generated: bool,
    pub spawned_process: bool,
    pub called_real_component: bool,
    pub transport: Option<String>,
    pub process_exit_code: Option<i32>,
}

impl KernelInvocationLedgerRecord {
    pub fn new(
        stage: impl Into<String>,
        status: impl Into<String>,
        envelope: &KernelInvocationEnvelope,
    ) -> Self {
        let stage = stage.into();
        let counter = LEDGER_RECORD_COUNTER.fetch_add(1, Ordering::Relaxed);
        let process_id = std::process::id();
        let timestamp = SystemClock.now().unix_millis().to_string();
        Self {
            schema_version: INVOCATION_LEDGER_SCHEMA.to_string(),
            record_id: format!("ledger.{timestamp}.{process_id}.{counter}.{stage}"),
            timestamp,
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            instance_id: envelope.instance_id.clone(),
            operation: envelope.operation.clone(),
            stage,
            status: status.into(),
            component_id: None,
            app_id: None,
            capability_id: None,
            contract_version: None,
            failure_stage: None,
            failure_reason: None,
            handler_kind: None,
            handler_executed: false,
            event_generated: false,
            spawned_process: false,
            called_real_component: false,
            transport: None,
            process_exit_code: None,
        }
    }

    pub fn with_route(mut self, route: &KernelRouteRuntimeOutput) -> Self {
        self.component_id = Some(route.component_id.clone());
        self.app_id = Some(route.app_id.clone());
        self.capability_id = Some(route.capability_id.clone());
        self.contract_version = Some(format_contract(&route.contract_version));
        self
    }

    pub fn with_failure(
        mut self,
        failure_stage: impl Into<String>,
        failure_reason: impl Into<String>,
    ) -> Self {
        self.failure_stage = Some(failure_stage.into());
        self.failure_reason = Some(redact_failure_reason(&failure_reason.into()));
        self
    }

    pub fn with_handler(
        mut self,
        handler_kind: Option<&str>,
        handler_executed: bool,
        event_generated: bool,
        spawned_process: bool,
        called_real_component: bool,
    ) -> Self {
        self.handler_kind = handler_kind.map(ToOwned::to_owned);
        self.handler_executed = handler_executed;
        self.event_generated = event_generated;
        self.spawned_process = spawned_process;
        self.called_real_component = called_real_component;
        self
    }

    pub fn with_transport(mut self, transport: Option<&str>) -> Self {
        self.transport = transport.map(ToOwned::to_owned);
        self
    }

    pub fn with_process_exit_code(mut self, code: Option<i32>) -> Self {
        self.process_exit_code = code;
        self
    }

    pub fn to_json_line(&self) -> String {
        format!(
            "{{{}}}",
            [
                json_string("schema_version", &self.schema_version),
                json_string("record_id", &self.record_id),
                json_string("timestamp", &self.timestamp),
                json_string("invocation_id", &self.invocation_id),
                json_string("trace_id", &self.trace_id),
                json_string("instance_id", &self.instance_id),
                json_string("operation", &self.operation),
                json_string("stage", &self.stage),
                json_string("status", &self.status),
                json_optional_string("component_id", self.component_id.as_deref()),
                json_optional_string("app_id", self.app_id.as_deref()),
                json_optional_string("capability_id", self.capability_id.as_deref()),
                json_optional_string("contract_version", self.contract_version.as_deref()),
                json_optional_string("failure_stage", self.failure_stage.as_deref()),
                json_optional_string("failure_reason", self.failure_reason.as_deref()),
                json_optional_string("handler_kind", self.handler_kind.as_deref()),
                json_bool("handler_executed", self.handler_executed),
                json_bool("event_generated", self.event_generated),
                json_bool("spawned_process", self.spawned_process),
                json_bool("called_real_component", self.called_real_component),
                json_optional_string("transport", self.transport.as_deref()),
                json_optional_i32("process_exit_code", self.process_exit_code),
            ]
            .join(",")
        )
    }
}

pub fn redact_failure_reason(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    let sensitive_markers = [
        "secret://",
        "credential_lease_ref",
        "secret_ref",
        "api key",
        "api_key",
        "token=",
        "cookie",
        "sk-",
    ];
    if sensitive_markers
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "[redacted:failure_reason]".to_string()
    } else {
        reason.to_string()
    }
}

fn json_string(key: &str, value: &str) -> String {
    format!("\"{}\":\"{}\"", escape_json(key), escape_json(value))
}

fn json_optional_string(key: &str, value: Option<&str>) -> String {
    match value {
        Some(value) => json_string(key, value),
        None => format!("\"{}\":null", escape_json(key)),
    }
}

fn json_bool(key: &str, value: bool) -> String {
    format!("\"{}\":{}", escape_json(key), value)
}

fn json_optional_i32(key: &str, value: Option<i32>) -> String {
    match value {
        Some(value) => format!("\"{}\":{}", escape_json(key), value),
        None => format!("\"{}\":null", escape_json(key)),
    }
}

fn escape_json(value: &str) -> String {
    let mut escaped = String::new();
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", value as u32));
            }
            value => escaped.push(value),
        }
    }
    escaped
}
