use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crate::{KernelInvocationEnvelope, KernelRouteRuntimeOutput};

pub type KernelHandlerFn = Arc<
    dyn Fn(
            &KernelInvocationEnvelope,
            &KernelRouteRuntimeOutput,
        ) -> Result<KernelHandlerResult, KernelHandlerError>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandlerResult {
    pub summary: String,
    pub result_kind: Option<String>,
    pub public_fields: BTreeMap<String, String>,
}

impl KernelHandlerResult {
    pub fn summary(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            result_kind: Some("summary".to_string()),
            public_fields: BTreeMap::new(),
        }
    }

    pub fn structured(
        result_kind: impl Into<String>,
        public_fields: BTreeMap<String, String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            summary: summary.into(),
            result_kind: Some(result_kind.into()),
            public_fields,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandlerError {
    pub message: String,
}

impl KernelHandlerError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for KernelHandlerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for KernelHandlerError {}

#[derive(Clone)]
pub struct KernelHandlerRegistry {
    handlers: BTreeMap<String, KernelHandlerFn>,
}

impl KernelHandlerRegistry {
    pub fn new() -> Self {
        Self {
            handlers: BTreeMap::new(),
        }
    }

    pub fn with_handler<F>(mut self, operation: impl Into<String>, handler: F) -> Self
    where
        F: Fn(
                &KernelInvocationEnvelope,
                &KernelRouteRuntimeOutput,
            ) -> Result<KernelHandlerResult, KernelHandlerError>
            + Send
            + Sync
            + 'static,
    {
        self.register(operation, handler);
        self
    }

    pub fn register<F>(&mut self, operation: impl Into<String>, handler: F)
    where
        F: Fn(
                &KernelInvocationEnvelope,
                &KernelRouteRuntimeOutput,
            ) -> Result<KernelHandlerResult, KernelHandlerError>
            + Send
            + Sync
            + 'static,
    {
        self.handlers.insert(operation.into(), Arc::new(handler));
    }

    pub fn get(&self, operation: &str) -> Option<KernelHandlerFn> {
        self.handlers.get(operation).cloned()
    }
}

impl Default for KernelHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
