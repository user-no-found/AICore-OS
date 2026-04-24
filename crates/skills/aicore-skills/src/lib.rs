#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillScope {
    Builtin,
    Global,
    GlobalMainPrivate,
    Instance { instance_id: String },
    Task { task_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStatus {
    Draft,
    Candidate,
    Active,
    Deprecated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillRecord {
    pub id: String,
    pub scope: SkillScope,
    pub status: SkillStatus,
    pub owner: String,
    pub delete_with_instance: bool,
}

pub fn default_skill_records() -> Vec<SkillRecord> {
    vec![
        SkillRecord {
            id: "skill.git.basic".to_string(),
            scope: SkillScope::Global,
            status: SkillStatus::Active,
            owner: "global-skill-registry".to_string(),
            delete_with_instance: false,
        },
        SkillRecord {
            id: "skill.project.notes".to_string(),
            scope: SkillScope::Instance {
                instance_id: "inst_project_a".to_string(),
            },
            status: SkillStatus::Candidate,
            owner: "inst_project_a".to_string(),
            delete_with_instance: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::{default_skill_records, SkillScope, SkillStatus};

    #[test]
    fn global_skill_does_not_delete_with_instance() {
        let skills = default_skill_records();
        let global = skills
            .iter()
            .find(|skill| skill.id == "skill.git.basic")
            .expect("global skill should exist");

        assert_eq!(global.scope, SkillScope::Global);
        assert!(!global.delete_with_instance);
    }

    #[test]
    fn instance_skill_deletes_with_instance() {
        let skills = default_skill_records();
        let instance_skill = skills
            .iter()
            .find(|skill| skill.id == "skill.project.notes")
            .expect("instance skill should exist");

        assert!(instance_skill.delete_with_instance);
    }

    #[test]
    fn new_skill_can_start_as_draft_or_candidate() {
        let skills = default_skill_records();
        let instance_skill = skills
            .iter()
            .find(|skill| skill.id == "skill.project.notes")
            .expect("instance skill should exist");

        assert_eq!(instance_skill.status, SkillStatus::Candidate);
    }
}
