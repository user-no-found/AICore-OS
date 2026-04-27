use crate::{TraceContext, Visibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEventType {
    AppRegistered,
    InvocationStarted,
    InvocationCompleted,
    InvocationFailed,
    RouteDecided,
    WorkQueued,
    WorkStarted,
    WorkCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEventSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelEventPayload {
    Summary(String),
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelEventEnvelope {
    pub event_id: String,
    pub event_type: KernelEventType,
    pub instance_id: String,
    pub app_id: String,
    pub invocation_id: String,
    pub visibility: Visibility,
    pub severity: KernelEventSeverity,
    pub payload: KernelEventPayload,
    pub trace_context: TraceContext,
}

impl KernelEventEnvelope {
    pub fn new(
        event_id: impl Into<String>,
        event_type: KernelEventType,
        instance_id: impl Into<String>,
        app_id: impl Into<String>,
        invocation_id: impl Into<String>,
        visibility: Visibility,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_type,
            instance_id: instance_id.into(),
            app_id: app_id.into(),
            invocation_id: invocation_id.into(),
            visibility,
            severity: KernelEventSeverity::Info,
            payload: KernelEventPayload::Empty,
            trace_context: TraceContext::new("trace.event"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Visibility;

    use super::{KernelEventEnvelope, KernelEventType};

    #[test]
    fn event_envelope_carries_visibility_and_trace_context() {
        let event = KernelEventEnvelope::new(
            "evt.1",
            KernelEventType::InvocationStarted,
            "global-main",
            "app.provider",
            "invoke.1",
            Visibility::Audit,
        );

        assert_eq!(event.visibility, Visibility::Audit);
        assert_eq!(event.trace_context.trace_id, "trace.event");
    }
}
