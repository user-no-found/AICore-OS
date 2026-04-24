use crate::LifecycleState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryKind {
    Component,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistrationRecord {
    pub registry_kind: RegistryKind,
    pub subject_id: String,
    pub lifecycle_state: LifecycleState,
}
