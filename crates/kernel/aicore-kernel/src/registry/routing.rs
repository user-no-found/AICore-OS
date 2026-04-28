use crate::{
    CapabilityDescriptor, ContractVersion, KernelError, KernelErrorCode, KernelErrorStage,
    KernelRouteDecision, KernelRouteRequest, KernelRouteTarget, RouteReason,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRegistry {
    entries: Vec<CapabilityRegistryEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityRegistryEntry {
    pub app_id: String,
    pub capability: CapabilityDescriptor,
    pub contract_version: ContractVersion,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn register(
        &mut self,
        app_id: impl Into<String>,
        capability: CapabilityDescriptor,
        contract_version: ContractVersion,
    ) {
        self.entries.push(CapabilityRegistryEntry {
            app_id: app_id.into(),
            capability,
            contract_version,
        });
    }

    pub fn find(&self, capability: &str, operation: &str) -> Option<&CapabilityRegistryEntry> {
        self.entries.iter().find(|entry| {
            entry.capability.capability_id == capability
                && entry
                    .capability
                    .operations
                    .iter()
                    .any(|item| item.operation == operation)
        })
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRoutePlanner {
    capability_registry: CapabilityRegistry,
}

impl KernelRoutePlanner {
    pub fn new(capability_registry: CapabilityRegistry) -> Self {
        Self {
            capability_registry,
        }
    }

    pub fn plan(&self, request: KernelRouteRequest) -> Result<KernelRouteDecision, KernelError> {
        let entry = self
            .capability_registry
            .find(&request.capability, &request.operation)
            .ok_or_else(|| {
                KernelError::new(
                    KernelErrorCode::MissingCapability,
                    KernelErrorStage::Route,
                    "缺少能力路由",
                )
            })?;

        if let Some(requested) = &request.requested_contract {
            if requested.contract_id != entry.contract_version.contract_id
                || requested.major != entry.contract_version.major
            {
                return Err(KernelError::new(
                    KernelErrorCode::VersionMismatch,
                    KernelErrorStage::Route,
                    "合同版本不兼容",
                ));
            }
        }

        Ok(KernelRouteDecision {
            target: KernelRouteTarget {
                app_id: entry.app_id.clone(),
                contract_version: entry.contract_version.clone(),
            },
            request,
            route_policy: crate::KernelRoutePolicy::PrimaryOnly,
            route_reason: RouteReason::ExactCapabilityOperation,
            fallback_chain: crate::FallbackChain {
                targets: Vec::new(),
            },
        })
    }
}
