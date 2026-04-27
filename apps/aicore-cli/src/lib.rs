use std::{env, fs, path::PathBuf};

use aicore_agent::{
    AgentSessionRunner, AgentSessionStopReason, AgentTurnInput, AgentTurnOutcome, AgentTurnRunner,
};
use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{
    ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ServiceProfile, ServiceProfileMode, ServiceRole,
};
use aicore_kernel::default_control_plane;
use aicore_kernel::{
    DeliveryIdentity, GatewaySource, InterruptMode, OutputTarget, TransportEnvelope,
    default_runtime,
};
use aicore_memory::{
    MemoryAuditReport, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemorySource,
    MemoryType, RememberInput, SearchQuery,
};
use aicore_provider::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderError, ProviderInvoker, ProviderResolver,
};

pub fn run_from_args(args: Vec<String>) -> i32 {
    match args.as_slice() {
        [cmd] if cmd == "status" => {
            print_status();
            0
        }
        [group, action] if group == "instance" && action == "list" => {
            print_instance_list();
            0
        }
        [group, action] if group == "runtime" && action == "smoke" => {
            print_runtime_smoke();
            0
        }
        [group, action] if group == "config" && action == "smoke" => {
            run_config_command(print_config_smoke)
        }
        [group, action] if group == "config" && action == "path" => {
            run_config_command(print_config_path)
        }
        [group, action] if group == "config" && action == "init" => {
            run_config_command(print_config_init)
        }
        [group, action] if group == "config" && action == "validate" => {
            run_config_command(print_config_validate)
        }
        [group, action] if group == "auth" && action == "list" => {
            run_config_command(print_auth_list)
        }
        [group, action] if group == "model" && action == "show" => {
            run_config_command(print_model_show)
        }
        [group, action] if group == "service" && action == "list" => {
            run_config_command(print_service_list)
        }
        [group, action] if group == "provider" && action == "smoke" => {
            run_config_command(print_provider_smoke)
        }
        [group, action, content] if group == "agent" && action == "smoke" => {
            run_config_command_with_arg(content, print_agent_smoke)
        }
        [group, action, first, second] if group == "agent" && action == "session-smoke" => {
            run_config_command_with_two_args(first, second, print_agent_session_smoke)
        }
        [group, action] if group == "memory" && action == "status" => {
            run_memory_command(print_memory_status)
        }
        [group, action] if group == "memory" && action == "audit" => {
            run_memory_command(print_memory_audit)
        }
        [group, action] if group == "memory" && action == "proposals" => {
            run_memory_command(print_memory_proposals)
        }
        [group, action] if group == "memory" && action == "wiki" => {
            run_memory_command(print_memory_wiki_index)
        }
        [group, action, content] if group == "memory" && action == "remember" => {
            run_memory_command_with_arg(content, print_memory_remember)
        }
        [group, action, page] if group == "memory" && action == "wiki" => {
            run_memory_command_with_arg(page, print_memory_wiki_page)
        }
        [group, action, query, rest @ ..] if group == "memory" && action == "search" => {
            run_memory_search_command(query, rest)
        }
        [group, action, proposal_id] if group == "memory" && action == "accept" => {
            run_memory_command_with_arg(proposal_id, print_memory_accept)
        }
        [group, action, proposal_id] if group == "memory" && action == "reject" => {
            run_memory_command_with_arg(proposal_id, print_memory_reject)
        }
        [group, _] if group == "config" => {
            eprintln!("未知 config 命令。");
            eprintln!("可用命令：config smoke | config path | config init | config validate");
            1
        }
        [group, _] if group == "memory" => {
            eprintln!("未知 memory 命令。");
            eprintln!(
                "可用命令：memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
        [group, _] if group == "agent" => {
            eprintln!("未知 agent 命令。");
            eprintln!(
                "可用命令：agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容>"
            );
            1
        }
        _ => {
            eprintln!("未知命令。");
            eprintln!(
                "可用命令：status | instance list | runtime smoke | config smoke | config path | config init | config validate | auth list | model show | service list | provider smoke | agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容> | memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
    }
}

fn print_status() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    println!("AICore CLI");
    println!("主实例：{}", main_instance.id);
    println!("组件数量：{}", control_summary.component_count);
    println!("实例数量：{}", control_summary.instance_count);
    println!(
        "Runtime：{}/{}",
        runtime_summary.instance_id, runtime_summary.conversation_id
    );
}

fn print_instance_list() {
    let control_plane = default_control_plane();

    println!("实例列表：");
    for instance in control_plane.instance_registry().list() {
        let kind = match instance.kind {
            aicore_kernel::InstanceKind::GlobalMain => "global_main",
            aicore_kernel::InstanceKind::Workspace => "workspace",
        };

        println!(
            "- {} [{}] {}",
            instance.id.as_str(),
            kind,
            instance.workspace_root.display()
        );
    }
}

fn print_runtime_smoke() {
    let mut cli_runtime = default_runtime();
    let cli_ingress = cli_runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        "hello",
        InterruptMode::Queue,
    );
    let cli_routed = cli_runtime.append_assistant_output("reply");
    let cli_first = cli_routed
        .events
        .first()
        .expect("runtime smoke must have at least one output");

    let mut external_runtime = default_runtime();
    external_runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-1".to_string()),
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        },
        "hello from external",
        InterruptMode::Queue,
    );
    let external_routed = external_runtime.append_assistant_output("reply external");
    let external_origin = external_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::Origin)
        .expect("external origin smoke must include origin output");

    let mut follow_runtime = default_runtime();
    follow_runtime.follow_external(TransportEnvelope {
        source: GatewaySource::External,
        platform: Some("feishu".to_string()),
        target_id: Some("chat-2".to_string()),
        sender_id: Some("user-2".to_string()),
        is_group: true,
        mentioned_bot: true,
    });
    let follow_routed = follow_runtime.append_assistant_output("reply followed");
    let followed_external = follow_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::FollowedExternal)
        .expect("follow smoke must include followed external output");

    println!("Runtime Smoke：");
    println!("CLI 场景：");
    println!("  接收决策：{:?}", cli_ingress.decision);
    println!("  账本消息数：{}", cli_runtime.summary().event_count);
    println!("  输出目标：{}", output_target_name(&cli_first.target));
    println!(
        "  投递身份：{}",
        delivery_identity_name(&cli_first.identity)
    );
    println!("External Origin 场景：");
    println!(
        "  输出目标：{}",
        output_target_name(&external_origin.target)
    );
    println!(
        "  投递身份：{}",
        delivery_identity_name(&external_origin.identity)
    );
    println!("Follow 场景：");
    println!(
        "  输出目标：{}",
        output_target_name(&followed_external.target)
    );
    println!(
        "  投递身份：{}",
        delivery_identity_name(&followed_external.identity)
    );
}

fn run_config_command(command: fn() -> Result<(), String>) -> i32 {
    match command() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_config_command_with_arg(arg: &str, command: fn(&str) -> Result<(), String>) -> i32 {
    match command(arg) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_config_command_with_two_args(
    first: &str,
    second: &str,
    command: fn(&str, &str) -> Result<(), String>,
) -> i32 {
    match command(first, second) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_memory_command(command: fn() -> Result<(), String>) -> i32 {
    match command() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn run_memory_command_with_arg(arg: &str, command: fn(&str) -> Result<(), String>) -> i32 {
    match command(arg) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn run_memory_search_command(query: &str, args: &[String]) -> i32 {
    match parse_memory_search_options(args).and_then(|options| print_memory_search(query, options))
    {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn print_config_smoke() -> Result<(), String> {
    let store = prepare_demo_config_store("config-smoke")?;

    let auth_toml_exists = store.paths.auth_toml.exists();
    let services_toml_exists = store.paths.services_toml.exists();
    let runtime_toml_exists = store.paths.runtime_toml_for("global-main").exists();

    let loaded_auth_pool = store.load_auth_pool().map_err(config_error)?;
    let loaded_runtime = store
        .load_instance_runtime("global-main")
        .map_err(config_error)?;
    let loaded_services = store.load_services().map_err(config_error)?;

    if !(auth_toml_exists && services_toml_exists && runtime_toml_exists) {
        return Err("默认配置文件未完整创建".to_string());
    }

    if loaded_auth_pool != demo_auth_pool() {
        return Err("认证池读取结果与写入内容不一致".to_string());
    }

    if loaded_runtime != demo_runtime_config() {
        return Err("实例运行配置读取结果与写入内容不一致".to_string());
    }

    if loaded_services != demo_service_profiles() {
        return Err("服务角色配置读取结果与写入内容不一致".to_string());
    }

    ConfigStore::validate_runtime_config(&loaded_runtime, &loaded_auth_pool)
        .map_err(config_error)?;
    ConfigStore::validate_service_profiles(&loaded_services, &loaded_auth_pool)
        .map_err(config_error)?;

    println!("配置 Smoke Test：");
    println!("- 默认配置文件：通过");
    println!("- 认证池保存/读取：通过");
    println!("- 实例运行配置保存/读取：通过");
    println!("- 服务角色配置保存/读取：通过");
    println!("- 配置校验：通过");

    Ok(())
}

fn print_config_path() -> Result<(), String> {
    let paths = real_config_paths()?;

    println!("配置路径：");
    println!("root: {}", paths.root.display());
    println!("auth.toml: {}", paths.auth_toml.display());
    println!("services.toml: {}", paths.services_toml.display());
    println!("instances: {}", paths.instances_dir.display());
    println!(
        "global-main runtime: {}",
        paths.runtime_toml_for("global-main").display()
    );

    Ok(())
}

fn print_config_init() -> Result<(), String> {
    let store = real_config_store()?;
    let status = initialize_real_config(&store)?;

    println!("配置初始化：");
    println!("- root: {}", store.paths.root.display());
    println!("- auth.toml: {}", init_status_name(status.auth_created));
    println!(
        "- services.toml: {}",
        init_status_name(status.services_created)
    );
    println!(
        "- global-main runtime.toml: {}",
        init_status_name(status.runtime_created)
    );

    Ok(())
}

fn print_config_validate() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = store.load_auth_pool().map_err(config_error)?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let services = store.load_services().map_err(config_error)?;

    ConfigStore::validate_runtime_config(&runtime, &auth_pool).map_err(config_error)?;
    ConfigStore::validate_service_profiles(&services, &auth_pool).map_err(config_error)?;

    println!("配置校验：");
    println!("- 认证池：已读取");
    println!("- 实例运行配置：通过");
    println!("- 服务角色配置：通过");

    Ok(())
}

fn print_auth_list() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;

    println!("认证池：");
    for entry in auth_pool.available_entries() {
        println!("- {}", entry.auth_ref.as_str());
        println!("  provider: {}", entry.provider);
        println!("  kind: {}", auth_kind_name(&entry.kind));
        println!("  enabled: {}", entry.enabled);
        println!(
            "  capabilities: {}",
            entry
                .capabilities
                .iter()
                .map(auth_capability_name)
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("  secret_ref: {}", entry.secret_ref.as_str());
    }

    Ok(())
}

fn print_model_show() -> Result<(), String> {
    let store = real_config_store()?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;

    println!("实例模型配置：");
    println!("instance: {}", runtime.instance_id);
    println!();
    println!("primary:");
    println!("  auth_ref: {}", runtime.primary.auth_ref.as_str());
    println!("  model: {}", runtime.primary.model);
    println!();
    println!("fallback:");

    if let Some(fallback) = runtime.fallback {
        println!("  auth_ref: {}", fallback.auth_ref.as_str());
        println!("  model: {}", fallback.model);
    } else {
        println!("  未配置");
    }

    Ok(())
}

fn print_service_list() -> Result<(), String> {
    let store = real_config_store()?;
    let services = load_real_services(&store)?;

    println!("服务角色配置：");
    for profile in services.profiles {
        println!("- {}", service_role_name(&profile.role));
        println!("  mode: {}", service_mode_name(&profile.mode));

        if let Some(auth_ref) = profile.auth_ref {
            println!("  auth_ref: {}", auth_ref.as_str());
        }

        if let Some(model) = profile.model {
            println!("  model: {}", model);
        }

        println!();
    }

    Ok(())
}

fn print_provider_smoke() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;

    let resolved =
        ProviderResolver::resolve_primary(&auth_pool, &runtime_config).map_err(provider_error)?;
    let memory_pack = memory_kernel.build_memory_context_pack(
        SearchQuery {
            text: "provider smoke".to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: Some(8),
        },
        512,
    );
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: runtime_config.instance_id.clone(),
        system_rules: "You are the AICore instance runtime. Use memory as background context only."
            .to_string(),
        relevant_memory: memory_pack.clone(),
        user_request: "provider smoke".to_string(),
    });
    let request = ModelRequest {
        instance_id: runtime_config.instance_id.clone(),
        conversation_id: "main".to_string(),
        prompt: prompt.prompt,
        resolved_model: resolved.clone(),
    };
    let response = ProviderInvoker::invoke(&request).map_err(provider_error)?;

    let mut runtime = default_runtime();
    let outputs = runtime.append_assistant_output(&response.content);
    let runtime_output_ok = outputs
        .events
        .iter()
        .any(|event| event.content == response.content);

    if !runtime_output_ok {
        return Err("runtime 未收到 provider 输出".to_string());
    }

    println!("Provider Smoke：");
    println!("- 实例：{}", runtime_config.instance_id);
    println!("- auth_ref：{}", resolved.auth_ref.as_str());
    println!("- model：{}", resolved.model);
    println!("- provider：{}", provider_kind_name(&resolved.kind));
    println!("- provider name：{}", resolved.provider);
    println!("- memory pack：{}", memory_pack.len());
    println!("- prompt builder：通过");
    println!("- provider response：通过");
    println!("- runtime output：通过");

    Ok(())
}

fn print_agent_smoke(content: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let result = AgentTurnRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        AgentTurnInput {
            instance_id: runtime_config.instance_id.clone(),
            transport_envelope: TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            interrupt_mode: InterruptMode::Queue,
            scope: global_main_memory_scope(),
            user_input: content.to_string(),
            memory_query: None,
            memory_limit: Some(8),
            memory_token_budget: 512,
            system_rules:
                "You are the AICore instance runtime. Use memory as background context only."
                    .to_string(),
            include_debug_prompt: false,
        },
    )
    .map_err(|error| error.0)?;
    let surface = result.to_conversation_surface();
    let turn = &surface.latest_turn;

    if matches!(result.outcome, AgentTurnOutcome::Failed) {
        let stage = turn
            .failure_stage
            .as_ref()
            .map(agent_turn_failure_stage_name)
            .unwrap_or("unknown");
        let message = turn.error_message.as_deref().unwrap_or("未知错误");
        return Err(format!("Agent Turn 失败：阶段={stage}，错误={message}"));
    }

    println!("Agent Loop：通过");
    println!("- 实例：{}", runtime_config.instance_id);
    println!("- outcome：{}", agent_turn_outcome_name(&turn.outcome));
    println!("- memory pack：{} 条", turn.memory_count);
    println!("- prompt builder：通过");
    println!("- ingress source：{}", turn.accepted_source);
    println!(
        "- provider invoked：{}",
        bool_status_name(turn.provider_invoked)
    );
    println!(
        "- provider：{}",
        turn.provider_kind.as_deref().unwrap_or("<none>")
    );
    println!(
        "- provider name：{}",
        turn.provider_name.as_deref().unwrap_or("<none>")
    );
    println!(
        "- assistant output present：{}",
        bool_status_name(turn.assistant_output_present)
    );
    println!(
        "- failure stage：{}",
        turn.failure_stage
            .as_ref()
            .map(agent_turn_failure_stage_name)
            .unwrap_or("<none>")
    );
    println!("- runtime output：已追加");
    println!("- conversation：{}", surface.conversation_id);
    println!("- event count：{}", turn.event_count);
    println!("- queue len：{}", turn.queue_len);

    Ok(())
}

fn print_agent_session_smoke(first: &str, second: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        vec![
            AgentTurnInput {
                instance_id: runtime_config.instance_id.clone(),
                transport_envelope: TransportEnvelope {
                    source: GatewaySource::Cli,
                    platform: None,
                    target_id: None,
                    sender_id: None,
                    is_group: false,
                    mentioned_bot: false,
                },
                interrupt_mode: InterruptMode::Queue,
                scope: global_main_memory_scope(),
                user_input: first.to_string(),
                memory_query: None,
                memory_limit: Some(8),
                memory_token_budget: 512,
                system_rules:
                    "You are the AICore instance runtime. Use memory as background context only."
                        .to_string(),
                include_debug_prompt: false,
            },
            AgentTurnInput {
                instance_id: runtime_config.instance_id.clone(),
                transport_envelope: TransportEnvelope {
                    source: GatewaySource::Cli,
                    platform: None,
                    target_id: None,
                    sender_id: None,
                    is_group: false,
                    mentioned_bot: false,
                },
                interrupt_mode: InterruptMode::Queue,
                scope: global_main_memory_scope(),
                user_input: second.to_string(),
                memory_query: None,
                memory_limit: Some(8),
                memory_token_budget: 512,
                system_rules:
                    "You are the AICore instance runtime. Use memory as background context only."
                        .to_string(),
                include_debug_prompt: false,
            },
        ],
    )
    .map_err(|error| error.0)?;

    let surface = session.surface();

    println!("Agent Session：通过");
    println!("- conversation：{}", surface.conversation_id);
    println!("- turns：{}", surface.turn_count);
    println!(
        "- completed all inputs：{}",
        bool_status_name(surface.completed_all_inputs)
    );
    println!(
        "- stop reason：{}",
        surface
            .stop_reason
            .as_ref()
            .map(agent_session_stop_reason_name)
            .unwrap_or("<none>")
    );
    println!(
        "- latest outcome：{}",
        surface
            .latest_turn
            .as_ref()
            .map(|turn| agent_turn_outcome_name(&turn.outcome))
            .unwrap_or("<none>")
    );
    println!("- conversation status：{}", surface.conversation_status);
    println!("- event count：{}", surface.event_count);
    println!("- queue len：{}", surface.queue_len);
    for (index, turn) in surface.turns.iter().enumerate() {
        println!(
            "- turn {} outcome：{}",
            index + 1,
            agent_turn_outcome_name(&turn.outcome)
        );
        println!(
            "  provider invoked: {}",
            bool_status_name(turn.provider_invoked)
        );
        println!(
            "  assistant output present: {}",
            bool_status_name(turn.assistant_output_present)
        );
        println!(
            "  failure stage: {}",
            turn.failure_stage
                .as_ref()
                .map(agent_turn_failure_stage_name)
                .unwrap_or("<none>")
        );
    }

    Ok(())
}

fn agent_turn_outcome_name(outcome: &AgentTurnOutcome) -> &'static str {
    match outcome {
        AgentTurnOutcome::Completed => "completed",
        AgentTurnOutcome::Queued => "queued",
        AgentTurnOutcome::AppendedContext => "appended_context",
        AgentTurnOutcome::Interrupted => "interrupted",
        AgentTurnOutcome::Failed => "failed",
    }
}

fn agent_turn_failure_stage_name(stage: &aicore_agent::AgentTurnFailureStage) -> &'static str {
    match stage {
        aicore_agent::AgentTurnFailureStage::ProviderResolve => "provider_resolve",
        aicore_agent::AgentTurnFailureStage::ProviderInvoke => "provider_invoke",
        aicore_agent::AgentTurnFailureStage::RuntimeAppend => "runtime_append",
    }
}

fn agent_session_stop_reason_name(reason: &AgentSessionStopReason) -> &'static str {
    match reason {
        AgentSessionStopReason::Failed => "failed",
        AgentSessionStopReason::Queued => "queued",
        AgentSessionStopReason::AppendedContext => "appended_context",
        AgentSessionStopReason::Interrupted => "interrupted",
    }
}

fn bool_status_name(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn print_memory_status() -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel = MemoryKernel::open(paths.clone()).map_err(memory_error)?;

    println!("Memory Status：");
    println!("- instance: global-main");
    println!("- root: {}", paths.root.display());
    println!("- records: {}", kernel.records().len());
    println!("- proposals: {}", kernel.proposals().len());
    println!("- events: {}", kernel.events().len());
    println!("- projection stale: {}", kernel.projection_state().stale);
    println!(
        "- projection warning: {}",
        kernel
            .projection_state()
            .warning
            .as_deref()
            .unwrap_or("<none>")
    );
    println!(
        "- last rebuild at: {}",
        kernel
            .projection_state()
            .last_rebuild_at
            .as_deref()
            .unwrap_or("<none>")
    );

    Ok(())
}

fn print_memory_audit() -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let report = kernel.verify_ledger_consistency();

    render_memory_audit(&report);
    Ok(())
}

fn print_memory_proposals() -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let proposals = kernel.list_open_proposals();

    if proposals.is_empty() {
        println!("暂无待审阅记忆提案。");
        return Ok(());
    }

    println!("Memory Proposals：");
    for proposal in proposals {
        let display_text = if !proposal.localized_summary.is_empty() {
            proposal.localized_summary
        } else if !proposal.content.is_empty() {
            proposal.content
        } else {
            proposal.normalized_content
        };
        println!(
            "- {} [{}] {}",
            proposal.proposal_id,
            memory_type_name(&proposal.memory_type),
            display_text
        );
    }

    Ok(())
}

fn print_memory_wiki_index() -> Result<(), String> {
    print_memory_wiki_page("index")
}

fn print_memory_wiki_page(page: &str) -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel = MemoryKernel::open(paths.clone()).map_err(memory_error)?;
    let (page_name, page_path) = resolve_memory_wiki_page(&paths, page)?;

    if !page_path.exists() {
        return Err("缺少 Wiki Projection，请先写入记忆或重建 projection。".to_string());
    }

    let content = fs::read_to_string(&page_path)
        .map_err(|error| format!("无法读取 Wiki Projection {}: {error}", page_path.display()))?;

    println!("记忆 Wiki Projection：");
    for line in wiki_projection_status_lines(kernel.projection_state()) {
        println!("{line}");
    }
    println!("- page: {page_name}");
    println!("- path: {}", page_path.display());
    println!();
    print!("{content}");

    Ok(())
}

fn print_memory_remember(content: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_main_memory_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .map_err(memory_error)?;

    println!("记忆已写入：");
    println!("- id: {memory_id}");
    println!("- type: core");
    println!("- status: active");

    Ok(())
}

fn print_memory_accept(proposal_id: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .accept_proposal(proposal_id, "user", Some("cli accept"))
        .map_err(memory_error)?;

    println!("记忆提案已接受：");
    println!("- proposal: {proposal_id}");
    println!("- memory: {memory_id}");

    Ok(())
}

fn print_memory_reject(proposal_id: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    kernel
        .reject_proposal(proposal_id, "user", Some("cli reject"))
        .map_err(memory_error)?;

    println!("记忆提案已拒绝：");
    println!("- proposal: {proposal_id}");

    Ok(())
}

fn print_memory_search(query: &str, options: MemorySearchOptions) -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let results = kernel
        .search(SearchQuery {
            text: query.to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: options.memory_type,
            source: options.source,
            permanence: options.permanence,
            limit: options.limit,
        })
        .map_err(memory_error)?;

    println!("记忆搜索：");
    if results.is_empty() {
        println!("- 无匹配记忆");
    } else {
        for result in results {
            let record = result.record;
            println!(
                "- {} [{}] {}",
                record.memory_id,
                memory_type_name(&record.memory_type),
                record.content
            );
            println!("  source: {}", memory_source_name(&record.source));
            println!(
                "  permanence: {}",
                memory_permanence_name(&record.permanence)
            );
            println!("  score: {}", result.score);
            println!("  matched: {}", result.matched_fields.join(","));
        }
    }

    Ok(())
}

fn render_memory_audit(report: &MemoryAuditReport) {
    println!("Memory Audit：");
    println!("- checked events: {}", report.checked_events);
    println!("- status: {}", if report.ok { "ok" } else { "failed" });

    if !report.ok {
        for issue in &report.issues {
            println!("- issue: {issue}");
        }
    }
}

fn output_target_name(target: &OutputTarget) -> &'static str {
    match target {
        OutputTarget::Origin => "origin",
        OutputTarget::ActiveViews => "active-views",
        OutputTarget::FollowedExternal => "followed-external",
    }
}

fn delivery_identity_name(identity: &DeliveryIdentity) -> String {
    match identity {
        DeliveryIdentity::ActiveViews => "active-views".to_string(),
        DeliveryIdentity::External {
            platform,
            target_id,
        } => {
            format!("external:{platform}:{target_id}")
        }
    }
}

fn prepare_demo_config_store(command_name: &str) -> Result<ConfigStore, String> {
    let root = demo_config_root(command_name);
    reset_demo_root(&root)?;

    let store = ConfigStore::new(ConfigPaths::new(&root));
    store.ensure_default_files().map_err(config_error)?;

    let auth_pool = demo_auth_pool();
    let runtime = demo_runtime_config();
    let services = demo_service_profiles();

    store.save_auth_pool(&auth_pool).map_err(config_error)?;
    store
        .save_instance_runtime(&runtime)
        .map_err(config_error)?;
    store.save_services(&services).map_err(config_error)?;

    Ok(store)
}

fn real_config_store() -> Result<ConfigStore, String> {
    Ok(ConfigStore::new(real_config_paths()?))
}

fn real_memory_kernel() -> Result<MemoryKernel, String> {
    MemoryKernel::open(real_memory_paths()?).map_err(memory_error)
}

fn load_real_auth_pool(store: &ConfigStore) -> Result<GlobalAuthPool, String> {
    if !store.paths.auth_toml.exists() {
        return Err("缺少认证池配置，请先运行 config init。".to_string());
    }

    store.load_auth_pool().map_err(config_error)
}

fn load_real_services(store: &ConfigStore) -> Result<GlobalServiceProfiles, String> {
    if !store.paths.services_toml.exists() {
        return Err("缺少服务角色配置，请先运行 config init。".to_string());
    }

    store.load_services().map_err(config_error)
}

fn real_config_paths() -> Result<ConfigPaths, String> {
    Ok(ConfigPaths::new(resolve_real_config_root()?))
}

fn real_memory_paths() -> Result<MemoryPaths, String> {
    Ok(MemoryPaths::new(
        resolve_real_config_root()?
            .join("instances")
            .join("global-main")
            .join("memory"),
    ))
}

fn resolve_memory_wiki_page(
    paths: &MemoryPaths,
    page: &str,
) -> Result<(&'static str, PathBuf), String> {
    if page.contains('/') || page.contains('\\') || page.contains("..") {
        return Err("不允许读取任意 Wiki 路径。".to_string());
    }

    let normalized = page.trim_end_matches(".md");

    match normalized {
        "index" => Ok(("index", paths.wiki_index_md.clone())),
        "core" => Ok(("core", paths.wiki_core_md.clone())),
        "decisions" => Ok(("decisions", paths.wiki_decisions_md.clone())),
        "status" => Ok(("status", paths.wiki_status_md.clone())),
        _ => Err(format!("未知 Wiki 页面：{page}")),
    }
}

fn wiki_projection_status_lines(state: &aicore_memory::ProjectionState) -> Vec<String> {
    let mut lines = Vec::new();
    if state.stale {
        lines.push("Projection 状态：stale".to_string());
    }
    if let Some(warning) = state.warning.as_deref() {
        lines.push(format!("Projection warning：{warning}"));
    }
    lines
}

fn resolve_real_config_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("AICORE_CONFIG_ROOT") {
        return Ok(PathBuf::from(root));
    }

    let home = env::var_os("HOME")
        .ok_or_else(|| "无法确定配置根目录，请设置 HOME 或 AICORE_CONFIG_ROOT。".to_string())?;

    Ok(PathBuf::from(home).join(".aicore").join("config"))
}

fn demo_config_root(command_name: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "aicore-cli-p45-{command_name}-{}",
        std::process::id()
    ))
}

fn reset_demo_root(root: &PathBuf) -> Result<(), String> {
    if root.exists() {
        fs::remove_dir_all(root)
            .map_err(|error| format!("无法清理临时配置目录 {}: {error}", root.display()))?;
    }

    Ok(())
}

struct InitStatus {
    auth_created: bool,
    services_created: bool,
    runtime_created: bool,
}

fn initialize_real_config(store: &ConfigStore) -> Result<InitStatus, String> {
    let auth_created = write_auth_pool_if_missing(store, &demo_auth_pool())?;
    let services_created = write_services_if_missing(store, &demo_service_profiles())?;
    let runtime_created = write_runtime_if_missing(store, &demo_runtime_config())?;

    Ok(InitStatus {
        auth_created,
        services_created,
        runtime_created,
    })
}

fn write_auth_pool_if_missing(store: &ConfigStore, pool: &GlobalAuthPool) -> Result<bool, String> {
    if store.paths.auth_toml.exists() {
        return Ok(false);
    }

    store.save_auth_pool(pool).map_err(config_error)?;
    Ok(true)
}

fn write_services_if_missing(
    store: &ConfigStore,
    services: &GlobalServiceProfiles,
) -> Result<bool, String> {
    if store.paths.services_toml.exists() {
        return Ok(false);
    }

    store.save_services(services).map_err(config_error)?;
    Ok(true)
}

fn write_runtime_if_missing(
    store: &ConfigStore,
    runtime: &InstanceRuntimeConfig,
) -> Result<bool, String> {
    let runtime_path = store.paths.runtime_toml_for(&runtime.instance_id);
    if runtime_path.exists() {
        return Ok(false);
    }

    store.save_instance_runtime(runtime).map_err(config_error)?;
    Ok(true)
}

fn demo_auth_pool() -> GlobalAuthPool {
    GlobalAuthPool::new(vec![
        AuthEntry {
            auth_ref: AuthRef::new("auth.dummy.main"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.dummy.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat, AuthCapability::Vision],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openai.backup"),
            provider: "openai".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openai.backup"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.search"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.search"),
            capabilities: vec![AuthCapability::Search],
            enabled: true,
        },
    ])
}

fn demo_runtime_config() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.dummy.main"),
            model: "dummy/default-chat".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        }),
    }
}

fn demo_service_profiles() -> GlobalServiceProfiles {
    GlobalServiceProfiles {
        profiles: vec![
            ServiceProfile {
                role: ServiceRole::MemoryDreamer,
                mode: ServiceProfileMode::InheritInstance,
                auth_ref: None,
                model: None,
            },
            ServiceProfile {
                role: ServiceRole::EvolutionReviewer,
                mode: ServiceProfileMode::Disabled,
                auth_ref: None,
                model: None,
            },
            ServiceProfile {
                role: ServiceRole::Search,
                mode: ServiceProfileMode::Explicit,
                auth_ref: Some(AuthRef::new("auth.openrouter.search")),
                model: Some("perplexity/sonar".to_string()),
            },
        ],
    }
}

fn config_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) => format!("I/O 错误：{message}"),
        aicore_config::ConfigError::Parse(message) => format!("配置解析错误：{message}"),
        aicore_config::ConfigError::Validation(message) => {
            format!("配置校验错误：{message}")
        }
    }
}

fn memory_error(error: aicore_memory::MemoryError) -> String {
    error.0
}

fn provider_error(error: ProviderError) -> String {
    match error {
        ProviderError::Resolve(message) => format!("provider 解析错误：{message}"),
        ProviderError::Invoke(message) => format!("provider 调用错误：{message}"),
    }
}

fn provider_kind_name(kind: &aicore_provider::ProviderKind) -> &'static str {
    match kind {
        aicore_provider::ProviderKind::Dummy => "dummy",
        aicore_provider::ProviderKind::OpenRouter => "openrouter",
        aicore_provider::ProviderKind::OpenAI => "openai",
    }
}

fn map_runtime_load_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) if message.contains("missing runtime config") => {
            "缺少 global-main runtime 配置，请先运行 config init 或配置模型。".to_string()
        }
        other => config_error(other),
    }
}

fn init_status_name(created: bool) -> &'static str {
    if created {
        "已创建"
    } else {
        "已存在，未覆盖"
    }
}

fn auth_kind_name(kind: &AuthKind) -> &'static str {
    match kind {
        AuthKind::ApiKey => "api_key",
        AuthKind::OAuth => "oauth",
        AuthKind::Session => "session",
        AuthKind::Token => "token",
    }
}

fn auth_capability_name(capability: &AuthCapability) -> &'static str {
    match capability {
        AuthCapability::Chat => "chat",
        AuthCapability::Vision => "vision",
        AuthCapability::Search => "search",
        AuthCapability::Embedding => "embedding",
    }
}

fn service_role_name(role: &ServiceRole) -> &'static str {
    match role {
        ServiceRole::MemoryExtractor => "memory_extractor",
        ServiceRole::MemoryCurator => "memory_curator",
        ServiceRole::MemoryDreamer => "memory_dreamer",
        ServiceRole::EvolutionProposer => "evolution_proposer",
        ServiceRole::EvolutionReviewer => "evolution_reviewer",
        ServiceRole::Search => "search",
        ServiceRole::Tts => "tts",
        ServiceRole::ImageGeneration => "image_generation",
        ServiceRole::VideoGeneration => "video_generation",
        ServiceRole::Vision => "vision",
        ServiceRole::Reranker => "reranker",
    }
}

fn service_mode_name(mode: &ServiceProfileMode) -> &'static str {
    match mode {
        ServiceProfileMode::InheritInstance => "inherit_instance",
        ServiceProfileMode::Explicit => "explicit",
        ServiceProfileMode::Disabled => "disabled",
    }
}

fn memory_type_name(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

fn memory_source_name(source: &MemorySource) -> &'static str {
    match source {
        MemorySource::UserExplicit => "user_explicit",
        MemorySource::UserCorrection => "user_correction",
        MemorySource::AssistantSummary => "assistant_summary",
        MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

fn memory_permanence_name(permanence: &MemoryPermanence) -> &'static str {
    match permanence {
        MemoryPermanence::Standard => "standard",
        MemoryPermanence::Permanent => "permanent",
    }
}

fn global_main_memory_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

#[derive(Debug, Default)]
struct MemorySearchOptions {
    memory_type: Option<MemoryType>,
    source: Option<MemorySource>,
    permanence: Option<MemoryPermanence>,
    limit: Option<usize>,
}

fn parse_memory_search_options(args: &[String]) -> Result<MemorySearchOptions, String> {
    let mut options = MemorySearchOptions::default();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--type" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --type 参数值。".to_string())?;
                options.memory_type = Some(parse_memory_type_filter(value)?);
                index += 2;
            }
            "--source" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --source 参数值。".to_string())?;
                options.source = Some(parse_memory_source_filter(value)?);
                index += 2;
            }
            "--permanence" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --permanence 参数值。".to_string())?;
                options.permanence = Some(parse_memory_permanence_filter(value)?);
                index += 2;
            }
            "--limit" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --limit 参数值。".to_string())?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "--limit 必须是正整数。".to_string())?;
                if parsed == 0 {
                    return Err("--limit 必须是正整数。".to_string());
                }
                options.limit = Some(parsed);
                index += 2;
            }
            other => {
                return Err(format!("未知参数：{other}"));
            }
        }
    }

    Ok(options)
}

fn parse_memory_type_filter(value: &str) -> Result<MemoryType, String> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(format!("无效的 --type：{value}")),
    }
}

fn parse_memory_source_filter(value: &str) -> Result<MemorySource, String> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(format!("无效的 --source：{value}")),
    }
}

fn parse_memory_permanence_filter(value: &str) -> Result<MemoryPermanence, String> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(format!("无效的 --permanence：{value}")),
    }
}

#[cfg(test)]
mod tests {
    use aicore_memory::ProjectionState;

    use super::{run_from_args, wiki_projection_status_lines};

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(run_from_args(vec!["unknown".to_string()]), 1);
    }

    #[test]
    fn rejects_unknown_config_command() {
        assert_eq!(
            run_from_args(vec!["config".to_string(), "unknown".to_string()]),
            1
        );
    }

    #[test]
    fn memory_wiki_warns_when_projection_stale() {
        let lines = wiki_projection_status_lines(&ProjectionState {
            stale: true,
            warning: None,
            last_rebuild_at: Some("123".to_string()),
        });

        assert!(lines.iter().any(|line| line == "Projection 状态：stale"));
    }

    #[test]
    fn memory_wiki_warns_when_projection_warning_exists() {
        let lines = wiki_projection_status_lines(&ProjectionState {
            stale: true,
            warning: Some("projection warning".to_string()),
            last_rebuild_at: Some("123".to_string()),
        });

        assert!(
            lines
                .iter()
                .any(|line| line == "Projection warning：projection warning")
        );
    }
}
