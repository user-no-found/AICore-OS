use std::path::PathBuf;

use aicore_foundation::InstanceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceKind {
    GlobalMain,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRecord {
    pub id: InstanceId,
    pub kind: InstanceKind,
    pub workspace_root: PathBuf,
    pub state_root: PathBuf,
}
