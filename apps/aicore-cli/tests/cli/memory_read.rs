use super::support::*;

#[test]
fn memory_wiki_defaults_to_index() {
    let root = temp_root("memory-wiki-index");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki index memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆 Wiki Projection（local direct）："));
    assert!(stdout.contains("- page: index"));
    assert!(stdout.contains("# Memory Wiki"));
    assert!(stdout.contains("[Core](core.md)"));
}

#[test]
fn memory_wiki_reads_core_page() {
    let root = temp_root("memory-wiki-core");
    let memory_id = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki core memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "core", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: core"));
    assert!(stdout.contains("# Core Memories"));
    assert!(stdout.contains(&memory_id));
    assert!(stdout.contains("wiki core memory"));
}

#[test]
fn memory_wiki_reads_decisions_page() {
    let root = temp_root("memory-wiki-decisions");
    let memory_id = seed_memory_record(
        &root,
        MemoryType::Decision,
        MemoryPermanence::Standard,
        "wiki decision memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "decisions", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: decisions"));
    assert!(stdout.contains("# Decisions"));
    assert!(stdout.contains(&memory_id));
}

#[test]
fn memory_wiki_reads_status_page() {
    let root = temp_root("memory-wiki-status");
    let memory_id = seed_memory_record(
        &root,
        MemoryType::Status,
        MemoryPermanence::Standard,
        "wiki status memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "status", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: status"));
    assert!(stdout.contains("# Status"));
    assert!(stdout.contains(&memory_id));
}

#[test]
fn memory_wiki_accepts_md_suffix() {
    let root = temp_root("memory-wiki-md-suffix");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki suffix memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "core.md", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: core"));
}

#[test]
fn memory_wiki_rejects_unknown_page() {
    let root = temp_root("memory-wiki-unknown");
    let output = run_cli_with_config_root(&["memory", "wiki", "unknown", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("未知 Wiki 页面"));
}

#[test]
fn memory_wiki_rejects_path_traversal() {
    let root = temp_root("memory-wiki-traversal");
    let output = run_cli_with_config_root(&["memory", "wiki", "../../secret", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("不允许读取任意 Wiki 路径"));
}

#[test]
fn memory_wiki_reports_missing_projection() {
    let root = temp_root("memory-wiki-missing");
    let output = run_cli_with_config_root(&["memory", "wiki", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 Wiki Projection"));
}

#[test]
fn memory_wiki_output_preserves_not_truth_source_notice() {
    let root = temp_root("memory-wiki-not-truth");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki notice memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki", "index", "--local"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("这是 generated projection"));
    assert!(stdout.contains("不是事实来源"));
    assert!(stdout.contains("不应手工编辑后期待反向同步"));
}

#[test]
fn cli_memory_wiki_rich_uses_terminal_markdown_or_panel() {
    let root = temp_root("memory-wiki-rich-terminal");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki rich terminal memory",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆 Wiki Projection"));
    assert!(stdout.contains("- page: index"));
    assert!(stdout.contains("# Memory Wiki"));
    assert!(stdout.contains("不是事实来源"));
}

#[test]
fn cli_memory_wiki_json_outputs_valid_json() {
    let root = temp_root("memory-wiki-json-terminal");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki json terminal memory",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(stdout.contains("不是事实来源"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_wiki_page_json_outputs_valid_json() {
    let root = temp_root("memory-wiki-page-json-terminal");
    let memory_id = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki page json terminal memory",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "wiki", "core", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(stdout.contains(&memory_id));
    assert!(stdout.contains("wiki page json terminal memory"));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_status_command_succeeds() {
    let root = temp_root("memory-status");
    let output = run_cli_with_config_root(&["memory", "status", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status（local direct）："));
    assert!(stdout.contains("instance: global-main"));
    assert!(stdout.contains("records: 0"));
    assert!(stdout.contains("proposals: 0"));
    assert!(stdout.contains("events: 0"));
    assert!(stdout.contains("projection stale: false"));
}

#[test]
fn cli_memory_status_rich_uses_terminal_panel() {
    let root = temp_root("memory-status-rich-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Status"));
    assert!(stdout.contains("instance: global-main"));
    assert!(stdout.contains("projection stale: false"));
}

#[test]
fn cli_memory_status_plain_has_no_ansi() {
    let root = temp_root("memory-status-plain-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "plain")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status"));
    assert!(!stdout.contains("\u{1b}["));
    assert!(!stdout.contains('╭'));
}

#[test]
fn cli_memory_status_json_outputs_valid_json() {
    let root = temp_root("memory-status-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_status_no_color_has_no_ansi() {
    let root = temp_root("memory-status-no-color-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "status", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich"), ("NO_COLOR", "1")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status"));
    assert!(!stdout.contains("\u{1b}["));
}
#[test]
fn memory_search_returns_remembered_record() {
    let root = temp_root("memory-search");
    let remember_output = run_cli_with_config_root(
        &["memory", "remember", "TUI 是类似 Codex 的终端 AI 编程界面"],
        &root,
    );
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "TUI", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索（local direct）："));
    assert!(stdout.contains("mem_"));
    assert!(stdout.contains("[core]"));
    assert!(stdout.contains("TUI 是类似 Codex 的终端 AI 编程界面"));
}

#[test]
fn cli_memory_search_rich_uses_terminal_panel_or_table() {
    let root = temp_root("memory-search-rich-terminal");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "rich search memory",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "search", "rich", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ 记忆搜索"));
    assert!(stdout.contains("rich search memory"));
    assert!(stdout.contains("score:"));
    assert!(stdout.contains("matched:"));
}

#[test]
fn cli_memory_search_json_outputs_valid_json() {
    let root = temp_root("memory-search-json-terminal");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "json search memory",
    );

    let output = run_cli_with_config_root_and_env(
        &["memory", "search", "json", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(stdout.contains("json search memory"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_memory_search_empty_result_json_outputs_valid_json() {
    let root = temp_root("memory-search-empty-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "search", "missing", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_search_uses_real_config_root() {
    let root_with_memory = temp_root("memory-search-root-a");
    let other_root = temp_root("memory-search-root-b");

    let remember_output = run_cli_with_config_root(
        &["memory", "remember", "只写在 root a 的记忆"],
        &root_with_memory,
    );
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "root a", "--local"], &other_root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索（local direct）："));
    assert!(stdout.contains("无匹配记忆"));
}

#[test]
fn memory_search_accepts_type_filter() {
    let root = temp_root("memory-search-type-filter");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "type filter shared",
    );
    let _ = seed_memory_record(
        &root,
        MemoryType::Decision,
        MemoryPermanence::Standard,
        "type filter shared",
    );

    let output = run_cli_with_config_root(
        &["memory", "search", "type", "--type", "decision", "--local"],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("[decision]"));
    assert!(!stdout.contains("[core]"));
}

#[test]
fn memory_search_accepts_source_filter() {
    let root = temp_root("memory-search-source-filter");
    let proposal_id = seed_rule_based_proposal(
        &root,
        MemoryTrigger::ExplicitRemember,
        "记住：source filter shared",
    );
    let _ = run_cli_with_config_root(&["memory", "accept", &proposal_id], &root);

    let output = run_cli_with_config_root(
        &[
            "memory",
            "search",
            "source",
            "--source",
            "rule_based_agent",
            "--local",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("source: rule_based_agent"));
}

#[test]
fn memory_search_accepts_permanence_filter() {
    let root = temp_root("memory-search-permanence-filter");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "permanence shared",
    );
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Permanent,
        "permanence shared",
    );

    let output = run_cli_with_config_root(
        &[
            "memory",
            "search",
            "permanence",
            "--permanence",
            "standard",
            "--local",
        ],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("permanence: standard"));
    assert!(!stdout.contains("permanence: permanent"));
}

#[test]
fn memory_search_accepts_limit() {
    let root = temp_root("memory-search-limit");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "limit shared a",
    );
    let _ = seed_memory_record(
        &root,
        MemoryType::Decision,
        MemoryPermanence::Standard,
        "limit shared b",
    );

    let output = run_cli_with_config_root(
        &["memory", "search", "limit", "--limit", "1", "--local"],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let count = stdout.matches("- mem_").count();
    assert_eq!(count, 1);
}

#[test]
fn memory_search_rejects_unknown_type() {
    let root = temp_root("memory-search-bad-type");
    let output = run_cli_with_config_root(
        &["memory", "search", "x", "--type", "unknown", "--local"],
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --type"));
}

#[test]
fn memory_search_rejects_unknown_source() {
    let root = temp_root("memory-search-bad-source");
    let output = run_cli_with_config_root(
        &["memory", "search", "x", "--source", "unknown", "--local"],
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --source"));
}

#[test]
fn memory_search_rejects_unknown_permanence() {
    let root = temp_root("memory-search-bad-permanence");
    let output = run_cli_with_config_root(
        &[
            "memory",
            "search",
            "x",
            "--permanence",
            "unknown",
            "--local",
        ],
        &root,
    );

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --permanence"));
}

#[test]
fn memory_search_rejects_invalid_limit() {
    let root = temp_root("memory-search-bad-limit");
    let output =
        run_cli_with_config_root(&["memory", "search", "x", "--limit", "0", "--local"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("--limit 必须是正整数"));
}

#[test]
fn memory_search_default_behavior_still_works() {
    let root = temp_root("memory-search-default-compatible");
    let remember_output =
        run_cli_with_config_root(&["memory", "remember", "default behavior memory"], &root);
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "default", "--local"], &root);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("default behavior memory"));
}

#[test]
fn memory_search_output_includes_score_and_matched_fields() {
    let root = temp_root("memory-search-score-fields");
    let remember_output =
        run_cli_with_config_root(&["memory", "remember", "score fields memory"], &root);
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "score", "--local"], &root);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("score:"));
    assert!(stdout.contains("matched:"));
    assert!(stdout.contains("source:"));
    assert!(stdout.contains("permanence:"));
}

#[test]
fn memory_search_filters_do_not_return_archived_records() {
    let root = temp_root("memory-search-archived-filter");
    let remember_output =
        run_cli_with_config_root(&["memory", "remember", "archived filter memory"], &root);
    assert!(remember_output.status.success());

    let kernel =
        MemoryKernel::open(memory_paths_for_root(&root)).expect("memory kernel should open");
    let memory_id = kernel.records()[0].memory_id.clone();
    drop(kernel);

    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(&root)).expect("memory kernel should reopen");
    kernel.archive(&memory_id).expect("archive should succeed");

    let output = run_cli_with_config_root(&["memory", "search", "archived", "--local"], &root);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("无匹配记忆"));
}

#[test]
fn memory_search_empty_result_prints_friendly_message() {
    let root = temp_root("memory-empty-search");
    let output = run_cli_with_config_root(&["memory", "search", "missing", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索（local direct）："));
    assert!(stdout.contains("无匹配记忆"));
}

#[test]
fn memory_status_shows_memory_root() {
    let root = temp_root("memory-status-root");
    let output = run_cli_with_config_root(&["memory", "status", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status（local direct）："));
    assert!(stdout.contains(&format!(
        "root: {}",
        root.join("instances")
            .join("global-main")
            .join("memory")
            .display()
    )));
}

#[test]
fn memory_status_shows_projection_metadata() {
    let root = temp_root("memory-status-projection-meta");
    let output = run_cli_with_config_root(&["memory", "status", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("projection stale: false"));
    assert!(stdout.contains("projection warning: <none>"));
    assert!(stdout.contains("last rebuild at: <none>"));
}

#[test]
fn memory_audit_command_succeeds() {
    let root = temp_root("memory-audit");
    let output = run_cli_with_config_root(&["memory", "audit", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Audit（local direct）："));
    assert!(stdout.contains("checked events: 0"));
    assert!(stdout.contains("status: ok"));
}

#[test]
fn cli_memory_audit_rich_uses_terminal_diagnostic_or_panel() {
    let root = temp_root("memory-audit-rich-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "audit", "--local"],
        &root,
        &[("AICORE_TERMINAL", "rich")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("╭─ Memory Audit"));
    assert!(stdout.contains("checked events: 0"));
    assert!(stdout.contains("status: ok"));
}

#[test]
fn cli_memory_audit_json_outputs_valid_json() {
    let root = temp_root("memory-audit-json-terminal");
    let output = run_cli_with_config_root_and_env(
        &["memory", "audit", "--local"],
        &root,
        &[("AICORE_TERMINAL", "json")],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);

    assert!(
        events
            .iter()
            .any(|event| event["event"] == "direct.command.result")
    );
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn memory_audit_reports_ok_for_valid_memory_store() {
    let root = temp_root("memory-audit-valid");
    let remember_output = run_cli_with_config_root(&["memory", "remember", "测试记忆审计"], &root);
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "audit", "--local"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Audit（local direct）："));
    assert!(stdout.contains("checked events: 1"));
    assert!(stdout.contains("status: ok"));
}
