#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityBoundary {
    pub capability_id: String,
    pub requires_approval: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionBoundary {
    pub scope: String,
    pub capabilities: Vec<CapabilityBoundary>,
}
