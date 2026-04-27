use crate::{AuditContext, TraceContext};

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
            instance_id: instance_id.into(),
            capability: capability.into(),
            operation: operation.into(),
            payload,
            policy: InvocationPolicy::default(),
            trace_context: TraceContext::new("trace.default"),
            audit_context: AuditContext::system("kernel invocation"),
        }
    }
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
    }
}
