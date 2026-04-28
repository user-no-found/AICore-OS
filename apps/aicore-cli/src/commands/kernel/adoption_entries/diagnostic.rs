use super::super::KernelInvocationAdoptionClass::KernelDiagnostic;
use super::super::KernelInvocationAdoptionEntry;
use super::entry;

pub(super) const ENTRIES: &[KernelInvocationAdoptionEntry] = &[
    entry(
        "aicore-cli kernel route <operation>",
        "<operation>",
        KernelDiagnostic,
        true,
        true,
        false,
        false,
        false,
        true,
        false,
        "route decision diagnostic",
    ),
    entry(
        "aicore-cli kernel invoke-smoke <operation>",
        "<operation>",
        KernelDiagnostic,
        true,
        true,
        true,
        true,
        false,
        true,
        false,
        "dispatcher and ledger diagnostic",
    ),
];
