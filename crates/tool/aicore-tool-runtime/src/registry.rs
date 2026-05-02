use std::collections::BTreeMap;

use aicore_foundation::{InstanceId, Timestamp};
use aicore_tool_protocol::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolRuntimeError {
    DuplicateTool(String),
    ToolNotFound(String),
    ForbiddenTool(String),
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryToolRegistry {
    revision: u64,
    entries: BTreeMap<String, ToolRegistryEntry>,
}

impl InMemoryToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn register_tool(
        &mut self,
        mut entry: ToolRegistryEntry,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        let tool_id = entry.descriptor.tool_id.as_str().to_string();
        if is_forbidden_tool_id(&tool_id)
            || entry.permission_class == ToolPermissionClass::Forbidden
        {
            return Err(ToolRuntimeError::ForbiddenTool(tool_id));
        }
        if self.entries.contains_key(&tool_id) {
            return Err(ToolRuntimeError::DuplicateTool(tool_id));
        }
        self.revision += 1;
        entry.registry_revision = self.revision;
        entry.lock_version = entry.lock_version.max(1);
        entry.updated_at = now;
        let notice = self.notice_for(&entry.descriptor.tool_id, ToolHotPlugChangeKind::Added, now);
        self.entries.insert(tool_id, entry);
        Ok(notice)
    }

    pub fn enable_tool(
        &mut self,
        tool_id: &ToolId,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        self.set_status(
            tool_id,
            ToolStatus::Enabled,
            ToolHotPlugChangeKind::Enabled,
            now,
        )
    }

    pub fn disable_tool(
        &mut self,
        tool_id: &ToolId,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        self.set_status(
            tool_id,
            ToolStatus::Disabled,
            ToolHotPlugChangeKind::Disabled,
            now,
        )
    }

    pub fn remove_tool(
        &mut self,
        tool_id: &ToolId,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        self.set_status(
            tool_id,
            ToolStatus::Removed,
            ToolHotPlugChangeKind::Removed,
            now,
        )
    }

    pub fn mark_broken(
        &mut self,
        tool_id: &ToolId,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        self.set_status(
            tool_id,
            ToolStatus::Broken,
            ToolHotPlugChangeKind::Broken,
            now,
        )
    }

    pub fn get_tool(&self, tool_id: &ToolId) -> Option<&ToolRegistryEntry> {
        self.entries.get(tool_id.as_str())
    }

    pub fn snapshot(&self, created_at: Timestamp) -> ToolRegistrySnapshot {
        ToolRegistrySnapshot {
            revision: self.revision,
            entries: self.entries.values().cloned().collect(),
            created_at,
        }
    }

    pub fn project_visible_capabilities(
        &self,
        instance_id: InstanceId,
        created_at: Timestamp,
    ) -> VisibleCapabilitiesProjection {
        let snapshot = self.snapshot(created_at);
        VisibleCapabilitiesProjection::from_snapshot(instance_id, &snapshot, created_at)
    }

    pub fn generate_hot_plug_notice(
        &self,
        tool_id: &ToolId,
        kind: ToolHotPlugChangeKind,
        now: Timestamp,
    ) -> ToolHotPlugNotice {
        self.notice_for(tool_id, kind, now)
    }

    fn set_status(
        &mut self,
        tool_id: &ToolId,
        status: ToolStatus,
        kind: ToolHotPlugChangeKind,
        now: Timestamp,
    ) -> Result<ToolHotPlugNotice, ToolRuntimeError> {
        if is_forbidden_tool_id(tool_id.as_str()) {
            return Err(ToolRuntimeError::ForbiddenTool(
                tool_id.as_str().to_string(),
            ));
        }
        let entry = self
            .entries
            .get_mut(tool_id.as_str())
            .ok_or_else(|| ToolRuntimeError::ToolNotFound(tool_id.as_str().to_string()))?;
        self.revision += 1;
        entry.status = status;
        entry.registry_revision = self.revision;
        entry.lock_version += 1;
        entry.updated_at = now;
        Ok(self.notice_for(tool_id, kind, now))
    }

    fn notice_for(
        &self,
        tool_id: &ToolId,
        kind: ToolHotPlugChangeKind,
        now: Timestamp,
    ) -> ToolHotPlugNotice {
        let available = matches!(
            kind,
            ToolHotPlugChangeKind::Added
                | ToolHotPlugChangeKind::Enabled
                | ToolHotPlugChangeKind::Repaired
        );
        let message_en = if available {
            format!(
                "Tool {} is now available for future turns.",
                tool_id.as_str()
            )
        } else {
            format!(
                "Tool {} is not available for future calls.",
                tool_id.as_str()
            )
        };
        let message_zh = if available {
            Some(format!("工具 {} 已可用于后续回合。", tool_id.as_str()))
        } else {
            Some(format!("工具 {} 后续不可用。", tool_id.as_str()))
        };
        ToolHotPlugNotice::new(
            notice_id_for(tool_id, kind, self.revision.max(1)),
            tool_id.clone(),
            kind,
            now,
            message_en,
            message_zh,
        )
    }
}
