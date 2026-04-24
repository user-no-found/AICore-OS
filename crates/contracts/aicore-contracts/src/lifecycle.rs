#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleState {
    Registered,
    Installed,
    Running,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallAction {
    Install,
    Upgrade,
    Remove,
}
