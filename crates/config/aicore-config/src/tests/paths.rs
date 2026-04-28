use super::*;

#[test]
fn config_paths_resolve_expected_files() {
    let paths = ConfigPaths::new("/tmp/aicore-config");

    assert_eq!(paths.root, PathBuf::from("/tmp/aicore-config"));
    assert_eq!(
        paths.auth_toml,
        PathBuf::from("/tmp/aicore-config/auth.toml")
    );
    assert_eq!(
        paths.services_toml,
        PathBuf::from("/tmp/aicore-config/services.toml")
    );
    assert_eq!(
        paths.providers_toml,
        PathBuf::from("/tmp/aicore-config/providers.toml")
    );
    assert_eq!(
        paths.instances_dir,
        PathBuf::from("/tmp/aicore-config/instances")
    );
}

#[test]
fn config_paths_resolve_instance_runtime_file() {
    let paths = ConfigPaths::new("/tmp/aicore-config");

    assert_eq!(
        paths.runtime_toml_for("global-main"),
        PathBuf::from("/tmp/aicore-config/instances/global-main/runtime.toml")
    );
}

#[test]
fn ensure_default_files_creates_empty_auth_and_services_files() {
    let root = temp_root("default-files");
    let store = ConfigStore::new(ConfigPaths::new(&root));

    store
        .ensure_default_files()
        .expect("default files should be created");

    assert!(store.paths.root.exists());
    assert!(store.paths.auth_toml.exists());
    assert!(store.paths.services_toml.exists());
    assert!(store.paths.providers_toml.exists());
    assert!(store.paths.instances_dir.exists());

    let auth = store
        .load_auth_pool()
        .expect("default auth pool should load");
    let services = store.load_services().expect("default services should load");
    let providers = store
        .load_provider_profiles()
        .expect("default provider profiles should load");

    assert!(auth.entries().is_empty());
    assert!(services.profiles.is_empty());
    assert!(providers.profiles.is_empty());
}

#[test]
fn config_paths_include_providers_toml() {
    let paths = ConfigPaths::new("/tmp/aicore-config");

    assert_eq!(
        paths.providers_toml,
        PathBuf::from("/tmp/aicore-config/providers.toml")
    );
}
