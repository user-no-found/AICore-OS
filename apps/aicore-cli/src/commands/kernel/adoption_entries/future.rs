use super::super::KernelInvocationAdoptionClass::MustMigrateToKernelInvocationLater;
use super::super::KernelInvocationAdoptionEntry;
use super::entry;

pub(super) const ENTRIES: &[KernelInvocationAdoptionEntry] = &[entry(
    "M3.x direct command adoption policy",
    "policy.direct_command_adoption",
    MustMigrateToKernelInvocationLater,
    false,
    false,
    false,
    false,
    false,
    false,
    true,
    "decide whether direct commands should default to kernel-native paths",
)];
