use aicore_contracts::{InstanceRecord, LifecycleState, RegistrationRecord, RegistryKind};

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
        if instance.id == "global-main" && instance.kind != "global_main" {
            return Err("global-main must use kind global_main".to_string());
        }

        if instance.id != "global-main" && instance.kind == "global_main" {
            return Err("only global-main can use kind global_main".to_string());
        }

        if self
            .instances
            .iter()
            .any(|existing| existing.id == instance.id)
        {
            return Err(format!("duplicate instance id: {}", instance.id));
        }

        if instance.id == "global-main"
            && self.instances.iter().any(|existing| existing.id == "global-main")
        {
            return Err("global-main already exists".to_string());
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
                subject_id: instance.id.clone(),
                lifecycle_state: LifecycleState::Registered,
            })
            .collect()
    }
}

pub fn global_main_instance() -> InstanceRecord {
    InstanceRecord {
        id: "global-main".to_string(),
        kind: "global_main".to_string(),
        workspace_root: "~".to_string(),
    }
}

pub fn workspace_instance(id: &str, workspace_root: &str) -> Result<InstanceRecord, String> {
    if id == "global-main" {
        return Err("workspace instance cannot use global-main id".to_string());
    }

    Ok(InstanceRecord {
        id: id.to_string(),
        kind: "workspace".to_string(),
        workspace_root: workspace_root.to_string(),
    })
}

pub fn default_instance_registry() -> InstanceRegistry {
    let mut registry = InstanceRegistry::new();
    registry
        .register(global_main_instance())
        .expect("default instance registry should contain global-main");
    registry
}

#[cfg(test)]
mod tests {
    use super::{default_instance_registry, global_main_instance, workspace_instance};

    #[test]
    fn contains_global_main_by_default() {
        let registry = default_instance_registry();
        assert_eq!(registry.list().len(), 1);
        assert_eq!(registry.list()[0].id, "global-main");
        assert_eq!(registry.list()[0].kind, "global_main");
    }

    #[test]
    fn allows_workspace_instance_registration() {
        let mut registry = default_instance_registry();
        let instance =
            workspace_instance("inst_project_a", "/workspace/project-a").expect("workspace instance should be valid");

        registry.register(instance).expect("workspace instance should register");

        assert_eq!(registry.list().len(), 2);
    }

    #[test]
    fn rejects_duplicate_global_main() {
        let mut registry = default_instance_registry();
        let error = registry
            .register(global_main_instance())
            .expect_err("duplicate global-main should be rejected");

        assert_eq!(error, "duplicate instance id: global-main");
    }

    #[test]
    fn rejects_workspace_impersonation_of_global_main() {
        let error =
            workspace_instance("global-main", "/workspace/fake").expect_err("global-main impersonation should fail");

        assert_eq!(error, "workspace instance cannot use global-main id");
    }
}
