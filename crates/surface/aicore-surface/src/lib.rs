use aicore_evolution::{EvolutionProposal, EvolutionTarget, default_evolution_proposals};
use aicore_memory::{MemoryProposal, MemoryType, default_memory_kernel};
use aicore_skills::{SkillRecord, SkillScope, default_skill_records};
use aicore_tools::{ToolDescriptor, default_tool_broker};

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

pub fn build_kernel_surface(
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

pub fn default_kernel_surface() -> KernelSurface {
    let tool_broker = default_tool_broker();
    let memory_kernel = default_memory_kernel();
    let skills = default_skill_records();
    let evolution = default_evolution_proposals();

    build_kernel_surface(
        tool_broker.list(),
        memory_kernel.proposals(),
        &skills,
        &evolution,
    )
}

#[cfg(test)]
mod tests {
    use super::default_kernel_surface;

    #[test]
    fn exposes_default_kernel_surface_summary() {
        let surface = default_kernel_surface();
        assert_eq!(surface.tools.len(), 2);
        assert_eq!(surface.memories.len(), 1);
        assert_eq!(surface.skills.len(), 2);
        assert_eq!(surface.evolution_proposals.len(), 2);
    }
}
