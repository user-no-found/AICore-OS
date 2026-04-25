use aicore_contracts::{HealthLevel, HealthStatus, LifecycleState};

use crate::component_registry::{AppSummary, ComponentRegistry};
use crate::instance_registry::InstanceRegistry;

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
                aicore_contracts::InstanceKind::GlobalMain => "global_main".to_string(),
                aicore_contracts::InstanceKind::Workspace => "workspace".to_string(),
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

pub fn default_control_plane() -> ControlPlane {
    ControlPlane::new(
        crate::default_component_registry(),
        crate::default_instance_registry(),
    )
}

#[cfg(test)]
mod tests {
    use super::ControlPlane;
    use crate::{default_component_registry, default_instance_registry};
    use aicore_contracts::LifecycleState;

    #[test]
    fn reports_control_plane_health() {
        let plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        let health = plane.health_status();

        assert_eq!(health.summary_zh, "控制内核骨架可用");
    }

    #[test]
    fn updates_control_plane_lifecycle() {
        let mut plane =
            ControlPlane::new(default_component_registry(), default_instance_registry());
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Registered);

        plane.install();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Installed);

        plane.start();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Running);

        plane.stop();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Stopped);
    }

    #[test]
    fn exposes_control_plane_summary() {
        let plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        let summary = plane.summary();

        assert_eq!(summary.component_count, 3);
        assert_eq!(summary.instance_count, 1);
        assert_eq!(summary.lifecycle_state, LifecycleState::Registered);
    }

    #[test]
    fn exposes_main_instance_summary() {
        let plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        let summary = plane.main_instance_summary();

        assert_eq!(summary.id, "global-main");
        assert_eq!(summary.kind, "global_main");
        assert!(!summary.workspace_root.is_empty());
        assert!(!summary.state_root.is_empty());
    }
}
