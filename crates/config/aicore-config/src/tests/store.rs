use super::*;

#[test]
fn save_and_load_auth_pool() {
    let root = temp_root("auth-pool");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let pool = auth_pool();

    store.save_auth_pool(&pool).expect("auth pool should save");
    let loaded = store.load_auth_pool().expect("auth pool should load");

    assert_eq!(loaded.entries().len(), 2);
    assert_eq!(
        loaded.entries()[0].auth_ref,
        AuthRef::new("auth.openrouter.main")
    );
}

#[test]
fn save_and_load_instance_runtime_config() {
    let root = temp_root("runtime-config");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let runtime = InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openai.backup"),
            model: "gpt-4.1".to_string(),
        }),
    };

    store
        .save_instance_runtime(&runtime)
        .expect("runtime config should save");
    let loaded = store
        .load_instance_runtime("global-main")
        .expect("runtime config should load");

    assert_eq!(
        loaded.primary.auth_ref,
        AuthRef::new("auth.openrouter.main")
    );
    assert_eq!(
        loaded.fallback.unwrap().auth_ref,
        AuthRef::new("auth.openai.backup")
    );
}

#[test]
fn save_and_load_global_service_profiles() {
    let root = temp_root("services");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let services = GlobalServiceProfiles {
        profiles: vec![ServiceProfile {
            role: ServiceRole::MemoryDreamer,
            mode: ServiceProfileMode::Explicit,
            auth_ref: Some(AuthRef::new("auth.openrouter.main")),
            model: Some("openai/gpt-5".to_string()),
        }],
    };

    store
        .save_services(&services)
        .expect("services should save");
    let loaded = store.load_services().expect("services should load");

    assert_eq!(loaded.profiles.len(), 1);
    assert_eq!(loaded.profiles[0].role, ServiceRole::MemoryDreamer);
    assert_eq!(
        loaded.profiles[0].auth_ref,
        Some(AuthRef::new("auth.openrouter.main"))
    );
}
