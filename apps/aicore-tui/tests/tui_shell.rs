use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn renders_terminal_tui_snapshot() {
    let home = TestDir::new("home");
    let workspace = TestDir::new("workspace");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-tui"))
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .output()
        .expect("aicore-tui should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore OS"));
    assert!(stdout.contains("实例"));
    assert!(stdout.contains("会话流"));
    assert!(stdout.contains("运行时"));
    assert!(stdout.contains("aicore >"));
    assert!(stdout.contains("不启动智能体运行时"));
    assert!(workspace.path().join(".aicore").is_dir());
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
            "aicore-tui-it-{name}-{}-{unique}",
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
