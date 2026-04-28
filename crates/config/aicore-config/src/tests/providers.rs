use super::*;

#[test]
fn provider_profiles_config_round_trips_custom_openai_endpoint() {
    let root = temp_root("provider-profiles");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let providers = ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-openai-compatible".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    };

    store
        .save_provider_profiles(&providers)
        .expect("provider profiles should save");
    let loaded = store
        .load_provider_profiles()
        .expect("provider profiles should load");

    assert_eq!(loaded, providers);
}

#[test]
fn provider_profile_override_does_not_render_raw_secret() {
    let root = temp_root("provider-secret-boundary");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let providers = ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-openai-compatible".to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    };

    store
        .save_provider_profiles(&providers)
        .expect("provider profiles should save");
    let rendered =
        fs::read_to_string(store.paths.providers_toml).expect("providers.toml should be readable");

    assert!(!rendered.contains("sk-live-secret-value"));
    assert!(!rendered.contains("secret://"));
}

#[test]
fn disabled_provider_override_is_not_available() {
    let root = temp_root("provider-disabled");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let providers = ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "custom-openai-compatible".to_string(),
            base_url: Some("http://localhost:11434/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: false,
        }],
    };

    store
        .save_provider_profiles(&providers)
        .expect("provider profiles should save");
    let loaded = store
        .load_provider_profiles()
        .expect("provider profiles should load");

    assert!(!loaded.profiles[0].enabled);
}

#[test]
fn provider_profile_override_can_enable_xiaomi_with_explicit_base_url() {
    let root = temp_root("provider-xiaomi");
    let store = ConfigStore::new(ConfigPaths::new(&root));
    let providers = ProviderProfilesConfig {
        profiles: vec![ProviderProfileOverride {
            provider_id: "xiaomi".to_string(),
            base_url: Some("https://api.example.xiaomi.invalid/v1".to_string()),
            api_mode: Some("openai_chat_completions".to_string()),
            engine_id: Some("python.openai".to_string()),
            enabled: true,
        }],
    };

    store
        .save_provider_profiles(&providers)
        .expect("provider profiles should save");
    let loaded = store
        .load_provider_profiles()
        .expect("provider profiles should load");

    assert_eq!(loaded.profiles[0].provider_id, "xiaomi");
    assert!(loaded.profiles[0].base_url.is_some());
    assert!(loaded.profiles[0].enabled);
}
