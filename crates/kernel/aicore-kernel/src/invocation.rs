use std::sync::atomic::{AtomicU64, Ordering};

use aicore_foundation::{AicoreClock, SystemClock};

use crate::{AuditContext, TraceContext};

static INVOCATION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelPayload {
    Text(String),
    JsonSummary(String),
    Empty,
}

impl KernelPayload {
    pub fn summary(&self) -> String {
        match self {
            Self::Text(value) => format!("text:{} bytes", value.len()),
            Self::JsonSummary(value) => format!("json:{value}"),
            Self::Empty => "empty".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutPolicy {
    Inherit,
    Millis(u64),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialRef {
    pub auth_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationPolicy {
    pub timeout: TimeoutPolicy,
    pub credential_ref: Option<CredentialRef>,
    pub allow_parallel_reads: bool,
}

impl Default for InvocationPolicy {
    fn default() -> Self {
        Self {
            timeout: TimeoutPolicy::Inherit,
            credential_ref: None,
            allow_parallel_reads: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationEnvelope {
    pub invocation_id: String,
    pub instance_id: String,
    pub capability: String,
    pub operation: String,
    pub payload: KernelPayload,
    pub policy: InvocationPolicy,
    pub trace_context: TraceContext,
    pub audit_context: AuditContext,
}

impl KernelInvocationEnvelope {
    pub fn new(
        instance_id: impl Into<String>,
        capability: impl Into<String>,
        operation: impl Into<String>,
        payload: KernelPayload,
    ) -> Self {
        Self {
            invocation_id: generate_invocation_id(),
            instance_id: instance_id.into(),
            capability: capability.into(),
            operation: operation.into(),
            payload,
            policy: InvocationPolicy::default(),
            trace_context: TraceContext::new("trace.default"),
            audit_context: AuditContext::system("kernel invocation"),
        }
    }

    pub fn with_invocation_id(mut self, invocation_id: impl Into<String>) -> Self {
        self.invocation_id = invocation_id.into();
        self
    }
}

fn generate_invocation_id() -> String {
    let timestamp = SystemClock.now().unix_millis();
    let process_id = std::process::id();
    let counter = INVOCATION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("invoke.{timestamp}.{process_id}.{counter}")
}

#[cfg(test)]
mod tests {
    use super::{KernelInvocationEnvelope, KernelPayload};

    #[test]
    fn invocation_envelope_carries_typed_payload_summary() {
        let envelope = KernelInvocationEnvelope::new(
            "global-main",
            "provider.chat",
            "complete",
            KernelPayload::Text("hello".to_string()),
        );

        assert_eq!(envelope.instance_id, "global-main");
        assert_eq!(envelope.capability, "provider.chat");
        assert_eq!(envelope.payload.summary(), "text:5 bytes");
        assert!(envelope.invocation_id.starts_with("invoke."));
    }

    #[test]
    fn kernel_invocation_envelope_assigns_unique_invocation_id() {
        let first = KernelInvocationEnvelope::new(
            "global-main",
            "memory.search",
            "memory.search",
            KernelPayload::Empty,
        );
        let second = KernelInvocationEnvelope::new(
            "global-main",
            "memory.search",
            "memory.search",
            KernelPayload::Empty,
        );

        assert_ne!(first.invocation_id, second.invocation_id);
        assert!(first.invocation_id.starts_with("invoke."));
        assert!(second.invocation_id.starts_with("invoke."));
    }

    #[test]
    fn invocation_envelope_allows_explicit_invocation_id_for_tests() {
        let envelope = KernelInvocationEnvelope::new(
            "global-main",
            "memory.search",
            "memory.search",
            KernelPayload::Empty,
        )
        .with_invocation_id("invoke.test.fixed");

        assert_eq!(envelope.invocation_id, "invoke.test.fixed");
    }
}
