use std::path::PathBuf;

use aicore_foundation::InstanceId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceKind {
    GlobalMain,
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRecord {
    pub id: InstanceId,
    pub kind: InstanceKind,
    pub workspace_root: PathBuf,
    pub state_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceScope {
    GlobalMain,
    Workspace(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceContext {
    pub instance_id: String,
    pub kind: InstanceKind,
    pub scope: InstanceScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionContext {
    pub session_id: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    pub trace_id: String,
    pub parent_span_id: Option<String>,
}

impl TraceContext {
    pub fn new(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            parent_span_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditContext {
    pub actor: String,
    pub reason: String,
}

impl AuditContext {
    pub fn system(reason: impl Into<String>) -> Self {
        Self {
            actor: "system".to_string(),
            reason: reason.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Internal,
    User,
    Audit,
}
