use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Installed,
    Enabled,
    Disabled,
    Removed,
    Broken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillChangeKind {
    Added,
    Enabled,
    Disabled,
    Removed,
    Updated,
    Broken,
    Repaired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillContextVisibility {
    ModelVisible,
    UserVisibleSummary,
    HiddenByPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillActivationMode {
    Manual,
    ConditionMatched,
    AlwaysAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillToolDependencyKind {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillPolicyViolation {
    ForbiddenToolDependency,
    InstanceSoulOverride,
    ApprovalOverride,
    SandboxOverride,
    MemoryWriteAttempt,
}
