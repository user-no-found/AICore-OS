use aicore_auth::GlobalAuthPool;
use aicore_config::{ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig};

use crate::config_store::{
    demo_auth_pool, demo_runtime_config, demo_service_profiles, initialize_real_config,
    prepare_demo_config_store, real_config_paths, real_config_store,
};
use crate::errors::{config_error, map_runtime_load_error};
use crate::names::init_status_name;
use crate::terminal::{cli_row, emit_cli_panel, emit_cli_panel_body};

pub(crate) fn print_config_smoke() -> Result<(), String> {
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

    ensure_demo_matches(loaded_auth_pool, loaded_runtime, loaded_services)?;

    emit_cli_panel_body(
        "配置 Smoke Test：",
        "- 默认配置文件：通过\n- 认证池保存/读取：通过\n- 实例运行配置保存/读取：通过\n- 服务角色配置保存/读取：通过\n- 配置校验：通过",
    );

    Ok(())
}

pub(crate) fn print_config_path() -> Result<(), String> {
    let paths = real_config_paths()?;

    emit_cli_panel(
        "配置路径",
        vec![
            cli_row("root", paths.root.display().to_string()),
            cli_row("auth.toml", paths.auth_toml.display().to_string()),
            cli_row("services.toml", paths.services_toml.display().to_string()),
            cli_row("instances", paths.instances_dir.display().to_string()),
            cli_row(
                "global-main runtime",
                paths.runtime_toml_for("global-main").display().to_string(),
            ),
        ],
    );

    Ok(())
}

pub(crate) fn print_config_init() -> Result<(), String> {
    let store = real_config_store()?;
    let status = initialize_real_config(&store)?;

    emit_cli_panel(
        "配置初始化",
        vec![
            cli_row("root", store.paths.root.display().to_string()),
            cli_row("auth.toml", init_status_name(status.auth_created)),
            cli_row("services.toml", init_status_name(status.services_created)),
            cli_row(
                "global-main runtime.toml",
                init_status_name(status.runtime_created),
            ),
        ],
    );

    Ok(())
}

pub(crate) fn print_config_validate() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = store.load_auth_pool().map_err(config_error)?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let services = store.load_services().map_err(config_error)?;

    ConfigStore::validate_runtime_config(&runtime, &auth_pool).map_err(config_error)?;
    ConfigStore::validate_service_profiles(&services, &auth_pool).map_err(config_error)?;

    emit_cli_panel(
        "配置校验",
        vec![
            cli_row("认证池", "已读取"),
            cli_row("实例运行配置", "通过"),
            cli_row("服务角色配置", "通过"),
        ],
    );

    Ok(())
}

fn ensure_demo_matches(
    loaded_auth_pool: GlobalAuthPool,
    loaded_runtime: InstanceRuntimeConfig,
    loaded_services: GlobalServiceProfiles,
) -> Result<(), String> {
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
    Ok(())
}
