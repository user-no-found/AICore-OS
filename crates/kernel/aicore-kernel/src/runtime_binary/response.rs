use super::error::KernelRuntimeBinaryErrorKind;
use super::protocol::{
    RUNTIME_BINARY_CONTRACT_VERSION, RUNTIME_BINARY_PROTOCOL, RUNTIME_BINARY_PROTOCOL_VERSION,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRuntimeBinaryResponse {
    pub protocol: String,
    pub protocol_version: String,
    pub contract_version: String,
    pub payload: serde_json::Value,
    pub exit_code: Option<i32>,
}

pub(super) fn parse_kernel_invocation_result_response(
    stdout: &str,
    exit_code: Option<i32>,
) -> Result<KernelRuntimeBinaryResponse, KernelRuntimeBinaryErrorKind> {
    stdout
        .lines()
        .find_map(|line| {
            let value: serde_json::Value = serde_json::from_str(line).ok()?;
            if value.get("event").and_then(|event| event.as_str())
                == Some("kernel.invocation.result")
            {
                let protocol = value
                    .get("protocol")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let protocol_version = value
                    .get("protocol_version")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                let contract_version = value
                    .get("contract_version")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default();
                if protocol != RUNTIME_BINARY_PROTOCOL
                    || protocol_version != RUNTIME_BINARY_PROTOCOL_VERSION
                {
                    return Some(Err(KernelRuntimeBinaryErrorKind::ProtocolVersionMismatch));
                }
                if contract_version != RUNTIME_BINARY_CONTRACT_VERSION {
                    return Some(Err(KernelRuntimeBinaryErrorKind::ContractVersionMismatch));
                }
                return Some(Ok(KernelRuntimeBinaryResponse {
                    protocol: protocol.to_string(),
                    protocol_version: protocol_version.to_string(),
                    contract_version: contract_version.to_string(),
                    payload: value
                        .get("payload")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    exit_code,
                }));
            }
            None
        })
        .unwrap_or(Err(KernelRuntimeBinaryErrorKind::InvalidJsonlOutput))
}
