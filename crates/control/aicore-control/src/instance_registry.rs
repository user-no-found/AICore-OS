use std::path::PathBuf;

use aicore_contracts::{InstanceKind, InstanceRecord, LifecycleState, RegistrationRecord, RegistryKind};
use aicore_foundation::{AicoreLayout, InstanceId};

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

    pub fn register(&mut self, instance: InstanceRecord) -> Result<(), String> {
        if instance.id == InstanceId::global_main() && instance.kind != InstanceKind::GlobalMain {
            return Err("global-main must use InstanceKind::GlobalMain".to_string());
        }

        if instance.id != InstanceId::global_main() && instance.kind == InstanceKind::GlobalMain {
            return Err("only global-main can use InstanceKind::GlobalMain".to_string());
        }

        if self.instances.iter().any(|existing| existing.id == instance.id) {
            return Err(format!("duplicate instance id: {}", instance.id.as_str()));
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
) -> Result<InstanceRecord, String> {
    let id = InstanceId::new(id).map_err(|error| error.to_string())?;
    if id == InstanceId::global_main() {
        return Err("workspace instance cannot use global-main id".to_string());
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
    let mut registry = InstanceRegistry::new();
    registry
        .register(global_main_instance(&layout))
        .expect("default instance registry should contain global-main");
    registry
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use aicore_contracts::InstanceKind;
    use aicore_foundation::AicoreLayout;

    use super::{default_instance_registry, global_main_instance, workspace_instance};

    #[test]
    fn contains_global_main_by_default() {
        let registry = default_instance_registry();
        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0].id.as_str(), "global-main");
        assert_eq!(registry.list()[0].kind, InstanceKind::GlobalMain);
    }

    #[test]
    fn allows_workspace_instance_registration() {
        let layout = AicoreLayout::new("/home/demo");
        let mut registry = default_instance_registry();
        let instance = workspace_instance("inst_project_a", "/workspace/project-a", &layout)
            .expect("workspace instance should be valid");

        registry.register(instance).expect("workspace instance should register");

        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn rejects_duplicate_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let mut registry = default_instance_registry();
        let error = registry
            .register(global_main_instance(&layout))
            .expect_err("duplicate global-main should be rejected");

        assert_eq!(error, "duplicate instance id: global-main");
    }

    #[test]
    fn rejects_workspace_impersonation_of_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let error = workspace_instance("global-main", "/workspace/fake", &layout)
            .expect_err("global-main impersonation should fail");

        assert_eq!(error, "workspace instance cannot use global-main id");
    }

    #[test]
    fn uses_real_paths_for_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let main = global_main_instance(&layout);

        assert_eq!(main.workspace_root, PathBuf::from("/home/demo"));
        assert_eq!(main.state_root, PathBuf::from("/home/demo/.aicore/main"));
    }
}
