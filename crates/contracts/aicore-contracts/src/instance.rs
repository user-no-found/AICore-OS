#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRecord {
    pub id: String,
    pub kind: String,
    pub workspace_root: String,
}
