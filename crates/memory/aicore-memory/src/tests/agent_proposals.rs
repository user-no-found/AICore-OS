use super::support::*;

#[test]
fn rule_based_agent_extracts_remember_proposal() {
    let output = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::ExplicitRemember,
        vec!["记住：TUI 是类似 Codex 的终端 AI 编程界面"],
    ));

    assert_eq!(output.proposals.len(), 1);
    assert_eq!(output.proposals[0].memory_type, MemoryType::Core);
    assert_eq!(
        output.proposals[0].content,
        "TUI 是类似 Codex 的终端 AI 编程界面"
    );
}

#[test]
fn rule_based_agent_extracts_stage_status_proposal() {
    let output = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::StageCompleted,
        vec!["已完成 P6.2.4 Memory Lock / Single Writer Guard"],
    ));

    assert_eq!(output.proposals.len(), 1);
    assert_eq!(output.proposals[0].memory_type, MemoryType::Status);
    assert!(
        output.proposals[0]
            .content
            .contains("已完成 P6.2.4 Memory Lock / Single Writer Guard")
    );
}

#[test]
fn rule_based_agent_outputs_proposals_only() {
    let output = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::Correction,
        vec!["纠正：上一条记忆不准确"],
    ));

    assert!(!output.proposals.is_empty());
    assert!(output.corrections.is_empty());
    assert!(output.archive_suggestions.is_empty());
}

#[test]
fn memory_agent_does_not_create_records() {
    let kernel =
        MemoryKernel::open(temp_paths("agent-no-records")).expect("memory kernel should open");
    let before = kernel.records().len();

    let _ = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::ExplicitRemember,
        vec!["记住：不要直接写 record"],
    ));

    assert_eq!(kernel.records().len(), before);
}

#[test]
fn memory_agent_does_not_accept_proposals() {
    let output = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::ExplicitRemember,
        vec!["记住：proposal 不能自动 accept"],
    ));

    assert!(
        output
            .proposals
            .iter()
            .all(|proposal| proposal.status == MemoryProposalStatus::Open)
    );
}

#[test]
fn proposal_dedupe_merges_same_content() {
    let output = RuleBasedMemoryAgent::analyze(&work_batch(
        MemoryTrigger::ExplicitRemember,
        vec!["记住：统一术语", "记住：统一术语"],
    ));

    assert_eq!(output.proposals.len(), 1);
}

#[test]
fn proposal_dedupe_keeps_different_memory_types() {
    let output = RuleBasedMemoryAgent::analyze(&MemoryWorkBatch {
        instance_id: "global-main".to_string(),
        scope: global_scope(),
        trigger: MemoryTrigger::SessionClosed,
        recent_events_summary: String::new(),
        raw_excerpts: vec!["记住：统一术语".to_string(), "已完成 P6.2".to_string()],
        existing_memory_hits: Vec::new(),
        token_budget: 1024,
        requested_outputs: vec![MemoryRequestedOutput::Proposals],
    });

    assert_eq!(output.proposals.len(), 2);
    assert!(
        output
            .proposals
            .iter()
            .any(|proposal| proposal.memory_type == MemoryType::Core)
    );
    assert!(
        output
            .proposals
            .iter()
            .any(|proposal| proposal.memory_type == MemoryType::Status)
    );
}

#[test]
fn submit_agent_output_stores_open_proposals() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-open")).expect("memory kernel should open");

    let inserted = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "记住这个提案")],
            corrections: vec!["ignored".to_string()],
            archive_suggestions: vec!["ignored".to_string()],
        })
        .expect("agent output should be stored");

    assert_eq!(inserted.len(), 1);
    assert_eq!(kernel.proposals().len(), 1);
    assert_eq!(kernel.proposals()[0].status, MemoryProposalStatus::Open);
}

#[test]
fn submit_agent_output_does_not_create_records() {
    let mut kernel = MemoryKernel::open(temp_paths("agent-intake-no-records"))
        .expect("memory kernel should open");

    kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "不要创建 record")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");

    assert!(kernel.records().is_empty());
}

#[test]
fn submit_agent_output_writes_proposed_events() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-events")).expect("memory kernel should open");

    let inserted = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Status, "已完成 P6.3.1")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");

    assert_eq!(kernel.events().len(), 1);
    assert_eq!(kernel.events()[0].event_kind, MemoryEventKind::Proposed);
    assert_eq!(
        kernel.events()[0].proposal_id.as_deref(),
        Some(inserted[0].as_str())
    );
}

#[test]
fn submit_agent_output_dedupes_existing_open_proposals() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-dedupe")).expect("memory kernel should open");

    let first = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "重复提案")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("first intake should succeed");
    let second = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "重复提案")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("second intake should succeed");

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
    assert_eq!(kernel.proposals().len(), 1);
}

#[test]
fn submit_agent_output_keeps_different_memory_types() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-types")).expect("memory kernel should open");

    let inserted = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![
                agent_proposal(MemoryType::Core, "同内容"),
                agent_proposal(MemoryType::Working, "同内容"),
            ],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");

    assert_eq!(inserted.len(), 2);
    assert_eq!(kernel.proposals().len(), 2);
}

#[test]
fn submit_agent_output_preserves_language_fields() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-language")).expect("memory kernel should open");

    kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "中文提案")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");

    let proposal = &kernel.proposals()[0];
    assert_eq!(proposal.content_language, "zh-CN");
    assert_eq!(proposal.normalized_content, "中文提案");
    assert_eq!(proposal.normalized_language, "zh-CN");
}

#[test]
fn submit_agent_output_reassigns_stable_kernel_proposal_ids() {
    let mut kernel =
        MemoryKernel::open(temp_paths("agent-intake-id")).expect("memory kernel should open");

    let inserted = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "重新分配 id")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");

    assert_eq!(inserted.len(), 1);
    assert_ne!(inserted[0], "agent_prop_重新分配 id");
    assert!(inserted[0].starts_with("prop_"));
    assert_eq!(kernel.proposals()[0].proposal_id, inserted[0]);
}

#[test]
fn list_open_proposals_returns_only_open() {
    let mut kernel =
        MemoryKernel::open(temp_paths("proposal-open-list")).expect("memory kernel should open");

    let inserted = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![
                agent_proposal(MemoryType::Core, "开放提案"),
                agent_proposal(MemoryType::Status, "将被拒绝"),
            ],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored");
    kernel
        .reject_proposal(&inserted[1], "user", Some("不需要"))
        .expect("reject should succeed");

    let open = kernel.list_open_proposals();
    assert_eq!(open.len(), 1);
    assert_eq!(open[0].proposal_id, inserted[0]);
    assert_eq!(open[0].status, MemoryProposalStatus::Open);
}

#[test]
fn accept_proposal_creates_active_memory_record() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-record"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "接受后创建记忆")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    let memory_id = kernel
        .accept_proposal(&proposal_id, "user", Some("采纳"))
        .expect("accept should succeed");

    let record = kernel
        .records()
        .iter()
        .find(|record| record.memory_id == memory_id)
        .expect("accepted record should exist");
    assert_eq!(record.status, MemoryStatus::Active);
    assert_eq!(record.permanence, MemoryPermanence::Standard);
    assert_eq!(record.content, "接受后创建记忆");
}

#[test]
fn accept_proposal_marks_proposal_accepted() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-status"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Working, "proposal accepted")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    kernel
        .accept_proposal(&proposal_id, "user", Some("通过"))
        .expect("accept should succeed");

    let proposal = kernel
        .proposals()
        .iter()
        .find(|proposal| proposal.proposal_id == proposal_id)
        .expect("proposal should exist");
    assert_eq!(proposal.status, MemoryProposalStatus::Accepted);
}

#[test]
fn accept_proposal_writes_accepted_event() {
    let mut kernel =
        MemoryKernel::open(temp_paths("proposal-accept-event")).expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Status, "accept event")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    let memory_id = kernel
        .accept_proposal(&proposal_id, "user", Some("通过"))
        .expect("accept should succeed");

    let event = kernel
        .events()
        .iter()
        .find(|event| {
            event.event_kind == MemoryEventKind::Accepted
                && event.proposal_id.as_deref() == Some(proposal_id.as_str())
        })
        .expect("accepted event should exist");
    assert_eq!(event.memory_id.as_deref(), Some(memory_id.as_str()));
}

#[test]
fn accept_proposal_rebuilds_projection() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-projection"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "进入 CORE projection")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    kernel
        .accept_proposal(&proposal_id, "user", Some("通过"))
        .expect("accept should succeed");

    let core = kernel
        .core_markdown()
        .expect("core projection should exist");
    assert!(core.contains("进入 CORE projection"));
    assert!(!kernel.projection_state().stale);
}

#[test]
fn accept_proposal_does_not_make_memory_permanent() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-permanence"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Decision, "标准持久化")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    let memory_id = kernel
        .accept_proposal(&proposal_id, "user", Some("通过"))
        .expect("accept should succeed");

    let record = kernel
        .records()
        .iter()
        .find(|record| record.memory_id == memory_id)
        .expect("record should exist");
    assert_eq!(record.permanence, MemoryPermanence::Standard);
}

#[test]
fn reject_proposal_marks_proposal_rejected() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-status"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Working, "reject me")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    kernel
        .reject_proposal(&proposal_id, "user", Some("拒绝"))
        .expect("reject should succeed");

    let proposal = kernel
        .proposals()
        .iter()
        .find(|proposal| proposal.proposal_id == proposal_id)
        .expect("proposal should exist");
    assert_eq!(proposal.status, MemoryProposalStatus::Rejected);
}

#[test]
fn reject_proposal_writes_rejected_event() {
    let mut kernel =
        MemoryKernel::open(temp_paths("proposal-reject-event")).expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Working, "reject event")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    kernel
        .reject_proposal(&proposal_id, "user", Some("拒绝"))
        .expect("reject should succeed");

    let event = kernel
        .events()
        .iter()
        .find(|event| {
            event.event_kind == MemoryEventKind::Rejected
                && event.proposal_id.as_deref() == Some(proposal_id.as_str())
        })
        .expect("rejected event should exist");
    assert_eq!(event.memory_id, None);
}

#[test]
fn reject_proposal_does_not_create_record() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-no-record"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "reject no record")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();

    kernel
        .reject_proposal(&proposal_id, "user", Some("拒绝"))
        .expect("reject should succeed");

    assert!(kernel.records().is_empty());
}

#[test]
fn accept_rejects_non_open_proposal() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-accept-non-open"))
        .expect("memory kernel should open");

    let accepted_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "already accepted")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();
    kernel
        .accept_proposal(&accepted_id, "user", Some("通过"))
        .expect("accept should succeed");

    let rejected_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "already rejected")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();
    kernel
        .reject_proposal(&rejected_id, "user", Some("拒绝"))
        .expect("reject should succeed");

    let accepted_error = kernel
        .accept_proposal(&accepted_id, "user", Some("重复接受"))
        .expect_err("accepted proposal should be rejected");
    let rejected_error = kernel
        .accept_proposal(&rejected_id, "user", Some("错误接受"))
        .expect_err("rejected proposal should be rejected");

    assert!(accepted_error.0.contains("non-open proposal"));
    assert!(rejected_error.0.contains("non-open proposal"));
}

#[test]
fn reject_rejects_non_open_proposal() {
    let mut kernel = MemoryKernel::open(temp_paths("proposal-reject-non-open"))
        .expect("memory kernel should open");

    let accepted_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "accepted then reject")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();
    kernel
        .accept_proposal(&accepted_id, "user", Some("通过"))
        .expect("accept should succeed");

    let rejected_id = kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![agent_proposal(MemoryType::Core, "rejected then reject")],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")[0]
        .clone();
    kernel
        .reject_proposal(&rejected_id, "user", Some("拒绝"))
        .expect("reject should succeed");

    let accepted_error = kernel
        .reject_proposal(&accepted_id, "user", Some("错误拒绝"))
        .expect_err("accepted proposal should reject further reject");
    let rejected_error = kernel
        .reject_proposal(&rejected_id, "user", Some("重复拒绝"))
        .expect_err("rejected proposal should reject further reject");

    assert!(accepted_error.0.contains("non-open proposal"));
    assert!(rejected_error.0.contains("non-open proposal"));
}
