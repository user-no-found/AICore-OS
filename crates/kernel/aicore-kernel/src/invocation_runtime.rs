use std::collections::BTreeMap;
use std::fmt;

use crate::{
    InstalledManifestRegistry, KernelEventEnvelope, KernelEventPayload, KernelEventType,
    KernelInvocationEnvelope, KernelRouteRuntime, KernelRouteRuntimeError, KernelRouteRuntimeInput,
    KernelRouteRuntimeOutput, Visibility,
};

pub type KernelHandlerFn = fn(
    &KernelInvocationEnvelope,
    &KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelHandlerResult {
    pub summary: String,
}

impl KernelHandlerResult {
    pub fn summary(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
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

    pub fn with_handler(mut self, operation: impl Into<String>, handler: KernelHandlerFn) -> Self {
        self.register(operation, handler);
        self
    }

    pub fn register(&mut self, operation: impl Into<String>, handler: KernelHandlerFn) {
        self.handlers.insert(operation.into(), handler);
    }

    pub fn get(&self, operation: &str) -> Option<KernelHandlerFn> {
        self.handlers.get(operation).copied()
    }
}

impl Default for KernelHandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelInvocationStatus {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelInvocationRuntimeOutput {
    pub status: KernelInvocationStatus,
    pub route: Option<KernelRouteRuntimeOutput>,
    pub event: Option<KernelEventEnvelope>,
    pub route_decision_made: bool,
    pub handler_executed: bool,
    pub event_generated: bool,
    pub handler_kind: Option<String>,
    pub failure_stage: Option<String>,
    pub failure_reason: Option<String>,
    pub spawned_process: bool,
    pub called_real_component: bool,
}

#[derive(Clone)]
pub struct KernelInvocationRuntime {
    route_runtime: KernelRouteRuntime,
    handlers: KernelHandlerRegistry,
}

impl KernelInvocationRuntime {
    pub fn new(registry: InstalledManifestRegistry, handlers: KernelHandlerRegistry) -> Self {
        Self {
            route_runtime: KernelRouteRuntime::from_registry(registry),
            handlers,
        }
    }

    pub fn invoke(&self, envelope: KernelInvocationEnvelope) -> KernelInvocationRuntimeOutput {
        let route = match self.route_runtime.route(
            KernelRouteRuntimeInput::new(envelope.operation.clone())
                .with_instance_id(envelope.instance_id.clone()),
        ) {
            Ok(route) => route,
            Err(error) => return Self::route_failure(error),
        };

        let Some(handler) = self.handlers.get(&envelope.operation) else {
            return Self::missing_handler(route);
        };

        match handler(&envelope, &route) {
            Ok(result) => Self::completed(envelope, route, result),
            Err(error) => Self::handler_failure(route, error),
        }
    }

    fn route_failure(error: KernelRouteRuntimeError) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: None,
            event: None,
            route_decision_made: false,
            handler_executed: false,
            event_generated: false,
            handler_kind: None,
            failure_stage: Some("route".to_string()),
            failure_reason: Some(error.to_string()),
            spawned_process: false,
            called_real_component: false,
        }
    }

    fn missing_handler(route: KernelRouteRuntimeOutput) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route),
            event: None,
            route_decision_made: true,
            handler_executed: false,
            event_generated: false,
            handler_kind: None,
            failure_stage: Some("handler_lookup".to_string()),
            failure_reason: Some("missing handler for operation".to_string()),
            spawned_process: false,
            called_real_component: false,
        }
    }

    fn handler_failure(
        route: KernelRouteRuntimeOutput,
        error: KernelHandlerError,
    ) -> KernelInvocationRuntimeOutput {
        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Failed,
            route: Some(route),
            event: None,
            route_decision_made: true,
            handler_executed: true,
            event_generated: false,
            handler_kind: Some("in_process".to_string()),
            failure_stage: Some("handler_execute".to_string()),
            failure_reason: Some(error.to_string()),
            spawned_process: false,
            called_real_component: false,
        }
    }

    fn completed(
        envelope: KernelInvocationEnvelope,
        route: KernelRouteRuntimeOutput,
        result: KernelHandlerResult,
    ) -> KernelInvocationRuntimeOutput {
        let mut event = KernelEventEnvelope::new(
            format!("event.{}", envelope.operation),
            KernelEventType::InvocationCompleted,
            envelope.instance_id.clone(),
            route.app_id.clone(),
            format!("invoke.{}", envelope.operation),
            Visibility::User,
        );
        event.payload = KernelEventPayload::Summary(result.summary);
        event.trace_context = envelope.trace_context;

        KernelInvocationRuntimeOutput {
            status: KernelInvocationStatus::Completed,
            route: Some(route),
            event: Some(event),
            route_decision_made: true,
            handler_executed: true,
            event_generated: true,
            handler_kind: Some("in_process".to_string()),
            failure_stage: None,
            failure_reason: None,
            spawned_process: false,
            called_real_component: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::{
        InstalledManifestRegistry, KernelEventPayload, KernelEventType, KernelHandlerError,
        KernelHandlerRegistry, KernelHandlerResult, KernelInvocationEnvelope,
        KernelInvocationRuntime, KernelInvocationStatus, KernelPayload,
    };

    #[test]
    fn invocation_runtime_routes_before_handler_execution() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert_eq!(
            output
                .route
                .as_ref()
                .expect("route should exist")
                .component_id,
            "aicore-cli"
        );
        assert!(output.route_decision_made);
        assert!(output.handler_executed);
    }

    #[test]
    fn invocation_runtime_executes_registered_in_process_handler() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Completed);
        assert!(output.handler_executed);
        assert_eq!(output.handler_kind.as_deref(), Some("in_process"));
        assert_eq!(output.failure_reason, None);
    }

    #[test]
    fn invocation_runtime_returns_event_envelope() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));
        let event = output.event.expect("event should be generated");

        assert_eq!(event.event_type, KernelEventType::InvocationCompleted);
        assert_eq!(event.app_id, "aicore-cli");
        assert_eq!(event.instance_id, "global-main");
        assert_eq!(event.invocation_id, "invoke.memory.search");
        assert_eq!(event.trace_context.trace_id, "trace.default");
        assert_eq!(
            event.payload,
            KernelEventPayload::Summary("smoke handled memory.search".to_string())
        );
    }

    #[test]
    fn invocation_runtime_missing_operation_does_not_execute_handler() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("unknown.operation", smoke_handler),
        );

        let output = runtime.invoke(envelope("unknown.operation"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.route_decision_made);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("missing capability")
        );
    }

    #[test]
    fn invocation_runtime_missing_handler_returns_structured_failure() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new(),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(output.route_decision_made);
        assert!(!output.handler_executed);
        assert_eq!(output.failure_stage.as_deref(), Some("handler_lookup"));
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("missing handler")
        );
    }

    #[test]
    fn invocation_runtime_ambiguous_route_does_not_execute_handler() {
        let root = temp_dir("ambiguous-route");
        write_manifest(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            &[("memory.search", "memory.search")],
        );
        write_manifest(
            &root,
            "aicore-memory.toml",
            "aicore-memory",
            &[("memory.search", "memory.search")],
        );
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("ambiguous route")
        );
    }

    #[test]
    fn invocation_runtime_contract_mismatch_does_not_execute_handler() {
        let root = temp_dir("contract-mismatch");
        write_manifest_with_contract(
            &root,
            "aicore-cli.toml",
            "aicore-cli",
            "kernel.app.v2",
            &[("memory.search", "memory.search")],
        );
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&root).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(!output.handler_executed);
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("contract version mismatch")
        );
    }

    #[test]
    fn invocation_runtime_handler_failure_returns_structured_failure() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", failing_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert_eq!(output.status, KernelInvocationStatus::Failed);
        assert!(output.route_decision_made);
        assert!(output.handler_executed);
        assert_eq!(output.failure_stage.as_deref(), Some("handler_execute"));
        assert!(
            output
                .failure_reason
                .as_deref()
                .expect("failure reason")
                .contains("smoke handler failed")
        );
    }

    #[test]
    fn invocation_runtime_output_marks_handler_executed() {
        let registry = registry_with_manifest(&[("memory.search", "memory.search")]);
        let runtime = KernelInvocationRuntime::new(
            InstalledManifestRegistry::load_from_dir(&registry).expect("registry"),
            KernelHandlerRegistry::new().with_handler("memory.search", smoke_handler),
        );

        let output = runtime.invoke(envelope("memory.search"));

        assert!(output.handler_executed);
        assert!(output.event_generated);
        assert!(!output.spawned_process);
        assert!(!output.called_real_component);
    }

    fn smoke_handler(
        envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        Ok(KernelHandlerResult::summary(format!(
            "smoke handled {}",
            envelope.operation
        )))
    }

    fn failing_handler(
        _envelope: &KernelInvocationEnvelope,
        _route: &crate::KernelRouteRuntimeOutput,
    ) -> Result<KernelHandlerResult, KernelHandlerError> {
        Err(KernelHandlerError::new("smoke handler failed"))
    }

    fn envelope(operation: &str) -> KernelInvocationEnvelope {
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty)
    }

    fn registry_with_manifest(capabilities: &[(&str, &str)]) -> PathBuf {
        let root = temp_dir("invocation-registry");
        write_manifest(&root, "aicore-cli.toml", "aicore-cli", capabilities);
        root
    }

    fn write_manifest(
        root: &PathBuf,
        file_name: &str,
        app_id: &str,
        capabilities: &[(&str, &str)],
    ) {
        write_manifest_with_contract(root, file_name, app_id, "kernel.app.v1", capabilities);
    }

    fn write_manifest_with_contract(
        root: &PathBuf,
        file_name: &str,
        app_id: &str,
        contract_version: &str,
        capabilities: &[(&str, &str)],
    ) {
        let mut content = format!(
            "component_id = \"{app_id}\"\napp_id = \"{app_id}\"\nkind = \"app\"\nentrypoint = \"/home/demo/.aicore/bin/{app_id}\"\ncontract_version = \"{contract_version}\"\n"
        );
        for (id, operation) in capabilities {
            content.push_str(&format!(
                "\n[[capabilities]]\nid = \"{id}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n"
            ));
        }
        fs::write(root.join(file_name), content).expect("write manifest");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aicore-kernel-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
