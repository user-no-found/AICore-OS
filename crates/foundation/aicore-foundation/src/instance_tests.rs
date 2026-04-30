use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::{
    DEFAULT_INSTANCE_SOUL, InstanceKind, ensure_instance_layout, ensure_workspace_gitignore,
    instance_paths, resolve_instance_for_cwd,
};

#[test]
fn cwd_equal_home_binds_global_main() {
    let sandbox = temp_root("global-main-home");
    let home = sandbox.path().join("home");
    fs::create_dir_all(&home).expect("home should create");

    let binding = resolve_instance_for_cwd(&home, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::GlobalMain);
    assert_eq!(binding.instance_id.as_str(), "global-main");
    assert_eq!(binding.root, home.join(".aicore"));
    assert_eq!(binding.workspace_root, None);
}

#[test]
fn binds_workspace_root_when_cwd_contains_aicore() {
    let sandbox = temp_root("workspace-root");
    let home = sandbox.path().join("home");
    let workspace = home.join("project");
    fs::create_dir_all(workspace.join(".aicore")).expect("workspace root should create");

    let binding = resolve_instance_for_cwd(&workspace, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::Workspace);
    assert_eq!(binding.root, workspace.join(".aicore"));
    assert_eq!(binding.workspace_root, Some(workspace));
}

#[test]
fn binds_parent_workspace_when_child_cwd_has_ancestor_aicore() {
    let sandbox = temp_root("workspace-parent");
    let home = sandbox.path().join("home");
    let workspace = home.join("project");
    let cwd = workspace.join("src/bin");
    fs::create_dir_all(workspace.join(".aicore")).expect("workspace root should create");
    fs::create_dir_all(&cwd).expect("child cwd should create");

    let binding = resolve_instance_for_cwd(&cwd, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::Workspace);
    assert_eq!(binding.root, workspace.join(".aicore"));
    assert_eq!(binding.workspace_root, Some(workspace));
}

#[test]
fn creates_workspace_binding_at_cwd_when_under_home_without_existing_marker() {
    let sandbox = temp_root("workspace-create-under-home");
    let home = sandbox.path().join("home");
    let cwd = home.join("project/nested");
    fs::create_dir_all(&cwd).expect("cwd should create");
    fs::create_dir_all(home.join(".aicore")).expect("global main root should exist");

    let binding = resolve_instance_for_cwd(&cwd, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::Workspace);
    assert_eq!(binding.root, cwd.join(".aicore"));
    assert_eq!(binding.workspace_root, Some(cwd));
}

#[test]
fn search_stops_before_home_and_does_not_treat_home_aicore_as_workspace() {
    let sandbox = temp_root("home-boundary");
    let home = sandbox.path().join("home");
    let cwd = home.join("project/subdir");
    fs::create_dir_all(home.join(".aicore")).expect("global main root should create");
    fs::create_dir_all(&cwd).expect("cwd should create");

    let binding = resolve_instance_for_cwd(&cwd, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::Workspace);
    assert_eq!(binding.root, cwd.join(".aicore"));
    assert_ne!(binding.root, home.join(".aicore"));
}

#[test]
fn outside_home_without_marker_creates_workspace_at_cwd() {
    let sandbox = temp_root("outside-home-cwd");
    let home = sandbox.path().join("home");
    let cwd = sandbox.path().join("external/project");
    fs::create_dir_all(&home).expect("home should create");
    fs::create_dir_all(&cwd).expect("cwd should create");

    let binding = resolve_instance_for_cwd(&cwd, &home).expect("resolve should succeed");

    assert_eq!(binding.kind, InstanceKind::Workspace);
    assert_eq!(binding.root, cwd.join(".aicore"));
    assert_eq!(binding.workspace_root, Some(cwd));
}

#[test]
fn ensure_workspace_gitignore_appends_aicore_entry() {
    let sandbox = temp_root("gitignore-append");
    let workspace = sandbox.path().join("workspace");
    fs::create_dir_all(&workspace).expect("workspace should create");
    fs::write(workspace.join(".gitignore"), "target/\n").expect("gitignore should write");

    ensure_workspace_gitignore(&workspace).expect("gitignore update should succeed");

    let gitignore =
        fs::read_to_string(workspace.join(".gitignore")).expect("gitignore should read");
    assert!(gitignore.contains("target/\n"));
    assert!(gitignore.contains(".aicore/\n"));
}

#[test]
fn ensure_workspace_gitignore_does_not_duplicate_entry() {
    let sandbox = temp_root("gitignore-dedupe");
    let workspace = sandbox.path().join("workspace");
    fs::create_dir_all(&workspace).expect("workspace should create");
    fs::write(workspace.join(".gitignore"), ".aicore/\n").expect("gitignore should write");

    ensure_workspace_gitignore(&workspace).expect("gitignore update should succeed");

    let gitignore =
        fs::read_to_string(workspace.join(".gitignore")).expect("gitignore should read");
    assert_eq!(gitignore.matches(".aicore/").count(), 1);
}

#[test]
fn ensure_instance_layout_creates_workspace_soul_without_user_profile() {
    let sandbox = temp_root("workspace-layout");
    let home = sandbox.path().join("home");
    let cwd = home.join("project");
    fs::create_dir_all(&cwd).expect("cwd should create");

    let binding = resolve_instance_for_cwd(&cwd, &home).expect("resolve should succeed");
    let paths = ensure_instance_layout(&binding).expect("layout should succeed");

    assert_eq!(
        fs::read_to_string(&paths.soul_md).expect("soul should read"),
        DEFAULT_INSTANCE_SOUL
    );
    assert!(paths.user_profile_md.is_none());
    assert!(!binding.root.join("user_profile.md").exists());
    assert!(cwd.join(".gitignore").exists());
    assert!(paths.bindings_dir.exists());
    assert!(paths.config_dir.exists());
    assert!(paths.registry_dir.exists());
    assert!(paths.cache_dir.exists());
    assert!(paths.logs_dir.exists());
    assert!(paths.tmp_dir.exists());
}

#[test]
fn ensure_instance_layout_creates_global_main_soul_and_user_profile() {
    let sandbox = temp_root("global-layout");
    let home = sandbox.path().join("home");
    fs::create_dir_all(&home).expect("home should create");

    let binding = resolve_instance_for_cwd(&home, &home).expect("resolve should succeed");
    let paths = ensure_instance_layout(&binding).expect("layout should succeed");

    assert_eq!(
        fs::read_to_string(&paths.soul_md).expect("soul should read"),
        DEFAULT_INSTANCE_SOUL
    );
    let user_profile = paths
        .user_profile_md
        .expect("global main should have user profile");
    assert!(user_profile.exists());
    assert!(!home.join(".gitignore").exists());
}

#[test]
fn instance_paths_keep_existing_events_directory_compatibility() {
    let sandbox = temp_root("events-compat");
    let home = sandbox.path().join("home");
    fs::create_dir_all(&home).expect("home should create");

    let binding = resolve_instance_for_cwd(&home, &home).expect("resolve should succeed");
    let paths = instance_paths(&binding);

    assert_eq!(paths.events_dir, home.join(".aicore").join("events"));
}

fn temp_root(name: &str) -> TempDir {
    tempfile::Builder::new()
        .prefix(&format!("aicore-foundation-{name}-"))
        .tempdir()
        .expect("temp dir should create")
}

#[allow(dead_code)]
fn assert_not_real_host_path(path: &Path) {
    let text = path.display().to_string();
    assert!(!text.starts_with("/vol1/"));
    assert!(!text.starts_with("/home/sun"));
}
