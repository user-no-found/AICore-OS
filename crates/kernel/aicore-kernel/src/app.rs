use crate::{CapabilityDescriptor, ContractVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Registered,
    Installed,
    Running,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallAction {
    Install,
    Upgrade,
    Remove,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    pub level: HealthLevel,
    pub summary_zh: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestMetadata {
    pub name: String,
    pub version: String,
    pub summary_zh: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionDescriptor {
    pub current: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compatibility {
    pub min_runtime_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub description_en: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityBoundary {
    pub capability_id: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionBoundary {
    pub scope: String,
    pub capabilities: Vec<CapabilityBoundary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentManifest {
    pub id: String,
    pub kind: String,
    pub manifest: ManifestMetadata,
    pub display_name_zh: String,
    pub description_zh: String,
    pub version: VersionDescriptor,
    pub compatibility: Compatibility,
    pub permission_boundary: PermissionBoundary,
    pub capabilities: Vec<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRuntimeKind {
    Cli,
    Tui,
    Web,
    Provider,
    Toolset,
    Gateway,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppStatus {
    Registered,
    Available,
    Running,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppLifecycleEvent {
    Registered,
    Started,
    Stopped,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHealth {
    pub status: AppStatus,
    pub health: HealthStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppManifest {
    pub app_id: String,
    pub runtime_kind: AppRuntimeKind,
    pub display_name_zh: String,
    pub contracts: Vec<ContractVersion>,
    pub capabilities: Vec<CapabilityDescriptor>,
    pub permission_boundary: PermissionBoundary,
}

impl AppManifest {
    pub fn new(app_id: impl Into<String>, kind: impl Into<String>) -> Self {
        let kind = match kind.into().as_str() {
            "cli" => AppRuntimeKind::Cli,
            "tui" => AppRuntimeKind::Tui,
            "web" => AppRuntimeKind::Web,
            "provider" => AppRuntimeKind::Provider,
            "toolset" => AppRuntimeKind::Toolset,
            "gateway" => AppRuntimeKind::Gateway,
            _ => AppRuntimeKind::Service,
        };

        Self {
            app_id: app_id.into(),
            runtime_kind: kind,
            display_name_zh: "应用".to_string(),
            contracts: Vec::new(),
            capabilities: Vec::new(),
            permission_boundary: PermissionBoundary {
                scope: "app".to_string(),
                capabilities: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRegistration {
    pub app_id: String,
    pub status: AppStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppHandshake {
    pub app_id: String,
    pub contracts: Vec<ContractVersion>,
    pub capabilities: Vec<CapabilityDescriptor>,
}

#[cfg(test)]
mod tests {
    use crate::{CapabilityDescriptor, ContractVersion};

    use super::AppHandshake;

    #[test]
    fn app_handshake_declares_contracts_and_capabilities() {
        let handshake = AppHandshake {
            app_id: "app.provider".to_string(),
            contracts: vec![ContractVersion::new("kernel.provider", 1, 0)],
            capabilities: vec![CapabilityDescriptor::new("provider.chat")],
        };

        assert_eq!(handshake.contracts[0].contract_id, "kernel.provider");
        assert_eq!(handshake.capabilities[0].capability_id, "provider.chat");
    }
}
