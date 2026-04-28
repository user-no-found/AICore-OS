use std::fmt;

use crate::{
    ComponentInvocationMode, ComponentTransport, ContractVersion, InstalledManifestRegistry,
    KernelErrorCode, KernelRouteDecision, KernelRoutePlanner, KernelRouteRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteRuntimeInput {
    pub instance_id: String,
    pub operation: String,
    pub requested_contract: Option<ContractVersion>,
}

impl KernelRouteRuntimeInput {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            instance_id: "global-main".to_string(),
            operation: operation.into(),
            requested_contract: None,
        }
    }

    pub fn with_instance_id(mut self, instance_id: impl Into<String>) -> Self {
        self.instance_id = instance_id.into();
        self
    }

    pub fn with_requested_contract(mut self, contract: ContractVersion) -> Self {
        self.requested_contract = Some(contract);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteRuntimeOutput {
    pub decision: KernelRouteDecision,
    pub component_id: String,
    pub app_id: String,
    pub capability_id: String,
    pub operation: String,
    pub entrypoint: String,
    pub invocation_mode: ComponentInvocationMode,
    pub transport: ComponentTransport,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_policy: Option<String>,
    pub visibility: String,
    pub contract_version: ContractVersion,
    pub handler_executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelRouteRuntimeError {
    MissingCapability {
        operation: String,
    },
    AmbiguousRoute {
        operation: String,
        candidates: Vec<String>,
    },
    ContractVersionMismatch {
        operation: String,
        installed: ContractVersion,
        expected: ContractVersion,
    },
    RoutePlanner {
        operation: String,
        code: KernelErrorCode,
        message: String,
    },
}

impl KernelRouteRuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingCapability { .. } => "missing capability",
            Self::AmbiguousRoute { .. } => "ambiguous route",
            Self::ContractVersionMismatch { .. } => "contract version mismatch",
            Self::RoutePlanner { .. } => "route planner failed",
        }
    }

    pub fn operation(&self) -> &str {
        match self {
            Self::MissingCapability { operation }
            | Self::AmbiguousRoute { operation, .. }
            | Self::ContractVersionMismatch { operation, .. }
            | Self::RoutePlanner { operation, .. } => operation,
        }
    }
}

impl fmt::Display for KernelRouteRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCapability { operation } => {
                write!(formatter, "missing capability for operation: {operation}")
            }
            Self::AmbiguousRoute {
                operation,
                candidates,
            } => write!(
                formatter,
                "ambiguous route for operation: {operation}; candidates: {}",
                candidates.join(", ")
            ),
            Self::ContractVersionMismatch {
                operation,
                installed,
                expected,
            } => write!(
                formatter,
                "contract version mismatch for operation: {operation}; installed: {}; expected: {}",
                format_contract(installed),
                format_contract(expected)
            ),
            Self::RoutePlanner {
                operation,
                code,
                message,
            } => write!(
                formatter,
                "route planner failed for operation: {operation}; code: {code:?}; message: {message}"
            ),
        }
    }
}

impl std::error::Error for KernelRouteRuntimeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteRuntime {
    registry: InstalledManifestRegistry,
    supported_contract: ContractVersion,
}

impl KernelRouteRuntime {
    pub fn from_registry(registry: InstalledManifestRegistry) -> Self {
        Self {
            registry,
            supported_contract: ContractVersion::new("kernel.app", 1, 0),
        }
    }

    pub fn route(
        &self,
        input: KernelRouteRuntimeInput,
    ) -> Result<KernelRouteRuntimeOutput, KernelRouteRuntimeError> {
        let candidates = self.registry.operation_candidates(&input.operation);
        let candidate = match candidates.as_slice() {
            [] => {
                return Err(KernelRouteRuntimeError::MissingCapability {
                    operation: input.operation,
                });
            }
            [candidate] => candidate,
            _ => {
                return Err(KernelRouteRuntimeError::AmbiguousRoute {
                    operation: input.operation,
                    candidates: candidates
                        .iter()
                        .map(|candidate| {
                            format!("{}:{}", candidate.component_id, candidate.capability_id)
                        })
                        .collect(),
                });
            }
        };

        if !same_major_contract(&candidate.contract_version, &self.supported_contract) {
            return Err(KernelRouteRuntimeError::ContractVersionMismatch {
                operation: input.operation,
                installed: candidate.contract_version.clone(),
                expected: self.supported_contract.clone(),
            });
        }

        if let Some(requested) = &input.requested_contract {
            if !same_major_contract(&candidate.contract_version, requested) {
                return Err(KernelRouteRuntimeError::ContractVersionMismatch {
                    operation: input.operation,
                    installed: candidate.contract_version.clone(),
                    expected: requested.clone(),
                });
            }
        }

        let mut request = KernelRouteRequest::new(
            input.instance_id,
            candidate.capability_id.clone(),
            input.operation.clone(),
        );
        request.requested_contract = input.requested_contract;

        let planner = KernelRoutePlanner::new(self.registry.to_capability_registry());
        let decision =
            planner
                .plan(request)
                .map_err(|error| KernelRouteRuntimeError::RoutePlanner {
                    operation: input.operation.clone(),
                    code: error.code,
                    message: error.message_zh,
                })?;

        Ok(KernelRouteRuntimeOutput {
            decision,
            component_id: candidate.component_id.clone(),
            app_id: candidate.app_id.clone(),
            capability_id: candidate.capability_id.clone(),
            operation: candidate.operation.clone(),
            entrypoint: candidate.entrypoint.clone(),
            invocation_mode: candidate.invocation_mode.clone(),
            transport: candidate.transport.clone(),
            args: candidate.args.clone(),
            working_dir: candidate.working_dir.clone(),
            env_policy: candidate.env_policy.clone(),
            visibility: candidate.visibility.clone(),
            contract_version: candidate.contract_version.clone(),
            handler_executed: false,
        })
    }
}

fn same_major_contract(left: &ContractVersion, right: &ContractVersion) -> bool {
    left.contract_id == right.contract_id && left.major == right.major
}

pub fn format_contract(contract: &ContractVersion) -> String {
    format!("{}.v{}", contract.contract_id, contract.major)
}
