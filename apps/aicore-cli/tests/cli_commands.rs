use std::{fs, path::PathBuf, process::Command};

use aicore_memory::{
    MemoryAgentOutput, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryProposal,
    MemoryProposalStatus, MemoryRequestedOutput, MemoryScope, MemorySource, MemoryTrigger,
    MemoryType, MemoryWorkBatch, RememberInput, RuleBasedMemoryAgent,
};

fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("aicore-cli-p46-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    root
}

fn run_cli_with_config_root(args: &[&str], root: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(args)
        .env("AICORE_CONFIG_ROOT", root)
        .output()
        .expect("aicore-cli should run")
}

fn memory_paths_for_root(root: &PathBuf) -> MemoryPaths {
    MemoryPaths::new(root.join("instances").join("global-main").join("memory"))
}

fn seed_open_proposal(root: &PathBuf, memory_type: MemoryType, content: &str) -> String {
    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![MemoryProposal {
                proposal_id: "agent_prop_seed".to_string(),
                memory_type,
                scope: MemoryScope::GlobalMain {
                    instance_id: "global-main".to_string(),
                },
                source: MemorySource::RuleBasedAgent,
                status: MemoryProposalStatus::Rejected,
                content: content.to_string(),
                content_language: if content.is_ascii() {
                    "en".to_string()
                } else {
                    "zh-CN".to_string()
                },
                normalized_content: content.to_string(),
                normalized_language: if content.is_ascii() {
                    "en".to_string()
                } else {
                    "zh-CN".to_string()
                },
                localized_summary: content.to_string(),
                created_at: "0".to_string(),
            }],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")
        .into_iter()
        .next()
        .expect("proposal id should exist")
}

fn global_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

fn seed_rule_based_proposal(root: &PathBuf, trigger: MemoryTrigger, excerpt: &str) -> String {
    let output = RuleBasedMemoryAgent::analyze(&MemoryWorkBatch {
        instance_id: "global-main".to_string(),
        scope: global_scope(),
        trigger,
        recent_events_summary: String::new(),
        raw_excerpts: vec![excerpt.to_string()],
        existing_memory_hits: Vec::new(),
        token_budget: 1024,
        requested_outputs: vec![MemoryRequestedOutput::Proposals],
    });

    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .submit_agent_output(output)
        .expect("agent output should be stored")
        .into_iter()
        .next()
        .expect("proposal id should exist")
}

fn seed_memory_record(
    root: &PathBuf,
    memory_type: MemoryType,
    permanence: MemoryPermanence,
    content: &str,
) -> String {
    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type,
            permanence,
            scope: global_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed")
}

#[test]
fn renders_status_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .arg("status")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore CLI"));
    assert!(stdout.contains("主实例：global-main"));
    assert!(stdout.contains("组件数量："));
    assert!(stdout.contains("实例数量："));
    assert!(stdout.contains("Runtime：global-main/main"));
}

#[test]
fn renders_instance_list_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["instance", "list"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例列表："));
    assert!(stdout.contains("global-main"));
    assert!(stdout.contains("global_main"));
}

#[test]
fn renders_runtime_smoke_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["runtime", "smoke"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Runtime Smoke："));
    assert!(stdout.contains("CLI 场景："));
    assert!(stdout.contains("接收决策：StartTurn"));
    assert!(stdout.contains("账本消息数：2"));
    assert!(stdout.contains("输出目标：active-views"));
    assert!(stdout.contains("投递身份：active-views"));
    assert!(stdout.contains("External Origin 场景："));
    assert!(stdout.contains("输出目标：origin"));
    assert!(stdout.contains("投递身份：external:feishu:chat-1"));
    assert!(stdout.contains("Follow 场景："));
    assert!(stdout.contains("输出目标：followed-external"));
    assert!(stdout.contains("投递身份：external:feishu:chat-2"));
}

#[test]
fn renders_config_smoke_command() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "smoke"])
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置 Smoke Test："));
    assert!(stdout.contains("默认配置文件：通过"));
    assert!(stdout.contains("认证池保存/读取：通过"));
    assert!(stdout.contains("实例运行配置保存/读取：通过"));
    assert!(stdout.contains("服务角色配置保存/读取：通过"));
    assert!(stdout.contains("配置校验：通过"));
}

#[test]
fn memory_wiki_defaults_to_index() {
    let root = temp_root("memory-wiki-index");
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "wiki index memory",
    );

    let output = run_cli_with_config_root(&["memory", "wiki"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆 Wiki Projection："));
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

    let output = run_cli_with_config_root(&["memory", "wiki", "core"], &root);

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

    let output = run_cli_with_config_root(&["memory", "wiki", "decisions"], &root);

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

    let output = run_cli_with_config_root(&["memory", "wiki", "status"], &root);

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

    let output = run_cli_with_config_root(&["memory", "wiki", "core.md"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("- page: core"));
}

#[test]
fn memory_wiki_rejects_unknown_page() {
    let root = temp_root("memory-wiki-unknown");
    let output = run_cli_with_config_root(&["memory", "wiki", "unknown"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("未知 Wiki 页面"));
}

#[test]
fn memory_wiki_rejects_path_traversal() {
    let root = temp_root("memory-wiki-traversal");
    let output = run_cli_with_config_root(&["memory", "wiki", "../../secret"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("不允许读取任意 Wiki 路径"));
}

#[test]
fn memory_wiki_reports_missing_projection() {
    let root = temp_root("memory-wiki-missing");
    let output = run_cli_with_config_root(&["memory", "wiki"], &root);

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

    let output = run_cli_with_config_root(&["memory", "wiki", "index"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("这是 generated projection"));
    assert!(stdout.contains("不是事实来源"));
    assert!(stdout.contains("不应手工编辑后期待反向同步"));
}

#[test]
fn auth_list_reads_real_config_root() {
    let root = temp_root("auth-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["auth", "list"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("认证池："));
    assert!(stdout.contains("auth.openrouter.main"));
    assert!(stdout.contains("provider: openrouter"));
    assert!(stdout.contains("kind: api_key"));
    assert!(stdout.contains("enabled: true"));
    assert!(stdout.contains("capabilities: chat, vision"));
    assert!(stdout.contains("secret_ref: secret://auth.openrouter.main"));
}

#[test]
fn model_show_reads_real_config_root() {
    let root = temp_root("model-show-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["model", "show"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("实例模型配置："));
    assert!(stdout.contains("instance: global-main"));
    assert!(stdout.contains("primary:"));
    assert!(stdout.contains("auth_ref: auth.openrouter.main"));
    assert!(stdout.contains("model: openai/gpt-5"));
    assert!(stdout.contains("fallback:"));
    assert!(stdout.contains("auth_ref: auth.openai.backup"));
    assert!(stdout.contains("model: gpt-4.1"));
}

#[test]
fn service_list_reads_real_config_root() {
    let root = temp_root("service-list-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["service", "list"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("服务角色配置："));
    assert!(stdout.contains("memory_dreamer"));
    assert!(stdout.contains("mode: inherit_instance"));
    assert!(stdout.contains("evolution_reviewer"));
    assert!(stdout.contains("mode: disabled"));
    assert!(stdout.contains("search"));
    assert!(stdout.contains("mode: explicit"));
    assert!(stdout.contains("auth_ref: auth.openrouter.search"));
    assert!(stdout.contains("model: perplexity/sonar"));
}

#[test]
fn auth_list_fails_when_config_missing() {
    let root = temp_root("auth-list-missing");
    let output = run_cli_with_config_root(&["auth", "list"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少认证池配置，请先运行 config init。"));
}

#[test]
fn model_show_fails_when_runtime_missing() {
    let root = temp_root("model-show-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["model", "show"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn service_list_fails_when_services_missing() {
    let root = temp_root("service-list-missing");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");

    let output = run_cli_with_config_root(&["service", "list"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少服务角色配置，请先运行 config init。"));
}

#[test]
fn provider_smoke_reads_real_config_root() {
    let root = temp_root("provider-smoke-real-root");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Provider Smoke："));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("auth_ref：auth.openrouter.main"));
    assert!(stdout.contains("model：openai/gpt-5"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider response：通过"));
    assert!(stdout.contains("runtime output：通过"));
}

#[test]
fn cli_agent_smoke_runs() {
    let root = temp_root("agent-smoke-runs");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["agent", "smoke", "agent smoke request"], &root);

    assert!(output.status.success());
}

#[test]
fn cli_agent_smoke_outputs_chinese_status() {
    let root = temp_root("agent-smoke-chinese-status");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(&["agent", "smoke", "继续实现"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Loop：通过"));
    assert!(stdout.contains("实例：global-main"));
    assert!(stdout.contains("runtime output：已追加"));
}

#[test]
fn cli_agent_smoke_reports_memory_prompt_provider_runtime_status() {
    let root = temp_root("agent-smoke-status-lines");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "agent loop memory context",
    );

    let output = run_cli_with_config_root(&["agent", "smoke", "agent loop"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("memory pack："));
    assert!(stdout.contains("prompt builder：通过"));
    assert!(stdout.contains("outcome：completed"));
    assert!(stdout.contains("ingress source：cli"));
    assert!(stdout.contains("provider invoked：yes"));
    assert!(stdout.contains("provider：dummy"));
    assert!(stdout.contains("provider name：openrouter"));
    assert!(stdout.contains("assistant output present：yes"));
    assert!(stdout.contains("failure stage：<none>"));
    assert!(stdout.contains("runtime output：已追加"));
    assert!(stdout.contains("event count："));
    assert!(stdout.contains("queue len：0"));
}

#[test]
fn cli_agent_smoke_does_not_print_prompt() {
    let root = temp_root("agent-smoke-no-prompt");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());
    let _ = seed_memory_record(
        &root,
        MemoryType::Core,
        MemoryPermanence::Standard,
        "sensitive prompt context should stay internal",
    );

    let output = run_cli_with_config_root(&["agent", "smoke", "please answer"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("SYSTEM:"));
    assert!(!stdout.contains("CURRENT USER REQUEST:"));
    assert!(!stdout.contains("sensitive prompt context should stay internal"));
}

#[test]
fn cli_agent_session_smoke_runs() {
    let root = temp_root("agent-session-smoke-runs");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &["agent", "session-smoke", "第一轮请求", "第二轮请求"],
        &root,
    );

    assert!(output.status.success());
}

#[test]
fn cli_agent_session_smoke_outputs_chinese_summary() {
    let root = temp_root("agent-session-smoke-summary");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &["agent", "session-smoke", "第一轮请求", "第二轮请求"],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Agent Session：通过"));
    assert!(stdout.contains("conversation："));
    assert!(stdout.contains("turns：2"));
    assert!(stdout.contains("latest outcome：completed"));
    assert!(stdout.contains("turn 1 outcome：completed"));
    assert!(stdout.contains("turn 2 outcome：completed"));
}

#[test]
fn cli_agent_session_smoke_does_not_print_prompt() {
    let root = temp_root("agent-session-smoke-no-prompt");
    let init_output = run_cli_with_config_root(&["config", "init"], &root);
    assert!(init_output.status.success());

    let output = run_cli_with_config_root(
        &["agent", "session-smoke", "第一轮请求", "第二轮请求"],
        &root,
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("SYSTEM:"));
    assert!(!stdout.contains("CURRENT USER REQUEST:"));
}

#[test]
fn provider_smoke_fails_when_auth_missing() {
    let root = temp_root("provider-smoke-missing-auth");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少认证池配置，请先运行 config init。"));
}

#[test]
fn cli_agent_smoke_provider_resolve_failure_prints_chinese_error() {
    let root = temp_root("agent-smoke-missing-auth");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(
        root.join("auth.toml"),
        r#"# AICore OS auth pool

[[auth]]
auth_ref = "auth.someone.else"
provider = "openrouter"
kind = "api_key"
secret_ref = "secret://auth.someone.else"
capabilities = ["chat"]
enabled = true
"#,
    )
    .expect("auth.toml should be writable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        r#"instance_id = "global-main"
primary_auth_ref = "auth.openrouter.main"
primary_model = "openai/gpt-5"
"#,
    )
    .expect("runtime.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["agent", "smoke", "需要失败"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("配置命令失败"));
    assert!(stderr.contains("Agent Turn 失败"));
    assert!(stderr.contains("provider_resolve"));
}

#[test]
fn provider_smoke_fails_when_runtime_missing() {
    let root = temp_root("provider-smoke-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = run_cli_with_config_root(&["provider", "smoke"], &root);

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn memory_status_command_succeeds() {
    let root = temp_root("memory-status");
    let output = run_cli_with_config_root(&["memory", "status"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status："));
    assert!(stdout.contains("instance: global-main"));
    assert!(stdout.contains("records: 0"));
    assert!(stdout.contains("proposals: 0"));
    assert!(stdout.contains("events: 0"));
    assert!(stdout.contains("projection stale: false"));
}

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
fn memory_search_returns_remembered_record() {
    let root = temp_root("memory-search");
    let remember_output = run_cli_with_config_root(
        &["memory", "remember", "TUI 是类似 Codex 的终端 AI 编程界面"],
        &root,
    );
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "search", "TUI"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索："));
    assert!(stdout.contains("mem_"));
    assert!(stdout.contains("[core]"));
    assert!(stdout.contains("TUI 是类似 Codex 的终端 AI 编程界面"));
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

    let output = run_cli_with_config_root(&["memory", "search", "root a"], &other_root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索："));
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

    let output =
        run_cli_with_config_root(&["memory", "search", "type", "--type", "decision"], &root);

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
        &["memory", "search", "source", "--source", "rule_based_agent"],
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
        &["memory", "search", "permanence", "--permanence", "standard"],
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

    let output = run_cli_with_config_root(&["memory", "search", "limit", "--limit", "1"], &root);

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let count = stdout.matches("- mem_").count();
    assert_eq!(count, 1);
}

#[test]
fn memory_search_rejects_unknown_type() {
    let root = temp_root("memory-search-bad-type");
    let output = run_cli_with_config_root(&["memory", "search", "x", "--type", "unknown"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --type"));
}

#[test]
fn memory_search_rejects_unknown_source() {
    let root = temp_root("memory-search-bad-source");
    let output = run_cli_with_config_root(&["memory", "search", "x", "--source", "unknown"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --source"));
}

#[test]
fn memory_search_rejects_unknown_permanence() {
    let root = temp_root("memory-search-bad-permanence");
    let output =
        run_cli_with_config_root(&["memory", "search", "x", "--permanence", "unknown"], &root);

    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("无效的 --permanence"));
}

#[test]
fn memory_search_rejects_invalid_limit() {
    let root = temp_root("memory-search-bad-limit");
    let output = run_cli_with_config_root(&["memory", "search", "x", "--limit", "0"], &root);

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

    let output = run_cli_with_config_root(&["memory", "search", "default"], &root);
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

    let output = run_cli_with_config_root(&["memory", "search", "score"], &root);
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

    let output = run_cli_with_config_root(&["memory", "search", "archived"], &root);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("无匹配记忆"));
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
fn memory_search_empty_result_prints_friendly_message() {
    let root = temp_root("memory-empty-search");
    let output = run_cli_with_config_root(&["memory", "search", "missing"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("记忆搜索："));
    assert!(stdout.contains("无匹配记忆"));
}

#[test]
fn memory_status_shows_memory_root() {
    let root = temp_root("memory-status-root");
    let output = run_cli_with_config_root(&["memory", "status"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Status："));
    assert!(stdout.contains(&format!(
        "root: {}",
        root.join("instances").join("global-main").join("memory").display()
    )));
}

#[test]
fn memory_status_shows_projection_metadata() {
    let root = temp_root("memory-status-projection-meta");
    let output = run_cli_with_config_root(&["memory", "status"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("projection stale: false"));
    assert!(stdout.contains("projection warning: <none>"));
    assert!(stdout.contains("last rebuild at: <none>"));
}

#[test]
fn memory_audit_command_succeeds() {
    let root = temp_root("memory-audit");
    let output = run_cli_with_config_root(&["memory", "audit"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Audit："));
    assert!(stdout.contains("checked events: 0"));
    assert!(stdout.contains("status: ok"));
}

#[test]
fn memory_audit_reports_ok_for_valid_memory_store() {
    let root = temp_root("memory-audit-valid");
    let remember_output = run_cli_with_config_root(&["memory", "remember", "测试记忆审计"], &root);
    assert!(remember_output.status.success());

    let output = run_cli_with_config_root(&["memory", "audit"], &root);

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("Memory Audit："));
    assert!(stdout.contains("checked events: 1"));
    assert!(stdout.contains("status: ok"));
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

#[test]
fn renders_config_path_command() {
    let root = temp_root("config-path");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置路径："));
    assert!(stdout.contains(&format!("root: {}", root.display())));
    assert!(stdout.contains(&format!("auth.toml: {}", root.join("auth.toml").display())));
    assert!(stdout.contains(&format!(
        "services.toml: {}",
        root.join("services.toml").display()
    )));
    assert!(stdout.contains(&format!("instances: {}", root.join("instances").display())));
    assert!(stdout.contains(&format!(
        "global-main runtime: {}",
        root.join("instances").join("global-main").join("runtime.toml").display()
    )));
}

#[test]
fn config_path_uses_default_home_root_without_override() {
    let home = temp_root("config-path-home");
    let expected_root = home.join(".aicore").join("config");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "path"])
        .env("HOME", &home)
        .env_remove("AICORE_CONFIG_ROOT")
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains(&format!("root: {}", expected_root.display())));
}

#[test]
fn config_init_creates_real_config_files_under_override_root() {
    let root = temp_root("config-init");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert!(root.join("auth.toml").exists());
    assert!(root.join("services.toml").exists());
    assert!(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml")
            .exists()
    );
}

#[test]
fn config_init_does_not_overwrite_existing_files() {
    let root = temp_root("config-init-no-overwrite");
    fs::create_dir_all(root.join("instances").join("global-main"))
        .expect("config directories should be creatable");
    fs::write(root.join("auth.toml"), "sentinel-auth").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "sentinel-services")
        .expect("services.toml should be writable");
    fs::write(
        root.join("instances")
            .join("global-main")
            .join("runtime.toml"),
        "sentinel-runtime",
    )
    .expect("runtime.toml should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(root.join("auth.toml")).expect("auth.toml should remain readable"),
        "sentinel-auth"
    );
    assert_eq!(
        fs::read_to_string(root.join("services.toml"))
            .expect("services.toml should remain readable"),
        "sentinel-services"
    );
    assert_eq!(
        fs::read_to_string(
            root.join("instances")
                .join("global-main")
                .join("runtime.toml")
        )
        .expect("runtime.toml should remain readable"),
        "sentinel-runtime"
    );
}

#[test]
fn config_validate_accepts_initialized_config() {
    let root = temp_root("config-validate-ok");

    let init_output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "init"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");
    assert!(init_output.status.success());

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("配置校验："));
    assert!(stdout.contains("认证池：已读取"));
    assert!(stdout.contains("实例运行配置：通过"));
    assert!(stdout.contains("服务角色配置：通过"));
}

#[test]
fn config_validate_fails_when_runtime_missing() {
    let root = temp_root("config-validate-missing-runtime");
    fs::create_dir_all(&root).expect("config root should be creatable");
    fs::write(root.join("auth.toml"), "").expect("auth.toml should be writable");
    fs::write(root.join("services.toml"), "").expect("services.toml should be writable");

    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "validate"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(!output.status.success());

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("缺少 global-main runtime 配置，请先运行 config init 或配置模型。"));
}

#[test]
fn config_smoke_still_uses_temp_demo_root() {
    let root = temp_root("config-smoke-real-root");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(["config", "smoke"])
        .env("AICORE_CONFIG_ROOT", &root)
        .output()
        .expect("aicore-cli should run");

    assert!(output.status.success());
    assert!(!root.exists());
}
