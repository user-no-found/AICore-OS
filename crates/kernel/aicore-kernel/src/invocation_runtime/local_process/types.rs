use std::process::ExitStatus;

use crate::KernelHandlerResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::invocation_runtime) struct ComponentProcessSuccess {
    pub(crate) result: KernelHandlerResult,
    pub(crate) exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::invocation_runtime) struct ComponentProcessFailure {
    pub(crate) stage: String,
    pub(crate) reason: String,
    pub(crate) result: Option<KernelHandlerResult>,
    pub(crate) spawned_process: bool,
    pub(crate) exit_code: Option<i32>,
}

pub(super) struct ComponentProcessOutput {
    pub(super) status: ExitStatus,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}
