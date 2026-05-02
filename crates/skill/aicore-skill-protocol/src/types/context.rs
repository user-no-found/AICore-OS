use aicore_foundation::{InstanceId, Timestamp};
use aicore_tool_protocol::{ToolId, ToolRegistryRevision};
use serde::{Deserialize, Serialize};

use super::{
    SkillContextId, SkillContextVisibility, SkillDescriptor, SkillId, SkillRegistryRevision,
    SkillStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRegistryEntry {
    pub descriptor: SkillDescriptor,
    pub status: SkillStatus,
    pub unavailable_reason: Option<String>,
    pub updated_at: Timestamp,
}

impl SkillRegistryEntry {
    pub fn new(descriptor: SkillDescriptor, updated_at: Timestamp) -> Self {
        Self {
            status: descriptor.status,
            descriptor,
            unavailable_reason: None,
            updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillRegistrySnapshot {
    pub instance_id: InstanceId,
    pub revision: SkillRegistryRevision,
    pub entries: Vec<SkillRegistryEntry>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillContextModule {
    pub skill_id: SkillId,
    pub version: String,
    pub visibility: SkillContextVisibility,
    pub available: bool,
    pub instructions_en: String,
    pub output_contract: String,
    pub safety_notes: Vec<String>,
    pub source_path: String,
    pub required_tools: Vec<ToolId>,
    pub optional_tools: Vec<ToolId>,
    pub missing_required_tools: Vec<ToolId>,
    pub missing_optional_tools: Vec<ToolId>,
    pub grants_tool_access: Vec<ToolId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillContextProjection {
    pub context_id: SkillContextId,
    pub instance_id: InstanceId,
    pub registry_revision: SkillRegistryRevision,
    pub tool_registry_revision: ToolRegistryRevision,
    pub modules: Vec<SkillContextModule>,
    pub created_at: Timestamp,
}

pub fn project_skill_context(
    snapshot: &SkillRegistrySnapshot,
    available_tools: &[ToolId],
    tool_registry_revision: ToolRegistryRevision,
    created_at: Timestamp,
) -> SkillContextProjection {
    let modules = snapshot
        .entries
        .iter()
        .filter(|entry| entry.status == SkillStatus::Enabled)
        .map(|entry| module_from_entry(entry, available_tools))
        .collect();

    SkillContextProjection {
        context_id: SkillContextId::new(format!(
            "skillctx.{}.{}",
            snapshot.instance_id.as_str(),
            created_at.unix_millis()
        ))
        .expect("context id uses safe tokens"),
        instance_id: snapshot.instance_id.clone(),
        registry_revision: snapshot.revision.clone(),
        tool_registry_revision,
        modules,
        created_at,
    }
}

fn module_from_entry(entry: &SkillRegistryEntry, available_tools: &[ToolId]) -> SkillContextModule {
    let required_tools: Vec<_> = entry
        .descriptor
        .required_tools
        .iter()
        .map(|dependency| dependency.tool_id.clone())
        .collect();
    let optional_tools: Vec<_> = entry
        .descriptor
        .optional_tools
        .iter()
        .map(|dependency| dependency.tool_id.clone())
        .collect();
    let missing_required_tools = missing(&required_tools, available_tools);
    let missing_optional_tools = missing(&optional_tools, available_tools);
    let available = missing_required_tools.is_empty();

    SkillContextModule {
        skill_id: entry.descriptor.skill_id.clone(),
        version: entry.descriptor.version.as_str().to_string(),
        visibility: if available {
            SkillContextVisibility::ModelVisible
        } else {
            SkillContextVisibility::HiddenByPolicy
        },
        available,
        instructions_en: if available {
            entry.descriptor.model_instructions.clone()
        } else {
            String::new()
        },
        output_contract: if available {
            entry.descriptor.output_contract.clone()
        } else {
            String::new()
        },
        safety_notes: if available {
            entry.descriptor.safety_notes.clone()
        } else {
            Vec::new()
        },
        source_path: entry.descriptor.source_path.clone(),
        required_tools,
        optional_tools,
        missing_required_tools,
        missing_optional_tools,
        grants_tool_access: Vec::new(),
    }
}

fn missing(declared: &[ToolId], available_tools: &[ToolId]) -> Vec<ToolId> {
    declared
        .iter()
        .filter(|tool_id| {
            !available_tools
                .iter()
                .any(|available| available == *tool_id)
        })
        .cloned()
        .collect()
}
