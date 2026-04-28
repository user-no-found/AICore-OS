use super::{KernelInvocationAdoptionClass, KernelInvocationAdoptionEntry};

mod diagnostic;
mod direct;
mod future;
mod kernel_native;
mod workflow;

pub(crate) fn adoption_entries() -> Vec<KernelInvocationAdoptionEntry> {
    let mut entries = Vec::new();
    entries.extend_from_slice(diagnostic::ENTRIES);
    entries.extend_from_slice(kernel_native::ENTRIES);
    entries.extend_from_slice(direct::ENTRIES);
    entries.extend_from_slice(future::ENTRIES);
    entries.extend_from_slice(workflow::ENTRIES);
    entries
}

const fn entry(
    command: &'static str,
    operation: &'static str,
    class: KernelInvocationAdoptionClass,
    manifest_capability_exists: bool,
    route_runtime_used: bool,
    invocation_runtime_used: bool,
    ledger_used: bool,
    structured_result_envelope_used: bool,
    direct_local_execution_allowed_for_now: bool,
    future_migration_required: bool,
    reason: &'static str,
) -> KernelInvocationAdoptionEntry {
    KernelInvocationAdoptionEntry {
        command,
        operation,
        class,
        manifest_capability_exists,
        route_runtime_used,
        invocation_runtime_used,
        ledger_used,
        structured_result_envelope_used,
        direct_local_execution_allowed_for_now,
        future_migration_required,
        reason,
    }
}
