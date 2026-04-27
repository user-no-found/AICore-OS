use std::path::PathBuf;

use aicore_foundation::{AicoreError, AicoreLayout, AicoreResult, InstanceId};

use crate::{
    AppManifest, CapabilityDescriptor, Compatibility, ComponentManifest, ContractVersion,
    HealthLevel, HealthStatus, InstallAction, InstanceKind, InstanceRecord, KernelError,
    KernelErrorCode, KernelErrorStage, KernelRouteDecision, KernelRouteRequest, KernelRouteTarget,
    LifecycleState, ManifestMetadata, PermissionBoundary, RouteReason, VersionDescriptor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryKind {
    Component,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationRecord {
    pub registry_kind: RegistryKind,
    pub subject_id: String,
    pub lifecycle_state: LifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSummary {
    pub id: String,
    pub kind: String,
    pub description_zh: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRegistrySummary {
    pub component_count: usize,
    pub lifecycle_state: LifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentRegistry {
    apps: Vec<ComponentManifest>,
    lifecycle_state: LifecycleState,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            apps: Vec::new(),
            lifecycle_state: LifecycleState::Registered,
        }
    }

    pub fn register(&mut self, app: ComponentManifest) -> Result<(), String> {
        if self.apps.iter().any(|existing| existing.id == app.id) {
            return Err(format!("duplicate component id: {}", app.id));
        }

        self.apps.push(app);
        Ok(())
    }

    pub fn list(&self) -> &[ComponentManifest] {
        &self.apps
    }

    pub fn summaries(&self) -> Vec<AppSummary> {
        self.apps
            .iter()
            .map(|app| AppSummary {
                id: app.id.clone(),
                kind: app.kind.clone(),
                description_zh: app.description_zh.clone(),
            })
            .collect()
    }

    pub fn registrations(&self) -> Vec<RegistrationRecord> {
        self.apps
            .iter()
            .map(|app| RegistrationRecord {
                registry_kind: RegistryKind::Component,
                subject_id: app.id.clone(),
                lifecycle_state: self.lifecycle_state.clone(),
            })
            .collect()
    }

    pub fn lifecycle_state(&self) -> &LifecycleState {
        &self.lifecycle_state
    }

    pub fn summary(&self) -> ComponentRegistrySummary {
        ComponentRegistrySummary {
            component_count: self.apps.len(),
            lifecycle_state: self.lifecycle_state.clone(),
        }
    }

    pub fn apply_install_action(&mut self, action: InstallAction) {
        self.lifecycle_state = match action {
            InstallAction::Install | InstallAction::Upgrade => LifecycleState::Installed,
            InstallAction::Remove => LifecycleState::Stopped,
        };
    }

    pub fn is_compatible_with(&self, runtime_version: &str) -> bool {
        self.apps
            .iter()
            .all(|app| app.compatibility.min_runtime_version.as_str() <= runtime_version)
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppRegistry {
    apps: Vec<AppManifest>,
}

impl AppRegistry {
    pub fn new() -> Self {
        Self { apps: Vec::new() }
    }

    pub fn register(&mut self, app: AppManifest) -> Result<(), KernelError> {
        if self
            .apps
            .iter()
            .any(|existing| existing.app_id == app.app_id)
        {
            return Err(KernelError::new(
                KernelErrorCode::Conflict,
                KernelErrorStage::Resolve,
                "应用重复注册",
            ));
        }

        self.apps.push(app);
        Ok(())
    }

    pub fn list(&self) -> &[AppManifest] {
        &self.apps
    }
}

impl Default for AppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRegistry {
    instances: Vec<InstanceRecord>,
}

impl InstanceRegistry {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
        }
    }

    pub fn register(&mut self, instance: InstanceRecord) -> AicoreResult<()> {
        if instance.id == InstanceId::global_main() && instance.kind != InstanceKind::GlobalMain {
            return Err(AicoreError::InvalidState(
                "global-main must use InstanceKind::GlobalMain".to_string(),
            ));
        }

        if instance.id != InstanceId::global_main() && instance.kind == InstanceKind::GlobalMain {
            return Err(AicoreError::InvalidState(
                "only global-main can use InstanceKind::GlobalMain".to_string(),
            ));
        }

        if self
            .instances
            .iter()
            .any(|existing| existing.id == instance.id)
        {
            return Err(AicoreError::Duplicate(format!(
                "instance id: {}",
                instance.id.as_str()
            )));
        }

        self.instances.push(instance);
        Ok(())
    }

    pub fn list(&self) -> &[InstanceRecord] {
        &self.instances
    }

    pub fn registrations(&self) -> Vec<RegistrationRecord> {
        self.instances
            .iter()
            .map(|instance| RegistrationRecord {
                registry_kind: RegistryKind::Instance,
                subject_id: instance.id.as_str().to_string(),
                lifecycle_state: LifecycleState::Registered,
            })
            .collect()
    }

    pub fn get(&self, id: &InstanceId) -> AicoreResult<&InstanceRecord> {
        self.instances
            .iter()
            .find(|instance| &instance.id == id)
            .ok_or_else(|| AicoreError::Missing(format!("instance id: {}", id.as_str())))
    }

    pub fn global_main(&self) -> Option<&InstanceRecord> {
        self.instances
            .iter()
            .find(|instance| instance.id == InstanceId::global_main())
    }

    pub fn workspaces(&self) -> Vec<&InstanceRecord> {
        self.instances
            .iter()
            .filter(|instance| instance.kind == InstanceKind::Workspace)
            .collect()
    }
}

impl Default for InstanceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneSummary {
    pub component_count: usize,
    pub instance_count: usize,
    pub lifecycle_state: LifecycleState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MainInstanceSummary {
    pub id: String,
    pub kind: String,
    pub workspace_root: String,
    pub state_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlane {
    component_registry: ComponentRegistry,
    instance_registry: InstanceRegistry,
    lifecycle_state: LifecycleState,
}

impl ControlPlane {
    pub fn new(component_registry: ComponentRegistry, instance_registry: InstanceRegistry) -> Self {
        Self {
            component_registry,
            instance_registry,
            lifecycle_state: LifecycleState::Registered,
        }
    }

    pub fn app_summaries(&self) -> Vec<AppSummary> {
        self.component_registry.summaries()
    }

    pub fn component_registry(&self) -> &ComponentRegistry {
        &self.component_registry
    }

    pub fn instance_registry(&self) -> &InstanceRegistry {
        &self.instance_registry
    }

    pub fn lifecycle_state(&self) -> &LifecycleState {
        &self.lifecycle_state
    }

    pub fn summary(&self) -> ControlPlaneSummary {
        ControlPlaneSummary {
            component_count: self.component_registry.list().len(),
            instance_count: self.instance_registry.list().len(),
            lifecycle_state: self.lifecycle_state.clone(),
        }
    }

    pub fn main_instance_summary(&self) -> MainInstanceSummary {
        let instance = self
            .instance_registry
            .global_main()
            .expect("global-main must exist in the default control-plane registry");

        MainInstanceSummary {
            id: instance.id.as_str().to_string(),
            kind: match instance.kind {
                InstanceKind::GlobalMain => "global_main".to_string(),
                InstanceKind::Workspace => "workspace".to_string(),
            },
            workspace_root: instance.workspace_root.display().to_string(),
            state_root: instance.state_root.display().to_string(),
        }
    }

    pub fn install(&mut self) {
        self.lifecycle_state = LifecycleState::Installed;
    }

    pub fn start(&mut self) {
        self.lifecycle_state = LifecycleState::Running;
    }

    pub fn stop(&mut self) {
        self.lifecycle_state = LifecycleState::Stopped;
    }

    pub fn health_status(&self) -> HealthStatus {
        HealthStatus {
            level: HealthLevel::Healthy,
            summary_zh: "控制内核骨架可用".to_string(),
        }
    }
}

pub fn global_main_instance(layout: &AicoreLayout) -> InstanceRecord {
    InstanceRecord {
        id: InstanceId::global_main(),
        kind: InstanceKind::GlobalMain,
        workspace_root: layout.home_root.clone(),
        state_root: layout.main_root.clone(),
    }
}

pub fn workspace_instance(
    id: &str,
    workspace_root: impl Into<PathBuf>,
    layout: &AicoreLayout,
) -> AicoreResult<InstanceRecord> {
    let id = InstanceId::new(id)?;
    if id == InstanceId::global_main() {
        return Err(AicoreError::InvalidState(
            "workspace instance cannot use global-main id".to_string(),
        ));
    }

    let workspace_root = workspace_root.into();
    let state_root = layout.instances_root.join(id.as_str());

    Ok(InstanceRecord {
        id,
        kind: InstanceKind::Workspace,
        workspace_root,
        state_root,
    })
}

pub fn default_instance_registry() -> InstanceRegistry {
    let layout = AicoreLayout::from_system_home();
    default_instance_registry_with_layout(&layout)
}

pub fn default_instance_registry_with_layout(layout: &AicoreLayout) -> InstanceRegistry {
    let mut registry = InstanceRegistry::new();
    registry
        .register(global_main_instance(layout))
        .expect("default instance registry should contain global-main");
    registry
}

pub fn default_component_registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();

    for app in [
        component_manifest("app.cli", "ui", "命令行", "固定总入口与脚本化控制界面"),
        component_manifest("ui.tui", "ui", "终端界面", "交互式本地终端界面组件"),
        component_manifest("ui.web", "ui", "网页界面", "可选网页界面组件"),
    ] {
        registry
            .register(app)
            .expect("default component registry should not contain duplicates");
    }

    registry
}

fn component_manifest(
    id: impl Into<String>,
    kind: impl Into<String>,
    display_name_zh: impl Into<String>,
    description_zh: impl Into<String>,
) -> ComponentManifest {
    let id = id.into();
    ComponentManifest {
        id: id.clone(),
        kind: kind.into(),
        manifest: ManifestMetadata {
            name: id.clone(),
            version: "0.1.0".to_string(),
            summary_zh: display_name_zh.into(),
        },
        display_name_zh: id.clone(),
        description_zh: description_zh.into(),
        version: VersionDescriptor {
            current: "0.1.0".to_string(),
        },
        compatibility: Compatibility {
            min_runtime_version: "0.1.0".to_string(),
        },
        permission_boundary: PermissionBoundary {
            scope: format!("component.{id}"),
            capabilities: Vec::new(),
        },
        capabilities: Vec::new(),
    }
}

pub fn default_control_plane() -> ControlPlane {
    ControlPlane::new(default_component_registry(), default_instance_registry())
}

pub fn default_capability_registry() -> CapabilityRegistry {
    let mut registry = CapabilityRegistry::new();
    for (app, capability, operation, contract) in [
        (
            "app.provider",
            "provider.chat",
            "complete",
            "kernel.provider",
        ),
        ("app.memory", "memory.search", "search", "kernel.memory"),
        ("app.tools", "tool.shell", "execute", "kernel.tool"),
    ] {
        registry.register(
            app,
            CapabilityDescriptor::new(capability).with_operation(operation),
            ContractVersion::new(contract, 1, 0),
        );
    }
    registry
}

#[cfg(test)]
mod tests {
    use aicore_foundation::{AicoreLayout, InstanceId};

    use super::{
        AppManifest, AppRegistry, CapabilityDescriptor, KernelRoutePlanner,
        default_capability_registry, default_instance_registry, workspace_instance,
    };
    use crate::{ContractVersion, InstanceKind, KernelErrorCode, KernelRouteRequest};

    #[test]
    fn app_registry_rejects_duplicate_app_id() {
        let mut registry = AppRegistry::new();
        registry
            .register(AppManifest::new("app.cli", "cli"))
            .expect("first app should register");

        let error = registry
            .register(AppManifest::new("app.cli", "cli"))
            .expect_err("duplicate app id should fail");

        assert_eq!(error.code, KernelErrorCode::Conflict);
    }

    #[test]
    fn instance_registry_contains_global_main() {
        let registry = default_instance_registry();

        assert_eq!(registry.list()[0].id.as_str(), "global-main");
        assert_eq!(registry.list()[0].kind, InstanceKind::GlobalMain);
    }

    #[test]
    fn workspace_instance_cannot_impersonate_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let error = workspace_instance("global-main", "/workspace/demo", &layout)
            .expect_err("workspace cannot use global-main");

        assert_eq!(
            error.to_string(),
            "invalid state: workspace instance cannot use global-main id"
        );
    }

    #[test]
    fn capability_registry_finds_app_for_operation() {
        let mut registry = super::CapabilityRegistry::new();
        registry.register(
            "app.memory",
            CapabilityDescriptor::new("memory.search").with_operation("search"),
            ContractVersion::new("kernel.memory", 1, 0),
        );

        let entry = registry
            .find("memory.search", "search")
            .expect("capability should resolve");

        assert_eq!(entry.app_id, "app.memory");
    }

    #[test]
    fn route_planner_routes_provider_chat_to_provider_app() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let decision = planner
            .plan(KernelRouteRequest::new(
                "global-main",
                "provider.chat",
                "complete",
            ))
            .expect("provider chat should route");

        assert_eq!(decision.target.app_id, "app.provider");
    }

    #[test]
    fn route_planner_routes_memory_search_to_memory_app() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let decision = planner
            .plan(KernelRouteRequest::new(
                "global-main",
                "memory.search",
                "search",
            ))
            .expect("memory search should route");

        assert_eq!(decision.target.app_id, "app.memory");
    }

    #[test]
    fn route_planner_routes_tool_shell_to_tools_app() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let decision = planner
            .plan(KernelRouteRequest::new(
                "global-main",
                "tool.shell",
                "execute",
            ))
            .expect("tool shell should route");

        assert_eq!(decision.target.app_id, "app.tools");
    }

    #[test]
    fn route_planner_rejects_missing_capability() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let error = planner
            .plan(KernelRouteRequest::new("global-main", "missing.cap", "run"))
            .expect_err("missing capability should fail");

        assert_eq!(error.code, KernelErrorCode::MissingCapability);
    }

    #[test]
    fn route_planner_rejects_contract_version_mismatch() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let mut request = KernelRouteRequest::new("global-main", "provider.chat", "complete");
        request.requested_contract = Some(ContractVersion::new("kernel.provider", 2, 0));

        let error = planner
            .plan(request)
            .expect_err("version mismatch should fail");

        assert_eq!(error.code, KernelErrorCode::VersionMismatch);
    }

    #[test]
    fn route_decision_includes_trace_and_audit_context() {
        let planner = KernelRoutePlanner::new(default_capability_registry());
        let decision = planner
            .plan(KernelRouteRequest::new(
                "global-main",
                "provider.chat",
                "complete",
            ))
            .expect("provider chat should route");

        assert_eq!(decision.request.trace_context.trace_id, "trace.route");
        assert_eq!(decision.request.audit_context.actor, "system");
    }

    #[test]
    fn workspace_instance_registration_rejects_global_kind_impersonation() {
        let mut registry = default_instance_registry();
        let layout = AicoreLayout::new("/home/demo");
        let mut record =
            workspace_instance("inst-a", "/workspace/a", &layout).expect("workspace should build");
        record.id = InstanceId::new("inst-global").expect("id should be safe");
        record.kind = InstanceKind::GlobalMain;

        let error = registry
            .register(record)
            .expect_err("workspace cannot claim global kind");

        assert_eq!(
            error.to_string(),
            "invalid state: only global-main can use InstanceKind::GlobalMain"
        );
    }
}
