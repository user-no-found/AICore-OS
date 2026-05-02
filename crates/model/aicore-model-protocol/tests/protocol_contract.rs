use aicore_foundation::{InstanceId, SessionId, Timestamp};
use aicore_model_protocol::*;

const FORBIDDEN_FIELDS: &[&str] = &[
    "raw_provider_request",
    "raw_provider_response",
    "raw_sdk_request",
    "raw_sdk_response",
    "raw_prompt",
    "raw_tool_input",
    "raw_tool_output",
    "raw_stdout",
    "raw_stderr",
    "raw_memory_content",
    "secret",
    "token",
    "api_key",
    "cookie",
    "credential",
    "authorization",
    "password",
];

fn request(modules: Vec<PromptModule>) -> PromptAssemblyRequest {
    PromptAssemblyRequest {
        assembly_id: PromptAssemblyId::new("assembly.1").unwrap(),
        instance_id: InstanceId::new("workspace.demo").unwrap(),
        session_id: Some(SessionId::new("session.1").unwrap()),
        turn_id: Some("turn.1".to_string()),
        is_global_main: false,
        modules,
        max_budget_units: Some(4096),
        created_at: Timestamp::from_unix_millis(10),
    }
}

fn module(kind: PromptModuleKind, source: PromptModuleSource, content: &str) -> PromptModule {
    PromptModule {
        module_id: PromptModuleId::new(format!("module.{:?}", kind).to_lowercase()).unwrap(),
        kind,
        source,
        visibility: PromptModuleVisibility::ModelVisible,
        content: content.to_string(),
        content_digest: Some("digest.1".to_string()),
        content_unit_estimate: 8,
        redaction_flags: vec![],
    }
}

fn valid_modules() -> Vec<PromptModule> {
    vec![
        module(
            PromptModuleKind::InstanceSoul,
            PromptModuleSource::CurrentInstanceSoul,
            "soul",
        ),
        module(
            PromptModuleKind::VisibleCapabilities,
            PromptModuleSource::CapabilityProjection,
            "caps",
        ),
        module(
            PromptModuleKind::MemoryContext,
            PromptModuleSource::VisibleMemorySummary,
            "memory",
        ),
        module(
            PromptModuleKind::SkillsContext,
            PromptModuleSource::SkillContext,
            "skills",
        ),
        module(
            PromptModuleKind::TeamContext,
            PromptModuleSource::TeamSummary,
            "team",
        ),
        module(
            PromptModuleKind::OutputContract,
            PromptModuleSource::OutputContract,
            "output",
        ),
        module(
            PromptModuleKind::TransientNotices,
            PromptModuleSource::TransientNotice,
            "notice",
        ),
        module(
            PromptModuleKind::UserMessage,
            PromptModuleSource::UserInput,
            "hello",
        ),
    ]
}

#[test]
fn prompt_module_order_is_fixed_and_serializable() {
    let assembly = PromptAssembly::build(request(valid_modules())).unwrap();
    let kinds: Vec<_> = assembly.modules.iter().map(|module| module.kind).collect();
    assert_eq!(kinds, PromptModuleKind::fixed_order());

    let encoded = serde_json::to_string(&assembly).unwrap();
    assert!(encoded.contains("instance_soul"));
    assert!(encoded.contains("user_message"));
    let decoded: PromptAssembly = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded.modules.len(), 8);
}

#[test]
fn prompt_assembly_rejects_missing_required_modules() {
    let without_soul: Vec<_> = valid_modules()
        .into_iter()
        .filter(|module| module.kind != PromptModuleKind::InstanceSoul)
        .collect();
    assert_eq!(
        PromptAssembly::build(request(without_soul)).unwrap_err(),
        PromptAssemblyError::MissingRequiredModule(PromptModuleKind::InstanceSoul)
    );

    let without_user: Vec<_> = valid_modules()
        .into_iter()
        .filter(|module| module.kind != PromptModuleKind::UserMessage)
        .collect();
    assert_eq!(
        PromptAssembly::build(request(without_user)).unwrap_err(),
        PromptAssemblyError::MissingRequiredModule(PromptModuleKind::UserMessage)
    );
}

#[test]
fn unsupported_prompt_modules_are_not_contract_values() {
    assert!(PromptModuleKind::from_contract_value("language_policy").is_none());
    assert!(PromptModuleKind::from_contract_value("core_system_rules").is_none());
    assert!(PromptModuleKind::from_contract_value("task_context").is_none());
    assert!(PromptModuleKind::from_contract_value("dynamic_context").is_none());
}

#[test]
fn workspace_assembly_rejects_global_main_visibility_sources() {
    let mut modules = valid_modules();
    modules[0] = module(
        PromptModuleKind::InstanceSoul,
        PromptModuleSource::GlobalMainSoul,
        "wrong soul",
    );
    assert_eq!(
        PromptAssembly::build(request(modules)).unwrap_err(),
        PromptAssemblyError::WorkspaceCannotUseGlobalMainSoul
    );

    let mut modules = valid_modules();
    modules.insert(
        1,
        module(
            PromptModuleKind::VisibleCapabilities,
            PromptModuleSource::GlobalMainUserProfile,
            "profile",
        ),
    );
    assert_eq!(
        PromptAssembly::build(request(modules)).unwrap_err(),
        PromptAssemblyError::WorkspaceCannotUseGlobalMainUserProfile
    );
}

#[test]
fn model_events_distinguish_delta_final_error_and_stop() {
    let request = model_request();
    let events = vec![
        ModelResponseEvent::started(&request, Timestamp::from_unix_millis(11)),
        ModelResponseEvent::assistant_delta(&request, "partial", Timestamp::from_unix_millis(12)),
        ModelResponseEvent::assistant_final(&request, "done", Timestamp::from_unix_millis(13)),
    ];
    let final_response = ModelFinalResponse::from_events(&events, None).unwrap();
    assert_eq!(final_response.status, ModelRunStatus::Completed);
    assert_eq!(final_response.final_text.as_deref(), Some("done"));

    let only_delta = vec![ModelResponseEvent::assistant_delta(
        &request,
        "partial",
        Timestamp::from_unix_millis(12),
    )];
    assert!(
        ModelFinalResponse::from_events(&only_delta, None)
            .unwrap()
            .final_text
            .is_none()
    );

    let failed = vec![ModelResponseEvent::provider_error(
        &request,
        "provider_failed",
        Timestamp::from_unix_millis(14),
    )];
    let final_response = ModelFinalResponse::from_events(&failed, None).unwrap();
    assert_eq!(final_response.status, ModelRunStatus::Failed);
    assert!(final_response.final_text.is_none());

    let stopped =
        ModelFinalResponse::from_events(&events, Some(Timestamp::from_unix_millis(12))).unwrap();
    assert_eq!(stopped.status, ModelRunStatus::StoppedBeforeFinal);
    assert!(stopped.final_text.is_none());
}

#[test]
fn final_before_stop_can_complete_and_cancelled_cannot_complete() {
    let request = model_request();
    let events = vec![ModelResponseEvent::assistant_final(
        &request,
        "done",
        Timestamp::from_unix_millis(10),
    )];
    assert_eq!(
        ModelFinalResponse::from_events(&events, Some(Timestamp::from_unix_millis(11)))
            .unwrap()
            .status,
        ModelRunStatus::Completed
    );

    let cancelled = vec![
        ModelResponseEvent::cancelled(&request, Timestamp::from_unix_millis(10)),
        ModelResponseEvent::assistant_final(&request, "late", Timestamp::from_unix_millis(11)),
    ];
    let final_response = ModelFinalResponse::from_events(&cancelled, None).unwrap();
    assert_eq!(final_response.status, ModelRunStatus::Cancelled);
    assert!(final_response.final_text.is_none());
}

#[test]
fn scripted_provider_returns_events_in_order_without_runtime_dependencies() {
    let request = model_request();
    let provider = ScriptedModelProvider::new(vec![
        ModelResponseEventKind::RequestStarted,
        ModelResponseEventKind::AssistantDelta,
        ModelResponseEventKind::AssistantFinal,
    ]);
    let events = provider.invoke(&request).unwrap();
    let kinds: Vec<_> = events.iter().map(|event| event.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ModelResponseEventKind::RequestStarted,
            ModelResponseEventKind::AssistantDelta,
            ModelResponseEventKind::AssistantFinal,
        ]
    );
}

#[test]
fn scripted_provider_can_simulate_error_stop_and_final() {
    let request = model_request();
    let error_provider = ScriptedModelProvider::new(vec![ModelResponseEventKind::ProviderError]);
    let error_events = error_provider.invoke(&request).unwrap();
    assert_eq!(error_events[0].kind, ModelResponseEventKind::ProviderError);
    assert_eq!(
        ModelFinalResponse::from_events(&error_events, None)
            .unwrap()
            .status,
        ModelRunStatus::Failed
    );

    let stop_provider =
        ScriptedModelProvider::new(vec![ModelResponseEventKind::StoppedBeforeFinal]);
    let stop_events = stop_provider.invoke(&request).unwrap();
    assert_eq!(
        stop_events[0].kind,
        ModelResponseEventKind::StoppedBeforeFinal
    );

    let final_provider = ScriptedModelProvider::new(vec![ModelResponseEventKind::AssistantFinal]);
    let final_events = final_provider.invoke(&request).unwrap();
    assert_eq!(
        ModelFinalResponse::from_events(&final_events, None)
            .unwrap()
            .status,
        ModelRunStatus::Completed
    );
}

#[test]
fn crate_manifest_has_no_live_provider_or_runtime_dependencies() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    for forbidden in [
        "reqwest",
        "hyper",
        "tokio",
        "async-std",
        "sqlx",
        "rusqlite",
        "tungstenite",
        "aicore-session-sqlite",
    ] {
        assert!(
            !manifest.contains(forbidden),
            "forbidden dependency found: {forbidden}"
        );
    }
}

#[test]
fn source_has_no_query_or_runtime_execution_entrypoints() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut source = String::new();
    for entry in std::fs::read_dir(src_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            for nested in std::fs::read_dir(entry.path()).unwrap() {
                let nested = nested.unwrap();
                if nested.path().extension().and_then(|ext| ext.to_str()) == Some("rs") {
                    source.push_str(&std::fs::read_to_string(nested.path()).unwrap());
                }
            }
        } else if entry.path().extension().and_then(|ext| ext.to_str()) == Some("rs") {
            source.push_str(&std::fs::read_to_string(entry.path()).unwrap());
        }
    }

    for forbidden in [
        "Query",
        "EventQuery",
        "execute_tool",
        "mcp_call",
        "team_spawn",
        "memory_proposal_execute",
        "provider_live_call",
        "http_client",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden runtime symbol found: {forbidden}"
        );
    }
}

#[test]
fn public_structures_do_not_expose_forbidden_fields() {
    let assembly = PromptAssembly::build(request(valid_modules())).unwrap();
    let request = model_request();
    let event =
        ModelResponseEvent::assistant_delta(&request, "partial", Timestamp::from_unix_millis(20));
    let value = serde_json::json!({
        "assembly": assembly,
        "request": request,
        "event": event,
    });
    let encoded = serde_json::to_string(&value).unwrap();
    for field in FORBIDDEN_FIELDS {
        assert!(!encoded.contains(field), "forbidden field leaked: {field}");
    }
}

#[test]
fn ids_and_enums_follow_contract_values() {
    assert_eq!(
        ModelRequestId::new("request.1").unwrap().as_str(),
        "request.1"
    );
    assert_eq!(ModelRunId::new("run.1").unwrap().as_str(), "run.1");
    assert_eq!(
        ProviderId::new("provider.test").unwrap().as_str(),
        "provider.test"
    );
    assert!(ModelId::new("bad/model").is_err());
    assert_eq!(
        serde_json::to_string(&ModelStopReason::StopRequested).unwrap(),
        "\"stop_requested\""
    );
    assert_eq!(
        serde_json::to_string(&ModelRunStatus::StoppedBeforeFinal).unwrap(),
        "\"stopped_before_final\""
    );
}

fn model_request() -> ModelRequestEnvelope {
    let assembly = PromptAssembly::build(request(valid_modules())).unwrap();
    ModelRequestEnvelope {
        request_id: ModelRequestId::new("request.1").unwrap(),
        run_id: ModelRunId::new("run.1").unwrap(),
        instance_id: InstanceId::new("workspace.demo").unwrap(),
        session_id: Some(SessionId::new("session.1").unwrap()),
        turn_id: Some("turn.1".to_string()),
        correlation_id: Some("corr.1".to_string()),
        causation_id: Some("turn.1".to_string()),
        provider: ModelProviderRef {
            provider_id: ProviderId::new("provider.test").unwrap(),
        },
        profile: ModelProfileRef {
            model_id: ModelId::new("model.test").unwrap(),
            profile_id: Some("profile.test".to_string()),
        },
        protocol_version: ModelProtocolVersion::V1,
        assembly,
        options: ModelRequestOptions {
            stream: true,
            max_output_units: Some(512),
            temperature: None,
        },
        created_at: Timestamp::from_unix_millis(9),
    }
}
