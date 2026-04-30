use std::{env, fs, path::PathBuf};

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{
    ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ServiceProfile, ServiceProfileMode, ServiceRole,
};
use aicore_foundation::{
    InstanceBinding, InstanceId, InstanceKind, ensure_instance_layout, instance_paths,
    resolve_instance_for_cwd,
};
use aicore_memory::{MemoryKernel, MemoryPaths, MemoryScope};

use crate::errors::{config_error, memory_error};

pub(crate) fn prepare_demo_config_store(command_name: &str) -> Result<ConfigStore, String> {
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

pub(crate) fn real_config_store() -> Result<ConfigStore, String> {
    Ok(ConfigStore::new(real_config_paths()?))
}

pub(crate) fn real_memory_kernel() -> Result<MemoryKernel, String> {
    MemoryKernel::open(real_memory_paths()?).map_err(memory_error)
}

pub(crate) fn load_real_auth_pool(store: &ConfigStore) -> Result<GlobalAuthPool, String> {
    if !store.paths.auth_toml.exists() {
        return Err("缺少认证池配置，请先运行 config init。".to_string());
    }

    store.load_auth_pool().map_err(config_error)
}

pub(crate) fn load_real_services(store: &ConfigStore) -> Result<GlobalServiceProfiles, String> {
    if !store.paths.services_toml.exists() {
        return Err("缺少服务角色配置，请先运行 config init。".to_string());
    }

    store.load_services().map_err(config_error)
}

pub(crate) fn real_config_paths() -> Result<ConfigPaths, String> {
    Ok(ConfigPaths::new(resolve_real_config_root()?))
}

pub(crate) fn real_memory_paths() -> Result<MemoryPaths, String> {
    if let Some(root) = env::var_os("AICORE_CONFIG_ROOT") {
        return Ok(MemoryPaths::new(
            PathBuf::from(root)
                .join("instances")
                .join("global-main")
                .join("memory"),
        ));
    }

    Ok(MemoryPaths::new(
        resolve_runtime_instance_context()?.paths.memory_dir,
    ))
}

pub(crate) fn real_memory_scope() -> Result<MemoryScope, String> {
    if env::var_os("AICORE_CONFIG_ROOT").is_some() {
        return Ok(MemoryScope::GlobalMain {
            instance_id: "global-main".to_string(),
        });
    }

    Ok(memory_scope_for_binding(
        &resolve_runtime_instance_context()?.binding,
    ))
}

pub(crate) fn real_event_store_binding() -> Result<(PathBuf, InstanceId), String> {
    if let Some(root) = env::var_os("AICORE_CONFIG_ROOT") {
        return Ok((
            PathBuf::from(root)
                .join("instances")
                .join("global-main")
                .join("events")
                .join("events.sqlite"),
            InstanceId::global_main(),
        ));
    }

    let context = resolve_runtime_instance_context()?;
    Ok((
        context.paths.events_dir.join("events.sqlite"),
        context.binding.instance_id,
    ))
}

pub(crate) struct InitStatus {
    pub(crate) auth_created: bool,
    pub(crate) services_created: bool,
    pub(crate) runtime_created: bool,
}

pub(crate) fn initialize_real_config(store: &ConfigStore) -> Result<InitStatus, String> {
    let auth_created = write_auth_pool_if_missing(store, &demo_auth_pool())?;
    let services_created = write_services_if_missing(store, &demo_service_profiles())?;
    let runtime_created = write_runtime_if_missing(store, &demo_runtime_config())?;

    Ok(InitStatus {
        auth_created,
        services_created,
        runtime_created,
    })
}

pub(crate) fn demo_auth_pool() -> GlobalAuthPool {
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

pub(crate) fn demo_runtime_config() -> InstanceRuntimeConfig {
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

pub(crate) fn demo_service_profiles() -> GlobalServiceProfiles {
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

fn resolve_real_config_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("AICORE_CONFIG_ROOT") {
        return Ok(PathBuf::from(root));
    }

    Ok(resolve_runtime_instance_context()?.paths.config_dir)
}

fn resolve_home_dir() -> Result<std::ffi::OsString, String> {
    env::var_os("HOME")
        .ok_or_else(|| "无法确定配置根目录，请设置 HOME 或 AICORE_CONFIG_ROOT。".to_string())
}

struct RuntimeInstanceContext {
    pub(crate) binding: InstanceBinding,
    pub(crate) paths: aicore_foundation::InstancePaths,
}

fn resolve_runtime_instance_context() -> Result<RuntimeInstanceContext, String> {
    let home = PathBuf::from(resolve_home_dir()?);
    let cwd = env::current_dir().map_err(|error| format!("无法获取当前目录：{error}"))?;
    let binding = resolve_instance_for_cwd(&cwd, &home).map_err(|error| error.to_string())?;
    let paths = instance_paths(&binding);
    ensure_instance_layout(&binding).map_err(|error| error.to_string())?;

    Ok(RuntimeInstanceContext { binding, paths })
}

fn memory_scope_for_binding(binding: &InstanceBinding) -> MemoryScope {
    match binding.kind {
        InstanceKind::GlobalMain => MemoryScope::GlobalMain {
            instance_id: binding.instance_id.as_str().to_string(),
        },
        InstanceKind::Workspace => MemoryScope::Workspace {
            instance_id: binding.instance_id.as_str().to_string(),
            workspace_root: binding
                .workspace_root
                .as_ref()
                .expect("workspace binding should include workspace root")
                .display()
                .to_string(),
        },
    }
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
