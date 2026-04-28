use super::support::*;

#[test]
fn memory_remember_writes_active_record() {
    let root = temp_root("memory-remember");
    let output = run_cli_with_config_root(
        &["memory", "remember", "TUI 是类似 Codex 的终端 AI 编程界面"],
        &root,
    );

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆已写入："));
    assert!(stdout.contains("id: mem_"));
    assert!(stdout.contains("type: core"));
    assert!(stdout.contains("status: active"));
}

#[test]
fn cli_memory_remember_rich_uses_terminal_panel() {
    let root = temp_root("memory-remember-rich-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "remember", "rich remember memory"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆已写入"));
    assert!(stdout.contains("id: mem_"));
    assert!(stdout.contains("type: core"));
    assert!(stdout.contains("status: active"));
}

#[test]
fn cli_memory_remember_plain_has_no_ansi() {
    let root = temp_root("memory-remember-plain-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "remember", "plain remember memory"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆已写入："));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_memory_remember_json_outputs_valid_json() {
    let root = temp_root("memory-remember-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "remember", "json remember memory"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("id: mem_"));
    assert!(stdout.contains("status: active"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_remember_no_color_has_no_ansi() {
    let root = temp_root("memory-remember-no-color-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "remember", "no color remember memory"],
        &root,
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆已写入"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_remember_preserves_chinese_text() {
    let root = temp_root("memory-remember-chinese");
    let remember_output = run_cli_with_config_root(
        &["memory", "remember", "记住：终端界面优先中文，命令保持英文"],
        &root,
    );
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "终端界面"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记住：终端界面优先中文，命令保持英文"));
}

#[test]
fn memory_remember_persists_across_cli_processes() {
    let root = temp_root("memory-persist-process");

    let remember_output =
        run_cli_with_config_root(&["memory", "remember", "跨进程持久化记忆"], &root);
    assert!(remember_output.status.success());

    let search_output = run_cli_with_config_root(&["memory", "search", "跨进程"], &root);
    assert!(search_output.status.success());

    let stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("跨进程持久化记忆"));
}

#[test]
fn memory_status_reports_real_counts_after_remember() {
    let root = temp_root("memory-status-after-remember");

    let remember_output =
        run_cli_with_config_root(&["memory", "remember", "status count memory"], &root);
    assert!(remember_output.status.success());

    let status_output = run_cli_with_config_root(&["memory", "status"], &root);
    assert!(status_output.status.success());

    let stdout = String::from_utf8(status_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("records: 1"));
    assert!(stdout.contains("events: 1"));
    assert!(stdout.contains("projection stale: false"));
}

#[test]
fn memory_proposals_empty_prints_friendly_message() {
    let root = temp_root("memory-proposals-empty");
    let output = run_cli_with_config_root(&["memory", "proposals"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("暂无待审阅记忆提案。"));
}

#[test]
fn cli_memory_proposals_rich_uses_terminal_panel_or_table() {
    let root = temp_root("memory-proposals-rich-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "rich proposal memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "proposals"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Proposals"));
    assert!(stdout.contains(&proposal_id));
    assert!(stdout.contains("rich proposal memory"));
}

#[test]
fn cli_memory_proposals_json_outputs_valid_json() {
    let root = temp_root("memory-proposals-json-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "json proposal memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "proposals"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains(&proposal_id));
    assert!(stdout.contains("json proposal memory"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_proposals_empty_json_outputs_valid_json() {
    let root = temp_root("memory-proposals-empty-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "proposals"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("暂无待审阅记忆提案"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_proposals_lists_open_proposals() {
    let root = temp_root("memory-proposals-list");
    let proposal_id = seed_open_proposal(
        &root,
        MemoryType::Core,
        "TUI 是类似 Codex 的终端 AI 编程界面",
    );

    let output = run_cli_with_config_root(&["memory", "proposals"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Proposals："));
    assert!(stdout.contains(&proposal_id));
    assert!(stdout.contains("[core]"));
    assert!(stdout.contains("TUI 是类似 Codex 的终端 AI 编程界面"));
}

#[test]
fn memory_accept_proposal_creates_record() {
    let root = temp_root("memory-accept-proposal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "接受后成为记忆");

    let output = run_cli_with_config_root(&["memory", "accept", &proposal_id], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已接受："));
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));
    assert!(stdout.contains("memory: mem_"));

    let search_output = run_cli_with_config_root(&["memory", "search", "接受后"], &root);
    assert!(search_output.status.success());
    let search_stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(search_stdout.contains("接受后成为记忆"));
}

#[test]
fn cli_memory_accept_rich_uses_terminal_panel() {
    let root = temp_root("memory-accept-rich-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "rich accept memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "accept", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆提案已接受"));
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));
    assert!(stdout.contains("memory: mem_"));
}

#[test]
fn cli_memory_accept_plain_has_no_ansi() {
    let root = temp_root("memory-accept-plain-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "plain accept memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "accept", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已接受："));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_memory_accept_json_outputs_valid_json_and_creates_record() {
    let root = temp_root("memory-accept-json-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "json accept memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "accept", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));
    assert!(stdout.contains("memory: mem_"));
    assert!(!stdout.contains("\u{1b}["));

    let search_output = run_cli_with_config_root(&["memory", "search", "json accept"], &root);
    assert!(search_output.status.success());
    let search_stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(search_stdout.contains("json accept memory"));
}

#[test]
fn memory_accept_proposal_removes_from_open_list() {
    let root = temp_root("memory-accept-removes-open");
    let proposal_id = seed_open_proposal(&root, MemoryType::Status, "accept removes open");

    let accept_output = run_cli_with_config_root(&["memory", "accept", &proposal_id], &root);
    assert!(accept_output.status.success());

    let proposals_output = run_cli_with_config_root(&["memory", "proposals"], &root);
    assert!(proposals_output.status.success());
    let stdout = String::from_utf8(proposals_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("暂无待审阅记忆提案。"));
}

#[test]
fn memory_reject_proposal_does_not_create_record() {
    let root = temp_root("memory-reject-proposal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Working, "拒绝后不生成记忆");

    let output = run_cli_with_config_root(&["memory", "reject", &proposal_id], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已拒绝："));
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));

    let search_output = run_cli_with_config_root(&["memory", "search", "拒绝后"], &root);
    assert!(search_output.status.success());
    let search_stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(search_stdout.contains("无匹配记忆"));
}

#[test]
fn cli_memory_reject_rich_uses_terminal_panel() {
    let root = temp_root("memory-reject-rich-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Working, "rich reject memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "reject", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆提案已拒绝"));
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));
}

#[test]
fn cli_memory_reject_json_outputs_valid_json_and_does_not_create_record() {
    let root = temp_root("memory-reject-json-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Working, "json reject memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "reject", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains(&format!("proposal: {proposal_id}")));
    assert!(!stdout.contains("\u{1b}["));

    let search_output = run_cli_with_config_root(&["memory", "search", "json reject"], &root);
    assert!(search_output.status.success());
    let search_stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(search_stdout.contains("无匹配记忆"));
}

#[test]
fn cli_memory_reject_no_color_has_no_ansi() {
    let root = temp_root("memory-reject-no-color-terminal");
    let proposal_id = seed_open_proposal(&root, MemoryType::Working, "no color reject memory");

    let output = run_cli_with_config_root_and_env(
        &["memory", "reject", &proposal_id],
        &root,
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆提案已拒绝"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_reject_proposal_removes_from_open_list() {
    let root = temp_root("memory-reject-removes-open");
    let proposal_id = seed_open_proposal(&root, MemoryType::Core, "reject removes open");

    let reject_output = run_cli_with_config_root(&["memory", "reject", &proposal_id], &root);
    assert!(reject_output.status.success());

    let proposals_output = run_cli_with_config_root(&["memory", "proposals"], &root);
    assert!(proposals_output.status.success());
    let stdout = String::from_utf8(proposals_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("暂无待审阅记忆提案。"));
}

#[test]
fn memory_accept_unknown_proposal_fails() {
    let root = temp_root("memory-accept-unknown");
    let output = run_cli_with_config_root(&["memory", "accept", "prop_missing"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unknown proposal_id: prop_missing"));
}

#[test]
fn memory_reject_unknown_proposal_fails() {
    let root = temp_root("memory-reject-unknown");
    let output = run_cli_with_config_root(&["memory", "reject", "prop_missing"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("unknown proposal_id: prop_missing"));
}

#[test]
fn rule_based_agent_output_can_be_submitted_and_listed_by_cli() {
    let root = temp_root("rule-agent-cli-list");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：TUI 是类似 Codex 的终端 AI 编程界面",
    );

    let output = run_cli_with_config_root(&["memory", "proposals"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Proposals："));
    assert!(stdout.contains(&proposal_id));
    assert!(stdout.contains("[core]"));
    assert!(stdout.contains("TUI 是类似 Codex 的终端 AI 编程界面"));
}

#[test]
fn accepted_rule_based_proposal_becomes_searchable_memory() {
    let root = temp_root("rule-agent-cli-accept-search");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：终端界面优先中文，命令保持英文",
    );

    let accept_output = run_cli_with_config_root(&["memory", "accept", &proposal_id], &root);
    assert!(accept_output.status.success());

    let search_output = run_cli_with_config_root(&["memory", "search", "终端界面"], &root);
    assert!(search_output.status.success());
    let stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("终端界面优先中文，命令保持英文"));
}

#[test]
fn rejected_rule_based_proposal_does_not_create_searchable_memory() {
    let root = temp_root("rule-agent-cli-reject-search");
    let proposal_id =
        seed_rule_based_proposal(&root, MemoryTrigger::Correction, "你看错了，这不是长期记忆");

    let reject_output = run_cli_with_config_root(&["memory", "reject", &proposal_id], &root);
    assert!(reject_output.status.success());

    let search_output = run_cli_with_config_root(&["memory", "search", "长期记忆"], &root);
    assert!(search_output.status.success());
    let stdout = String::from_utf8(search_output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("无匹配记忆"));
}

#[test]
fn proposal_pipeline_preserves_localized_summary() {
    let root = temp_root("rule-agent-localized-summary");
    let _proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：用户更喜欢 CLI 而不是 Web",
    );

    let output = run_cli_with_config_root(&["memory", "proposals"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("用户更喜欢 CLI 而不是 Web"));
}

#[test]
fn proposal_pipeline_writes_proposed_and_accepted_events() {
    let root = temp_root("rule-agent-events-accept");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::StageCompleted,
        "已完成 P6.3.4 CLI Proposal Review Smoke",
    );

    let accept_output = run_cli_with_config_root(&["memory", "accept", &proposal_id], &root);
    assert!(accept_output.status.success());

    let kernel =
        MemoryKernel::open(memory_paths_for_root(&root)).expect("memory kernel should open");
    assert!(kernel.events().iter().any(|event| {
        event.event_kind == aicore_memory::MemoryEventKind::Proposed
            && event.proposal_id.as_deref() == Some(proposal_id.as_str())
    }));
    assert!(kernel.events().iter().any(|event| {
        event.event_kind == aicore_memory::MemoryEventKind::Accepted
            && event.proposal_id.as_deref() == Some(proposal_id.as_str())
    }));
}

#[test]
fn proposal_pipeline_reject_writes_rejected_event() {
    let root = temp_root("rule-agent-events-reject");
    let proposal_id =
        seed_rule_based_proposal(&root, MemoryTrigger::Correction, "纠正：上一条描述不准确");

    let reject_output = run_cli_with_config_root(&["memory", "reject", &proposal_id], &root);
    assert!(reject_output.status.success());

    let kernel =
        MemoryKernel::open(memory_paths_for_root(&root)).expect("memory kernel should open");
    assert!(kernel.events().iter().any(|event| {
        event.event_kind == aicore_memory::MemoryEventKind::Rejected
            && event.proposal_id.as_deref() == Some(proposal_id.as_str())
    }));
}
