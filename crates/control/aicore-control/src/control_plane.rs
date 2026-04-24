use aicore_contracts::{HealthLevel, HealthStatus, LifecycleState};
use aicore_evolution::{default_evolution_proposals, EvolutionProposal, EvolutionTarget};
use aicore_memory::{default_memory_kernel, MemoryProposal, MemoryType};
use aicore_skills::{default_skill_records, SkillRecord, SkillScope};
use aicore_tools::{default_tool_broker, ToolDescriptor};

use crate::component_registry::{AppSummary, ComponentRegistry};
use crate::instance_registry::InstanceRegistry;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSummary {
    pub id: String,
    pub toolset: String,
    pub display_name_zh: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryProposalTypeView {
    Core,
    Permanent,
    Working,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummary {
    pub id: String,
    pub memory_type: MemoryProposalTypeView,
    pub normalized_memory: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillScopeView {
    Builtin,
    Global,
    GlobalMainPrivate,
    Instance,
    Task,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSummary {
    pub id: String,
    pub scope: SkillScopeView,
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionTargetView {
    Tool,
    Prompt,
    Skill,
    Soul,
    SecurityPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionSummary {
    pub id: String,
    pub target: EvolutionTargetView,
    pub requires_user_discussion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelSurface {
    pub tools: Vec<ToolSummary>,
    pub memories: Vec<MemorySummary>,
    pub skills: Vec<SkillSummary>,
    pub evolution_proposals: Vec<EvolutionSummary>,
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
            .list()
            .iter()
            .find(|item| item.id == "global-main")
            .expect("global-main must exist in the default control-plane registry");

        MainInstanceSummary {
            id: instance.id.clone(),
            kind: instance.kind.clone(),
            workspace_root: instance.workspace_root.clone(),
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

    pub fn kernel_surface(
        &self,
        tools: &[ToolDescriptor],
        memories: &[MemoryProposal],
        skills: &[SkillRecord],
        evolution_proposals: &[EvolutionProposal],
    ) -> KernelSurface {
        KernelSurface {
            tools: tools
                .iter()
                .map(|tool| ToolSummary {
                    id: tool.id.clone(),
                    toolset: tool.toolset.clone(),
                    display_name_zh: tool.display_name_zh.clone(),
                    revision: tool.revision,
                })
                .collect(),
            memories: memories
                .iter()
                .map(|memory| MemorySummary {
                    id: memory.id.clone(),
                    memory_type: match memory.memory_type {
                        MemoryType::Core => MemoryProposalTypeView::Core,
                        MemoryType::Permanent { .. } => MemoryProposalTypeView::Permanent,
                        MemoryType::Working => MemoryProposalTypeView::Working,
                    },
                    normalized_memory: memory.normalized_memory.clone(),
                })
                .collect(),
            skills: skills
                .iter()
                .map(|skill| SkillSummary {
                    id: skill.id.clone(),
                    scope: match &skill.scope {
                        SkillScope::Builtin => SkillScopeView::Builtin,
                        SkillScope::Global => SkillScopeView::Global,
                        SkillScope::GlobalMainPrivate => SkillScopeView::GlobalMainPrivate,
                        SkillScope::Instance { .. } => SkillScopeView::Instance,
                        SkillScope::Task { .. } => SkillScopeView::Task,
                    },
                    owner: skill.owner.clone(),
                })
                .collect(),
            evolution_proposals: evolution_proposals
                .iter()
                .map(|proposal| EvolutionSummary {
                    id: proposal.id.clone(),
                    target: match proposal.target {
                        EvolutionTarget::Tool => EvolutionTargetView::Tool,
                        EvolutionTarget::Prompt => EvolutionTargetView::Prompt,
                        EvolutionTarget::Skill => EvolutionTargetView::Skill,
                        EvolutionTarget::Soul => EvolutionTargetView::Soul,
                        EvolutionTarget::SecurityPolicy => EvolutionTargetView::SecurityPolicy,
                    },
                    requires_user_discussion: proposal.requires_user_discussion(),
                })
                .collect(),
        }
    }

    pub fn default_kernel_surface(&self) -> KernelSurface {
        let tool_broker = default_tool_broker();
        let memory_kernel = default_memory_kernel();
        let skills = default_skill_records();
        let evolution = default_evolution_proposals();

        self.kernel_surface(
            tool_broker.list(),
            memory_kernel.proposals(),
            &skills,
            &evolution,
        )
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
    use aicore_contracts::LifecycleState;
    use crate::{default_component_registry, default_instance_registry};

    #[test]
    fn reports_control_plane_health() {
        let plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        let health = plane.health_status();

        assert_eq!(health.summary_zh, "控制内核骨架可用");
    }

    #[test]
    fn updates_control_plane_lifecycle() {
        let mut plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Registered);

        plane.install();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Installed);

        plane.start();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Running);

        plane.stop();
        assert_eq!(plane.lifecycle_state(), &LifecycleState::Stopped);
    }

    #[test]
    fn exposes_kernel_surface_summary() {
        let plane = ControlPlane::new(default_component_registry(), default_instance_registry());
        let surface = plane.default_kernel_surface();

        assert_eq!(surface.tools.len(), 2);
        assert_eq!(surface.memories.len(), 1);
        assert_eq!(surface.skills.len(), 2);
        assert_eq!(surface.evolution_proposals.len(), 2);
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
        assert_eq!(summary.workspace_root, "~");
    }
}
