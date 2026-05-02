use aicore_foundation::{InstanceId, Timestamp};
use serde::{Deserialize, Serialize};

use super::{
    SandboxProfileId, ToolApprovalRequirement, ToolHotPlugChangeKind, ToolId, ToolModuleId,
    ToolNoticeId, ToolPermissionClass, ToolSchemaHash, ToolStatus, ToolVersion,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub tool_id: ToolId,
    pub module_id: ToolModuleId,
    pub version: ToolVersion,
    pub name: String,
    pub description_en: String,
    pub description_zh: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSchemaDescriptor {
    pub input_schema_summary: String,
    pub output_schema_summary: String,
    pub schema_hash: ToolSchemaHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSandboxPolicy {
    pub sandbox_profile_id: SandboxProfileId,
    pub filesystem_scope: String,
    pub command_scope: String,
    pub network_scope: String,
    pub max_output_bytes: u64,
}

impl ToolSandboxPolicy {
    pub fn readonly(profile_id: SandboxProfileId) -> Self {
        Self {
            sandbox_profile_id: profile_id,
            filesystem_scope: "readonly_workspace".to_string(),
            command_scope: "none".to_string(),
            network_scope: "none".to_string(),
            max_output_bytes: 4096,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRegistryEntry {
    pub descriptor: ToolDescriptor,
    pub schema: ToolSchemaDescriptor,
    pub status: ToolStatus,
    pub permission_class: ToolPermissionClass,
    pub approval_requirement: ToolApprovalRequirement,
    pub sandbox_policy: ToolSandboxPolicy,
    pub lock_version: u64,
    pub registry_revision: u64,
    pub updated_at: Timestamp,
}

impl ToolRegistryEntry {
    pub fn tool_id(&self) -> &ToolId {
        &self.descriptor.tool_id
    }

    pub fn is_visible(&self) -> bool {
        self.status == ToolStatus::Enabled
            && self.permission_class != ToolPermissionClass::Forbidden
            && !is_forbidden_tool_id(self.descriptor.tool_id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRegistrySnapshot {
    pub revision: u64,
    pub entries: Vec<ToolRegistryEntry>,
    pub created_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleToolCapability {
    pub tool_id: ToolId,
    pub name: String,
    pub description_en: String,
    pub compact_input_schema: String,
    pub output_summary_schema: String,
    pub permission_class: ToolPermissionClass,
    pub approval_required: bool,
    pub sandbox_profile_id: SandboxProfileId,
    pub schema_hash: ToolSchemaHash,
    pub version: ToolVersion,
}

impl VisibleToolCapability {
    pub fn from_entry(entry: &ToolRegistryEntry) -> Option<Self> {
        if !entry.is_visible() {
            return None;
        }
        Some(Self {
            tool_id: entry.descriptor.tool_id.clone(),
            name: entry.descriptor.name.clone(),
            description_en: entry.descriptor.description_en.clone(),
            compact_input_schema: entry.schema.input_schema_summary.clone(),
            output_summary_schema: entry.schema.output_schema_summary.clone(),
            permission_class: entry.permission_class,
            approval_required: entry.approval_requirement == ToolApprovalRequirement::Required,
            sandbox_profile_id: entry.sandbox_policy.sandbox_profile_id.clone(),
            schema_hash: entry.schema.schema_hash.clone(),
            version: entry.descriptor.version.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisibleCapabilitiesProjection {
    pub instance_id: InstanceId,
    pub registry_revision: u64,
    pub capabilities: Vec<VisibleToolCapability>,
    pub created_at: Timestamp,
}

impl VisibleCapabilitiesProjection {
    pub fn from_snapshot(
        instance_id: InstanceId,
        snapshot: &ToolRegistrySnapshot,
        created_at: Timestamp,
    ) -> Self {
        Self {
            instance_id,
            registry_revision: snapshot.revision,
            capabilities: snapshot
                .entries
                .iter()
                .filter_map(VisibleToolCapability::from_entry)
                .collect(),
            created_at,
        }
    }
}

pub fn forbidden_tool_ids() -> &'static [&'static str] {
    &[
        "event_query",
        "ledger_query",
        "self_evolution_query",
        "secret_read",
        "credential_export",
        "remote_deploy",
        "system_service_control",
        "destructive_git_auto_execution",
        "cross_instance_memory_search",
    ]
}

pub fn is_forbidden_tool_id(tool_id: &str) -> bool {
    forbidden_tool_ids().contains(&tool_id)
}

pub fn notice_id_for(tool_id: &ToolId, kind: ToolHotPlugChangeKind, revision: u64) -> ToolNoticeId {
    let suffix = match kind {
        ToolHotPlugChangeKind::Added => "added",
        ToolHotPlugChangeKind::Enabled => "enabled",
        ToolHotPlugChangeKind::Disabled => "disabled",
        ToolHotPlugChangeKind::Removed => "removed",
        ToolHotPlugChangeKind::SchemaChanged => "schema_changed",
        ToolHotPlugChangeKind::Broken => "broken",
        ToolHotPlugChangeKind::Repaired => "repaired",
    };
    ToolNoticeId::new(format!(
        "notice.{}.{}.{}",
        tool_id.as_str(),
        suffix,
        revision
    ))
    .expect("notice id uses safe tokens")
}
