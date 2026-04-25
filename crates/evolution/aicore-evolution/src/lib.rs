#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionTarget {
    Tool,
    Prompt,
    Skill,
    Soul,
    SecurityPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionMode {
    Suggest,
    Draft,
    Review,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvolutionProposal {
    pub id: String,
    pub target: EvolutionTarget,
    pub mode: EvolutionMode,
    pub summary: String,
}

impl EvolutionProposal {
    pub fn requires_user_discussion(&self) -> bool {
        matches!(
            self.target,
            EvolutionTarget::Tool
                | EvolutionTarget::Prompt
                | EvolutionTarget::Soul
                | EvolutionTarget::SecurityPolicy
        )
    }
}

pub fn default_evolution_proposals() -> Vec<EvolutionProposal> {
    vec![
        EvolutionProposal {
            id: "evo_tool_001".to_string(),
            target: EvolutionTarget::Tool,
            mode: EvolutionMode::Suggest,
            summary: "Add safer read window behavior for large file reads.".to_string(),
        },
        EvolutionProposal {
            id: "evo_skill_001".to_string(),
            target: EvolutionTarget::Skill,
            mode: EvolutionMode::Draft,
            summary: "Draft a reusable project note summarization skill.".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{EvolutionMode, EvolutionProposal, EvolutionTarget, default_evolution_proposals};

    #[test]
    fn high_risk_targets_require_user_discussion() {
        for target in [
            EvolutionTarget::Prompt,
            EvolutionTarget::Tool,
            EvolutionTarget::Soul,
            EvolutionTarget::SecurityPolicy,
        ] {
            let proposal = EvolutionProposal {
                id: "evo".to_string(),
                target,
                mode: EvolutionMode::Suggest,
                summary: "high risk change".to_string(),
            };

            assert!(proposal.requires_user_discussion());
        }
    }

    #[test]
    fn skill_proposal_can_be_draft() {
        let proposals = default_evolution_proposals();
        let skill = proposals
            .iter()
            .find(|proposal| proposal.id == "evo_skill_001")
            .expect("skill proposal should exist");

        assert_eq!(skill.target, EvolutionTarget::Skill);
        assert_eq!(skill.mode, EvolutionMode::Draft);
        assert!(!skill.requires_user_discussion());
    }
}
