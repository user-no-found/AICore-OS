use std::collections::BTreeMap;

use aicore_foundation::{InstanceId, Timestamp};
use aicore_tool_protocol::{ToolId, ToolRegistryRevision};

use crate::{
    SkillChangeKind, SkillChangeNotice, SkillContextProjection, SkillDescriptor, SkillId,
    SkillNoticeId, SkillRegistryEntry, SkillRegistryRevision, SkillRegistrySnapshot, SkillStatus,
    project_skill_context, validate_skill_descriptor,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillRegistryError {
    DuplicateSkill(String),
    SkillNotFound(String),
    PolicyViolation(String),
}

#[derive(Debug, Clone)]
pub struct InMemorySkillRegistry {
    instance_id: InstanceId,
    revision: u64,
    entries: BTreeMap<String, SkillRegistryEntry>,
}

impl InMemorySkillRegistry {
    pub fn new(instance_id: InstanceId) -> Self {
        Self {
            instance_id,
            revision: 0,
            entries: BTreeMap::new(),
        }
    }

    pub fn register_skill(
        &mut self,
        descriptor: SkillDescriptor,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        let skill_id = descriptor.skill_id.as_str().to_string();
        if self.entries.contains_key(&skill_id) {
            return Err(SkillRegistryError::DuplicateSkill(skill_id));
        }
        let validation = validate_skill_descriptor(&descriptor);
        if !validation.valid {
            return Err(SkillRegistryError::PolicyViolation(skill_id));
        }
        self.revision += 1;
        self.entries
            .insert(skill_id, SkillRegistryEntry::new(descriptor.clone(), now));
        Ok(self.notice_for(&descriptor.skill_id, SkillChangeKind::Added, now))
    }

    pub fn enable_skill(
        &mut self,
        skill_id: &SkillId,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        self.set_status(
            skill_id,
            SkillStatus::Enabled,
            SkillChangeKind::Enabled,
            now,
        )
    }

    pub fn disable_skill(
        &mut self,
        skill_id: &SkillId,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        self.set_status(
            skill_id,
            SkillStatus::Disabled,
            SkillChangeKind::Disabled,
            now,
        )
    }

    pub fn remove_skill(
        &mut self,
        skill_id: &SkillId,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        self.set_status(
            skill_id,
            SkillStatus::Removed,
            SkillChangeKind::Removed,
            now,
        )
    }

    pub fn mark_broken(
        &mut self,
        skill_id: &SkillId,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        self.set_status(skill_id, SkillStatus::Broken, SkillChangeKind::Broken, now)
    }

    pub fn snapshot(&self, created_at: Timestamp) -> SkillRegistrySnapshot {
        SkillRegistrySnapshot {
            instance_id: self.instance_id.clone(),
            revision: revision_id(self.revision),
            entries: self.entries.values().cloned().collect(),
            created_at,
        }
    }

    pub fn project_skill_context(
        &self,
        available_tools: &[ToolId],
        tool_registry_revision: ToolRegistryRevision,
        created_at: Timestamp,
    ) -> SkillContextProjection {
        let snapshot = self.snapshot(created_at);
        project_skill_context(
            &snapshot,
            available_tools,
            tool_registry_revision,
            created_at,
        )
    }

    pub fn generate_skill_change_notice(
        &self,
        skill_id: &SkillId,
        kind: SkillChangeKind,
        now: Timestamp,
    ) -> SkillChangeNotice {
        self.notice_for(skill_id, kind, now)
    }

    fn set_status(
        &mut self,
        skill_id: &SkillId,
        status: SkillStatus,
        kind: SkillChangeKind,
        now: Timestamp,
    ) -> Result<SkillChangeNotice, SkillRegistryError> {
        let entry = self
            .entries
            .get_mut(skill_id.as_str())
            .ok_or_else(|| SkillRegistryError::SkillNotFound(skill_id.as_str().to_string()))?;
        self.revision += 1;
        entry.status = status;
        entry.descriptor.status = status;
        entry.updated_at = now;
        Ok(self.notice_for(skill_id, kind, now))
    }

    fn notice_for(
        &self,
        skill_id: &SkillId,
        kind: SkillChangeKind,
        now: Timestamp,
    ) -> SkillChangeNotice {
        let suffix = match kind {
            SkillChangeKind::Added => "added",
            SkillChangeKind::Enabled => "enabled",
            SkillChangeKind::Disabled => "disabled",
            SkillChangeKind::Removed => "removed",
            SkillChangeKind::Updated => "updated",
            SkillChangeKind::Broken => "broken",
            SkillChangeKind::Repaired => "repaired",
        };
        SkillChangeNotice::new(
            SkillNoticeId::new(format!(
                "notice.{}.{}.{}",
                skill_id.as_str(),
                suffix,
                self.revision.max(1)
            ))
            .expect("notice id uses safe tokens"),
            skill_id.clone(),
            kind,
            now,
            format!("Skill {} changed for future turns.", skill_id.as_str()),
            Some(format!(
                "技能 {} 已更新，将影响后续回合。",
                skill_id.as_str()
            )),
        )
    }
}

fn revision_id(revision: u64) -> SkillRegistryRevision {
    SkillRegistryRevision::new(format!("skillrev.{revision}"))
        .expect("revision id uses safe tokens")
}
