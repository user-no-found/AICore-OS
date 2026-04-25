use std::{env, fs, path::PathBuf};

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{
    ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ServiceProfile, ServiceProfileMode, ServiceRole,
};
use aicore_control::default_control_plane;
use aicore_runtime::{
    DeliveryIdentity, GatewaySource, InterruptMode, OutputTarget, TransportEnvelope,
    default_runtime,
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
        [group, _] if group == "config" => {
            eprintln!("未知 config 命令。");
            eprintln!("可用命令：config smoke | config path | config init | config validate");
            1
        }
        _ => {
            eprintln!("未知命令。");
            eprintln!(
                "可用命令：status | instance list | runtime smoke | config smoke | config path | config init | config validate | auth list | model show | service list"
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
            aicore_contracts::InstanceKind::GlobalMain => "global_main",
            aicore_contracts::InstanceKind::Workspace => "workspace",
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
    let store = prepare_demo_config_store("auth-list")?;
    let auth_pool = store.load_auth_pool().map_err(config_error)?;

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
    let store = prepare_demo_config_store("model-show")?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(config_error)?;

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
    let store = prepare_demo_config_store("service-list")?;
    let services = store.load_services().map_err(config_error)?;

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

fn real_config_paths() -> Result<ConfigPaths, String> {
    Ok(ConfigPaths::new(resolve_real_config_root()?))
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
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openai.backup"),
            model: "gpt-4.1".to_string(),
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

#[cfg(test)]
mod tests {
    use super::run_from_args;

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
}
