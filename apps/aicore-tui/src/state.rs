use std::path::Path;

use aicore_foundation::{
    AicoreResult, InstanceKind, ensure_instance_layout, resolve_instance_for_cwd,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiLaunchContext {
    pub instance_id: String,
    pub instance_kind: String,
    pub workspace_root: String,
    pub instance_root: String,
}

impl TuiLaunchContext {
    pub fn instance_kind_label(&self) -> &'static str {
        match self.instance_kind.as_str() {
            "global-main" => "主实例",
            "workspace" => "工作区实例",
            _ => "未知实例",
        }
    }
}

pub fn build_launch_context(cwd: &Path, home: &Path) -> AicoreResult<TuiLaunchContext> {
    let binding = resolve_instance_for_cwd(cwd, home)?;
    let paths = ensure_instance_layout(&binding)?;

    Ok(TuiLaunchContext {
        instance_id: binding.instance_id.as_str().to_string(),
        instance_kind: match binding.kind {
            InstanceKind::GlobalMain => "global-main".to_string(),
            InstanceKind::Workspace => "workspace".to_string(),
        },
        workspace_root: binding
            .workspace_root
            .as_deref()
            .unwrap_or(cwd)
            .display()
            .to_string(),
        instance_root: paths.root.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::build_launch_context;

    #[test]
    fn workspace_launch_context_uses_project_aicore_root() {
        let home = TestDir::new("home");
        let workspace = TestDir::new("workspace");
        let context = build_launch_context(workspace.path(), home.path()).unwrap();

        assert_eq!(context.instance_kind, "workspace");
        assert!(context.instance_id.starts_with("workspace."));
        assert!(context.instance_root.ends_with(".aicore"));
        assert!(workspace.path().join(".gitignore").is_file());
    }

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(name: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time should be available")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "aicore-tui-state-{name}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).expect("test dir should create");
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
}
