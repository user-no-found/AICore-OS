use std::path::PathBuf;

use aicore_contracts::{
    InstanceKind, InstanceRecord, LifecycleState, RegistrationRecord, RegistryKind,
};
use aicore_foundation::{AicoreError, AicoreLayout, AicoreResult, InstanceId};

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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use aicore_contracts::InstanceKind;
    use aicore_foundation::{AicoreError, AicoreLayout};

    use super::{
        default_instance_registry, default_instance_registry_with_layout, global_main_instance,
        workspace_instance,
    };

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

        registry
            .register(instance)
            .expect("workspace instance should register");

        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn rejects_duplicate_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let mut registry = default_instance_registry();
        let error = registry
            .register(global_main_instance(&layout))
            .expect_err("duplicate global-main should be rejected");

        assert_eq!(error.to_string(), "duplicate: instance id: global-main");
    }

    #[test]
    fn rejects_workspace_impersonation_of_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let error = workspace_instance("global-main", "/workspace/fake", &layout)
            .expect_err("global-main impersonation should fail");

        assert_eq!(
            error,
            AicoreError::InvalidState("workspace instance cannot use global-main id".to_string())
        );
    }

    #[test]
    fn uses_real_paths_for_global_main() {
        let layout = AicoreLayout::new("/home/demo");
        let main = global_main_instance(&layout);

        assert_eq!(main.workspace_root, PathBuf::from("/home/demo"));
        assert_eq!(main.state_root, PathBuf::from("/home/demo/.aicore/main"));
    }

    #[test]
    fn supports_get_global_main_and_workspaces() {
        let layout = AicoreLayout::new("/home/demo");
        let mut registry = default_instance_registry();
        let workspace = workspace_instance("inst_project_a", "/workspace/project-a", &layout)
            .expect("workspace instance should be valid");
        let workspace_id = workspace.id.clone();
        registry
            .register(workspace)
            .expect("workspace instance should register");

        assert_eq!(
            registry
                .get(&workspace_id)
                .expect("workspace must exist")
                .id
                .as_str(),
            "inst_project_a"
        );
        assert_eq!(
            registry
                .global_main()
                .expect("global-main must exist")
                .id
                .as_str(),
            "global-main"
        );
        assert_eq!(registry.workspaces().len(), 1);
    }

    #[test]
    fn builds_default_registry_from_explicit_layout() {
        let layout = AicoreLayout::new("/home/custom");
        let registry = default_instance_registry_with_layout(&layout);
        let main = registry
            .global_main()
            .expect("global-main must exist in explicit layout registry");

        assert_eq!(main.workspace_root, PathBuf::from("/home/custom"));
        assert_eq!(main.state_root, PathBuf::from("/home/custom/.aicore/main"));
    }
}
