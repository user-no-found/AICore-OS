use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{bind_current_instance, bind_instance_for_paths};

#[test]
fn binds_workspace_instance_before_warp_launch() {
    let home = TestDir::new("home");
    let workspace = TestDir::new("workspace");

    let binding = bind_instance_for_paths(workspace.path(), home.path()).unwrap();

    assert_eq!(binding.instance_kind, "workspace");
    assert!(binding.instance_id.starts_with("workspace."));
    assert_eq!(binding.workspace_root, workspace.path());
    assert_eq!(binding.instance_root, workspace.path().join(".aicore"));
    assert!(workspace.path().join(".gitignore").is_file());
}

#[test]
fn rejects_workspace_metadata_that_impersonates_global_main() {
    let home = TestDir::new("home");
    let workspace = TestDir::new("workspace");
    let aicore_root = workspace.path().join(".aicore");
    std::fs::create_dir_all(&aicore_root).unwrap();
    std::fs::write(
        aicore_root.join("instance.toml"),
        "instance_id = \"global-main\"\ninstance_kind = \"workspace\"\n",
    )
    .unwrap();

    let error = bind_instance_for_paths(workspace.path(), home.path()).unwrap_err();

    assert!(error.to_string().contains("cannot use global-main"));
}

#[test]
fn uses_launcher_binding_when_available() {
    let workspace = TestDir::new("workspace");
    let instance_root = workspace.path().join(".aicore");

    let _guard = EnvGuard::set_many(&[
        ("AICORE_INSTANCE_ID", "workspace.demo.123"),
        ("AICORE_INSTANCE_KIND", "workspace"),
        (
            "AICORE_WORKSPACE_ROOT",
            workspace.path().to_str().expect("workspace path"),
        ),
        (
            "AICORE_INSTANCE_ROOT",
            instance_root.to_str().expect("instance path"),
        ),
    ]);

    let binding = bind_current_instance().unwrap();

    assert_eq!(binding.instance_id, "workspace.demo.123");
    assert_eq!(binding.instance_kind, "workspace");
    assert_eq!(binding.workspace_root, workspace.path());
    assert_eq!(binding.instance_root, instance_root);
}

#[test]
fn rejects_invalid_launcher_workspace_binding() {
    let workspace = TestDir::new("workspace");
    let instance_root = workspace.path().join(".aicore");

    let _guard = EnvGuard::set_many(&[
        ("AICORE_INSTANCE_ID", "global-main"),
        ("AICORE_INSTANCE_KIND", "workspace"),
        (
            "AICORE_WORKSPACE_ROOT",
            workspace.path().to_str().expect("workspace path"),
        ),
        (
            "AICORE_INSTANCE_ROOT",
            instance_root.to_str().expect("instance path"),
        ),
    ]);

    let error = bind_current_instance().unwrap_err();

    assert!(error.to_string().contains("cannot use global-main"));
}

struct TestDir {
    path: PathBuf,
}

impl TestDir {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aicore-warp-bridge-{name}-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

struct EnvGuard {
    previous: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvGuard {
    fn set_many(values: &[(&'static str, &str)]) -> Self {
        let keys = [
            "AICORE_INSTANCE_ID",
            "AICORE_INSTANCE_KIND",
            "AICORE_WORKSPACE_ROOT",
            "AICORE_INSTANCE_ROOT",
        ];
        let previous = keys
            .iter()
            .map(|key| (*key, std::env::var_os(key)))
            .collect::<Vec<_>>();
        for key in keys {
            unsafe {
                std::env::remove_var(key);
            }
        }
        for (key, value) in values {
            unsafe {
                std::env::set_var(key, value);
            }
        }
        Self { previous }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.previous.drain(..) {
            unsafe {
                match value {
                    Some(value) => std::env::set_var(key, value),
                    None => std::env::remove_var(key),
                }
            }
        }
    }
}
