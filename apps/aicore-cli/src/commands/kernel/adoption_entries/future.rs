use super::super::KernelInvocationAdoptionClass::MustMigrateToKernelInvocationLater;
use super::super::KernelInvocationAdoptionEntry;
use super::entry;

pub(super) const ENTRIES: &[KernelInvocationAdoptionEntry] = &[
    entry(
        "aicore-cli memory remember <内容>",
        "memory.remember",
        MustMigrateToKernelInvocationLater,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        "memory write capability needs memory contract and audit boundary",
    ),
    entry(
        "aicore-cli memory accept <proposal_id>",
        "memory.accept",
        MustMigrateToKernelInvocationLater,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        "memory write capability needs memory contract and audit boundary",
    ),
    entry(
        "aicore-cli memory reject <proposal_id>",
        "memory.reject",
        MustMigrateToKernelInvocationLater,
        false,
        false,
        false,
        false,
        false,
        false,
        true,
        "memory write capability needs memory contract and audit boundary",
    ),
];
