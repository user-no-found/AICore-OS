use aicore_foundation::AicoreLayout;

use crate::KernelInvocationEnvelope;

use super::protocol::{
    RUNTIME_BINARY_CONTRACT_VERSION, RUNTIME_BINARY_PROTOCOL, RUNTIME_BINARY_PROTOCOL_VERSION,
    RUNTIME_BINARY_REQUEST_SCHEMA_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryRequest {
    pub schema_version: String,
    pub request_id: String,
    pub protocol: String,
    pub protocol_version: String,
    pub contract_version: String,
    pub invocation_id: String,
    pub trace_id: String,
    pub instance_id: String,
    pub capability: String,
    pub operation: String,
    pub payload_summary: String,
    pub ledger_path: String,
}

impl KernelRuntimeBinaryRequest {
    pub fn from_envelope(envelope: &KernelInvocationEnvelope, layout: &AicoreLayout) -> Self {
        Self {
            schema_version: RUNTIME_BINARY_REQUEST_SCHEMA_VERSION.to_string(),
            request_id: format!("request.{}", envelope.invocation_id),
            protocol: RUNTIME_BINARY_PROTOCOL.to_string(),
            protocol_version: RUNTIME_BINARY_PROTOCOL_VERSION.to_string(),
            contract_version: RUNTIME_BINARY_CONTRACT_VERSION.to_string(),
            invocation_id: envelope.invocation_id.clone(),
            trace_id: envelope.trace_context.trace_id.clone(),
            instance_id: envelope.instance_id.clone(),
            capability: envelope.capability.clone(),
            operation: envelope.operation.clone(),
            payload_summary: envelope.payload.summary(),
            ledger_path: layout
                .kernel_state_root
                .join("invocation-ledger.jsonl")
                .display()
                .to_string(),
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "schema_version": self.schema_version,
            "request_id": self.request_id,
            "protocol": self.protocol,
            "protocol_version": self.protocol_version,
            "contract_version": self.contract_version,
            "invocation_id": self.invocation_id,
            "trace_id": self.trace_id,
            "instance_id": self.instance_id,
            "capability": self.capability,
            "operation": self.operation,
            "payload": self.payload_summary,
            "ledger_path": self.ledger_path,
        })
    }
}
