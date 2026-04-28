#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KernelInvocationAdoptionClass {
    KernelNativeNow,
    KernelDiagnostic,
    AllowedLocalDirectCommand,
    MustMigrateToKernelInvocationLater,
    NotKernelInvocationTarget,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KernelInvocationAdoptionEntry {
    pub(crate) command: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) class: KernelInvocationAdoptionClass,
    pub(crate) manifest_capability_exists: bool,
    pub(crate) route_runtime_used: bool,
    pub(crate) invocation_runtime_used: bool,
    pub(crate) ledger_used: bool,
    pub(crate) structured_result_envelope_used: bool,
    pub(crate) direct_local_execution_allowed_for_now: bool,
    pub(crate) future_migration_required: bool,
    pub(crate) reason: &'static str,
}

#[cfg(test)]
mod adoption_entries;

#[cfg(test)]
pub(crate) fn kernel_invocation_adoption_matrix() -> Vec<KernelInvocationAdoptionEntry> {
    adoption_entries::adoption_entries()
}
