use crate::{AuditContext, ContractVersion, TraceContext};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteRequest {
    pub instance_id: String,
    pub capability: String,
    pub operation: String,
    pub requested_contract: Option<ContractVersion>,
    pub trace_context: TraceContext,
    pub audit_context: AuditContext,
}

impl KernelRouteRequest {
    pub fn new(
        instance_id: impl Into<String>,
        capability: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            capability: capability.into(),
            operation: operation.into(),
            requested_contract: None,
            trace_context: TraceContext::new("trace.route"),
            audit_context: AuditContext::system("route request"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteTarget {
    pub app_id: String,
    pub contract_version: ContractVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelRoutePolicy {
    PrimaryOnly,
    AllowFallback,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteReason {
    ExactCapabilityOperation,
    FallbackCapability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackChain {
    pub targets: Vec<KernelRouteTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelRouteDecision {
    pub request: KernelRouteRequest,
    pub target: KernelRouteTarget,
    pub route_policy: KernelRoutePolicy,
    pub route_reason: RouteReason,
    pub fallback_chain: FallbackChain,
}

impl KernelRouteDecision {
    pub fn new(request: KernelRouteRequest, target: KernelRouteTarget) -> Self {
        Self {
            request,
            target,
            route_policy: KernelRoutePolicy::PrimaryOnly,
            route_reason: RouteReason::ExactCapabilityOperation,
            fallback_chain: FallbackChain {
                targets: Vec::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ContractVersion;

    use super::{KernelRouteDecision, KernelRouteRequest, KernelRouteTarget};

    #[test]
    fn route_request_carries_instance_and_capability() {
        let request = KernelRouteRequest::new("global-main", "memory.search", "search");

        assert_eq!(request.instance_id, "global-main");
        assert_eq!(request.capability, "memory.search");
    }

    #[test]
    fn route_decision_targets_app_and_contract_version() {
        let request = KernelRouteRequest::new("global-main", "provider.chat", "complete");
        let decision = KernelRouteDecision::new(
            request,
            KernelRouteTarget {
                app_id: "app.provider".to_string(),
                contract_version: ContractVersion::new("kernel.provider", 1, 0),
            },
        );

        assert_eq!(decision.target.app_id, "app.provider");
        assert_eq!(decision.target.contract_version.major, 1);
    }
}
