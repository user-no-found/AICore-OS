use aicore_tool_protocol::{ToolId, is_forbidden_tool_id};
use serde::{Deserialize, Serialize};

use super::{
    SkillActivationMode, SkillId, SkillPolicyViolation, SkillStatus, SkillToolDependencyKind,
    SkillVersion,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillActivationCondition {
    pub mode: SkillActivationMode,
    pub summary_en: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillToolDependency {
    pub tool_id: ToolId,
    pub kind: SkillToolDependencyKind,
    pub authorizes_tool: bool,
}

impl SkillToolDependency {
    pub fn required(tool_id: ToolId) -> Self {
        Self {
            tool_id,
            kind: SkillToolDependencyKind::Required,
            authorizes_tool: false,
        }
    }

    pub fn optional(tool_id: ToolId) -> Self {
        Self {
            tool_id,
            kind: SkillToolDependencyKind::Optional,
            authorizes_tool: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillDescriptor {
    pub skill_id: SkillId,
    pub version: SkillVersion,
    pub display_name: String,
    pub description_en: String,
    pub source_path: String,
    pub model_instructions: String,
    pub activation_conditions: Vec<SkillActivationCondition>,
    pub required_tools: Vec<SkillToolDependency>,
    pub optional_tools: Vec<SkillToolDependency>,
    pub output_contract: String,
    pub safety_notes: Vec<String>,
    pub status: SkillStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillValidationOutcome {
    pub valid: bool,
    pub policy_violations: Vec<SkillPolicyViolation>,
}

pub fn validate_skill_descriptor(descriptor: &SkillDescriptor) -> SkillValidationOutcome {
    let mut policy_violations = Vec::new();
    if descriptor
        .required_tools
        .iter()
        .chain(descriptor.optional_tools.iter())
        .any(|dependency| is_forbidden_tool_id(dependency.tool_id.as_str()))
    {
        policy_violations.push(SkillPolicyViolation::ForbiddenToolDependency);
    }

    let text = format!(
        "{} {} {}",
        descriptor.model_instructions,
        descriptor.output_contract,
        descriptor.safety_notes.join(" ")
    )
    .to_ascii_lowercase();

    if text.contains("instance_soul") || text.contains("soul.md") {
        policy_violations.push(SkillPolicyViolation::InstanceSoulOverride);
    }
    if contains_forbidden_instruction(&text, "bypass approval")
        || contains_forbidden_instruction(&text, "auto-approve")
    {
        policy_violations.push(SkillPolicyViolation::ApprovalOverride);
    }
    if contains_forbidden_instruction(&text, "bypass sandbox")
        || contains_forbidden_instruction(&text, "skip sandbox")
    {
        policy_violations.push(SkillPolicyViolation::SandboxOverride);
    }
    if text.contains("write memory") || text.contains("memory_write") {
        policy_violations.push(SkillPolicyViolation::MemoryWriteAttempt);
    }

    SkillValidationOutcome {
        valid: policy_violations.is_empty(),
        policy_violations,
    }
}

fn contains_forbidden_instruction(text: &str, phrase: &str) -> bool {
    text.contains(phrase)
        && !text.contains(&format!("do not {phrase}"))
        && !text.contains(&format!("never {phrase}"))
}
