use aicore_foundation::{InstanceId, SessionId, Timestamp, TurnId};
use aicore_memory_proposal::*;

fn now(value: u128) -> Timestamp {
    Timestamp::from_unix_millis(value)
}

fn workspace_instance() -> InstanceId {
    InstanceId::new("workspace.demo").unwrap()
}

fn other_workspace() -> InstanceId {
    InstanceId::new("workspace.other").unwrap()
}

fn proposal_id() -> MemoryProposalId {
    MemoryProposalId::new("proposal.1").unwrap()
}

fn review_id() -> MemoryReviewId {
    MemoryReviewId::new("review.1").unwrap()
}

fn decision_id() -> MemoryDecisionId {
    MemoryDecisionId::new("decision.1").unwrap()
}

fn source_ref(instance_id: InstanceId) -> MemorySourceRef {
    MemorySourceRef {
        source_instance_id: instance_id,
        source_workspace_path: Some("demo-workspace".to_string()),
        source_label: "turn-summary".to_string(),
    }
}

fn proposal_request(actor_kind: MemoryProposalSourceKind) -> MemoryProposalRequest {
    MemoryProposalRequest {
        proposal_id: proposal_id(),
        target_instance_id: workspace_instance(),
        source_instance_id: workspace_instance(),
        source_session_id: Some(SessionId::new("session.1").unwrap()),
        source_turn_id: Some(TurnId::new("turn.1").unwrap()),
        source_actor_kind: actor_kind,
        source_actor_id: Some("agent.main".to_string()),
        source_refs: vec![source_ref(workspace_instance())],
        proposed_memory_class: MemoryClass::Long,
        reason_en: "The user confirmed a reusable repository rule.".to_string(),
        context_summary_en: "The rule applies to future implementation turns.".to_string(),
        candidate_text_en: "Use isolated worktrees for milestone implementation.".to_string(),
        created_at: now(1),
    }
}

fn review() -> MemoryProposalReview {
    MemoryProposalReview {
        review_id: review_id(),
        proposal_id: proposal_id(),
        target_instance_id: workspace_instance(),
        target_memory_class: MemoryClass::Long,
        dedupe_summary_en: "No matching memory found in provided summaries.".to_string(),
        risk_flags: vec![MemoryRiskFlag::Safe],
        canonical_text_en:
            "For milestone implementation, use an isolated worktree before editing code."
                .to_string(),
        user_annotation_zh: "里程碑实现前使用隔离 worktree。".to_string(),
        review_summary_zh: "这是一条可复用的项目执行规则，建议写入当前实例记忆。".to_string(),
        source_refs: vec![source_ref(workspace_instance())],
        recommended_decision: MemoryUserDecisionKind::ApproveWrite,
        created_at: now(2),
    }
}

#[test]
fn core_types_round_trip_through_json() {
    let request = proposal_request(MemoryProposalSourceKind::Agent);
    let json = serde_json::to_string(&request).unwrap();
    let decoded: MemoryProposalRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.target_instance_id, workspace_instance());
    assert_eq!(decoded.proposed_memory_class, MemoryClass::Long);
    assert_eq!(decoded.source_actor_kind, MemoryProposalSourceKind::Agent);
}

#[test]
fn create_proposal_only_creates_pending_proposal_without_write() {
    let mut runtime = InMemoryMemoryProposalRuntime::new();
    let outcome = runtime
        .create_proposal(proposal_request(MemoryProposalSourceKind::Agent))
        .unwrap();
    assert_eq!(outcome.record.status, MemoryProposalStatus::PendingReview);
    assert_eq!(
        outcome.record.write_boundary_status,
        MemoryWriteBoundaryStatus::NotRequested
    );
    assert!(!outcome.record.write_applied);
    assert!(outcome.write_request.is_none());
    assert!(!runtime.proposal_can_enter_memory_context(&proposal_id()));
}

#[test]
fn instance_isolation_rejects_cross_instance_targets_and_global_main_from_workspace() {
    let mut runtime = InMemoryMemoryProposalRuntime::new();
    let mut request = proposal_request(MemoryProposalSourceKind::Agent);
    request.target_instance_id = other_workspace();
    assert_eq!(
        runtime.create_proposal(request).unwrap_err(),
        MemoryProposalRuntimeError::CrossInstanceProposalRejected
    );

    let mut request = proposal_request(MemoryProposalSourceKind::TeamAgent);
    request.target_instance_id = InstanceId::global_main();
    assert_eq!(
        runtime.create_proposal(request).unwrap_err(),
        MemoryProposalRuntimeError::CrossInstanceProposalRejected
    );
}

#[test]
fn team_agent_proposal_has_same_pseudo_write_boundary_as_agent() {
    let mut runtime = InMemoryMemoryProposalRuntime::new();
    let outcome = runtime
        .create_proposal(proposal_request(MemoryProposalSourceKind::TeamAgent))
        .unwrap();
    assert_eq!(
        outcome.record.source_actor_kind,
        MemoryProposalSourceKind::TeamAgent
    );
    assert_eq!(outcome.record.status, MemoryProposalStatus::PendingReview);
    assert_eq!(runtime.snapshot().write_requests.len(), 0);
}

#[test]
fn review_proposal_builds_ui_only_annotation_and_review_card() {
    let mut runtime = InMemoryMemoryProposalRuntime::new();
    runtime
        .create_proposal(proposal_request(MemoryProposalSourceKind::Agent))
        .unwrap();
    let stored_review = runtime.review_proposal(review()).unwrap();
    assert_eq!(
        stored_review.status_after_review,
        MemoryProposalStatus::ReviewReady
    );
    assert_eq!(
        stored_review.user_annotation_zh_visibility,
        MemoryFieldVisibility::UiOnly
    );
    assert!(!stored_review.user_annotation_enters_model_context);

    let card = runtime.build_review_card(&proposal_id()).unwrap();
    assert_eq!(
        card.proposed_summary_zh,
        stored_review.review.review_summary_zh
    );
    assert_eq!(card.memory_class, MemoryClass::Long);
    assert_eq!(card.target_instance_id, workspace_instance());
    assert!(
        card.available_decisions
            .contains(&MemoryUserDecisionKind::ApproveWrite)
    );
    assert!(
        card.available_decisions
            .contains(&MemoryUserDecisionKind::EditThenWrite)
    );
    assert!(
        card.available_decisions
            .contains(&MemoryUserDecisionKind::Reject)
    );
    assert!(
        card.available_decisions
            .contains(&MemoryUserDecisionKind::DeferReview)
    );
    assert_eq!(
        card.user_annotation_zh_visibility,
        MemoryFieldVisibility::UiOnly
    );
    assert!(!card.user_annotation_enters_model_context);
}

#[test]
fn approve_write_creates_write_request_without_applying_write() {
    let mut runtime = reviewed_runtime();
    let decision = MemoryUserDecision {
        decision_id: decision_id(),
        proposal_id: proposal_id(),
        actor_kind: MemoryProposalSourceKind::MemoryAgent,
        decision_kind: MemoryUserDecisionKind::ApproveWrite,
        edited_canonical_text_en: None,
        edited_user_annotation_zh: None,
        decided_at: now(3),
    };
    let outcome = runtime.record_user_decision(decision).unwrap();
    assert_eq!(outcome.record.status, MemoryProposalStatus::WriteRequested);
    let write_request = outcome.write_request.unwrap();
    assert_eq!(write_request.status, MemoryWriteBoundaryStatus::Requested);
    assert!(!write_request.applied);
    assert_eq!(write_request.canonical_text_en, review().canonical_text_en);
    assert_eq!(runtime.snapshot().write_requests.len(), 1);
}

#[test]
fn edit_then_write_uses_edited_canonical_text() {
    let mut runtime = reviewed_runtime();
    let decision = MemoryUserDecision {
        decision_id: decision_id(),
        proposal_id: proposal_id(),
        actor_kind: MemoryProposalSourceKind::MemoryAgent,
        decision_kind: MemoryUserDecisionKind::EditThenWrite,
        edited_canonical_text_en: Some("Edited reusable memory.".to_string()),
        edited_user_annotation_zh: Some("用户编辑后的注释。".to_string()),
        decided_at: now(4),
    };
    let outcome = runtime.record_user_decision(decision).unwrap();
    let write_request = outcome.write_request.unwrap();
    assert_eq!(write_request.canonical_text_en, "Edited reusable memory.");
    assert_eq!(write_request.user_annotation_zh, "用户编辑后的注释。");
    assert!(!write_request.applied);
}

#[test]
fn reject_and_defer_do_not_create_write_request_or_prompt_memory() {
    let mut reject_runtime = reviewed_runtime();
    let reject = MemoryUserDecision {
        decision_id: decision_id(),
        proposal_id: proposal_id(),
        actor_kind: MemoryProposalSourceKind::MemoryAgent,
        decision_kind: MemoryUserDecisionKind::Reject,
        edited_canonical_text_en: None,
        edited_user_annotation_zh: None,
        decided_at: now(5),
    };
    let reject_outcome = reject_runtime.record_user_decision(reject).unwrap();
    assert_eq!(reject_outcome.record.status, MemoryProposalStatus::Rejected);
    assert!(reject_outcome.write_request.is_none());
    assert!(!reject_runtime.proposal_can_enter_memory_context(&proposal_id()));

    let mut defer_runtime = reviewed_runtime();
    let defer = MemoryUserDecision {
        decision_id: MemoryDecisionId::new("decision.2").unwrap(),
        proposal_id: proposal_id(),
        actor_kind: MemoryProposalSourceKind::MemoryAgent,
        decision_kind: MemoryUserDecisionKind::DeferReview,
        edited_canonical_text_en: None,
        edited_user_annotation_zh: None,
        decided_at: now(6),
    };
    let defer_outcome = defer_runtime.record_user_decision(defer).unwrap();
    assert_eq!(defer_outcome.record.status, MemoryProposalStatus::Deferred);
    assert!(defer_outcome.write_request.is_none());
    assert!(!defer_runtime.proposal_can_enter_memory_context(&proposal_id()));
}

#[test]
fn ordinary_actor_cannot_create_memory_agent_write_request() {
    let mut runtime = reviewed_runtime();
    let decision = MemoryUserDecision {
        decision_id: decision_id(),
        proposal_id: proposal_id(),
        actor_kind: MemoryProposalSourceKind::Agent,
        decision_kind: MemoryUserDecisionKind::ApproveWrite,
        edited_canonical_text_en: None,
        edited_user_annotation_zh: None,
        decided_at: now(7),
    };
    assert_eq!(
        runtime.record_user_decision(decision).unwrap_err(),
        MemoryProposalRuntimeError::WriteRequestRequiresMemoryAgent
    );
    assert_eq!(runtime.snapshot().write_requests.len(), 0);
}

#[test]
fn no_raw_leak_guard_and_non_goal_symbols() {
    let record_json =
        serde_json::to_string(&proposal_request(MemoryProposalSourceKind::Agent)).unwrap();
    for word in [
        "raw_provider_request",
        "raw_provider_response",
        "raw_tool_input",
        "raw_tool_output",
        "raw_stdout",
        "raw_stderr",
        "raw_memory_content",
        "raw_prompt",
        "raw_log",
        "secret",
        "token",
        "api_key",
        "cookie",
        "credential",
        "authorization",
        "password",
    ] {
        assert!(
            !record_json.contains(word),
            "forbidden field leaked: {word}"
        );
    }

    for symbol in exported_memory_proposal_symbols() {
        for forbidden in [
            "memory_search",
            "memory_kernel_write",
            "memory_db_write",
            "query",
            "event_query",
            "provider_runtime",
            "tool_runtime",
            "team_runtime",
            "cli_review_ui",
            "tui_review_ui",
            "web_review_ui",
        ] {
            assert!(!symbol.contains(forbidden), "unexpected symbol: {symbol}");
        }
    }
}

fn reviewed_runtime() -> InMemoryMemoryProposalRuntime {
    let mut runtime = InMemoryMemoryProposalRuntime::new();
    runtime
        .create_proposal(proposal_request(MemoryProposalSourceKind::Agent))
        .unwrap();
    runtime.review_proposal(review()).unwrap();
    runtime
}
