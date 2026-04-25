use aicore_contracts::{
    Capability, CapabilityBoundary, Compatibility, ComponentManifest, InstallAction,
    LifecycleState, ManifestMetadata, PermissionBoundary, RegistrationRecord, RegistryKind,
    VersionDescriptor,
};

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
            InstallAction::Install => LifecycleState::Installed,
            InstallAction::Upgrade => LifecycleState::Installed,
            InstallAction::Remove => LifecycleState::Stopped,
        };
    }

    pub fn is_compatible_with(&self, runtime_version: &str) -> bool {
        self.apps
            .iter()
            .all(|app| app.compatibility.min_runtime_version.as_str() <= runtime_version)
    }
}

pub fn default_component_registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();

    for app in [
        ComponentManifest {
            id: "app.cli".to_string(),
            kind: "ui".to_string(),
            manifest: ManifestMetadata {
                name: "aicore-cli-entry".to_string(),
                version: "0.1.0".to_string(),
                summary_zh: "固定总入口组件".to_string(),
            },
            display_name_zh: "命令行".to_string(),
            description_zh: "固定总入口与脚本化控制界面".to_string(),
            version: VersionDescriptor {
                current: "0.1.0".to_string(),
            },
            compatibility: Compatibility {
                min_runtime_version: "0.1.0".to_string(),
            },
            permission_boundary: PermissionBoundary {
                scope: "component.app.cli".to_string(),
                capabilities: vec![CapabilityBoundary {
                    capability_id: "entry.command".to_string(),
                    requires_approval: false,
                }],
            },
            capabilities: vec![Capability {
                id: "entry.command".to_string(),
                description_en: "Provides the fixed top-level command entry.".to_string(),
            }],
        },
        ComponentManifest {
            id: "ui.tui".to_string(),
            kind: "ui".to_string(),
            manifest: ManifestMetadata {
                name: "aicore-tui".to_string(),
                version: "0.1.0".to_string(),
                summary_zh: "终端界面组件".to_string(),
            },
            display_name_zh: "终端界面".to_string(),
            description_zh: "交互式本地终端界面组件".to_string(),
            version: VersionDescriptor {
                current: "0.1.0".to_string(),
            },
            compatibility: Compatibility {
                min_runtime_version: "0.1.0".to_string(),
            },
            permission_boundary: PermissionBoundary {
                scope: "component.ui.tui".to_string(),
                capabilities: vec![CapabilityBoundary {
                    capability_id: "view.active".to_string(),
                    requires_approval: false,
                }],
            },
            capabilities: vec![Capability {
                id: "view.active".to_string(),
                description_en: "Provides a local active conversation view.".to_string(),
            }],
        },
        ComponentManifest {
            id: "ui.web".to_string(),
            kind: "ui".to_string(),
            manifest: ManifestMetadata {
                name: "aicore-web".to_string(),
                version: "0.1.0".to_string(),
                summary_zh: "网页界面组件".to_string(),
            },
            display_name_zh: "网页界面".to_string(),
            description_zh: "可选网页界面组件".to_string(),
            version: VersionDescriptor {
                current: "0.1.0".to_string(),
            },
            compatibility: Compatibility {
                min_runtime_version: "0.1.0".to_string(),
            },
            permission_boundary: PermissionBoundary {
                scope: "component.ui.web".to_string(),
                capabilities: vec![CapabilityBoundary {
                    capability_id: "view.web".to_string(),
                    requires_approval: false,
                }],
            },
            capabilities: vec![Capability {
                id: "view.web".to_string(),
                description_en: "Provides a web conversation and control view.".to_string(),
            }],
        },
    ] {
        registry
            .register(app)
            .expect("default component registry should not contain duplicates");
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::{ComponentRegistry, default_component_registry};
    use aicore_contracts::{
        Compatibility, ComponentManifest, InstallAction, LifecycleState, ManifestMetadata,
        PermissionBoundary, VersionDescriptor,
    };

    #[test]
    fn exposes_known_component_summaries() {
        let apps = default_component_registry().summaries();
        assert!(apps.iter().any(|app| app.id == "app.cli"));
        assert!(apps.iter().any(|app| app.id == "ui.tui"));
        assert!(apps.iter().any(|app| app.id == "ui.web"));
    }

    #[test]
    fn rejects_duplicate_component_ids() {
        let mut registry = ComponentRegistry::new();
        let manifest = ComponentManifest {
            id: "dup.component".to_string(),
            kind: "ui".to_string(),
            manifest: ManifestMetadata {
                name: "dup-component".to_string(),
                version: "0.1.0".to_string(),
                summary_zh: "重复组件".to_string(),
            },
            display_name_zh: "重复组件".to_string(),
            description_zh: "用于测试重复注册".to_string(),
            version: VersionDescriptor {
                current: "0.1.0".to_string(),
            },
            compatibility: Compatibility {
                min_runtime_version: "0.1.0".to_string(),
            },
            permission_boundary: PermissionBoundary {
                scope: "component.dup".to_string(),
                capabilities: Vec::new(),
            },
            capabilities: Vec::new(),
        };

        registry
            .register(manifest.clone())
            .expect("first insert should pass");
        let error = registry
            .register(manifest)
            .expect_err("duplicate insert should fail");

        assert_eq!(error, "duplicate component id: dup.component");
    }

    #[test]
    fn applies_install_action_to_registry_lifecycle() {
        let mut registry = default_component_registry();
        assert_eq!(registry.lifecycle_state(), &LifecycleState::Registered);

        registry.apply_install_action(InstallAction::Install);
        assert_eq!(registry.lifecycle_state(), &LifecycleState::Installed);

        registry.apply_install_action(InstallAction::Remove);
        assert_eq!(registry.lifecycle_state(), &LifecycleState::Stopped);
    }

    #[test]
    fn checks_runtime_compatibility() {
        let registry = default_component_registry();
        assert!(registry.is_compatible_with("0.1.0"));
        assert!(!registry.is_compatible_with("0.0.1"));
    }
}
