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
        .env("AICORE_TUI_SKIP_WARP_LAUNCH", "1")
        .output()
        .expect("aicore-tui should run");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore TUI Warp fork"));
    assert!(stdout.contains("实例"));
    assert!(stdout.contains("工作区"));
    assert!(stdout.contains("已完成实例绑定"));
    assert!(workspace.path().join(".aicore").is_dir());
    assert!(workspace.path().join(".gitignore").is_file());
}

#[test]
fn reports_missing_graphical_session_without_panic() {
    let home = TestDir::new("home");
    let workspace = TestDir::new("workspace");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-tui"))
        .current_dir(workspace.path())
        .env("HOME", home.path())
        .env_remove("AICORE_TUI_SKIP_WARP_LAUNCH")
        .env_remove("WAYLAND_DISPLAY")
        .env_remove("WAYLAND_SOCKET")
        .env_remove("DISPLAY")
        .output()
        .expect("aicore-tui should run");

    assert_eq!(output.status.code(), Some(3));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
    assert!(stderr.contains("没有图形会话"));
    assert!(stderr.contains("WAYLAND_DISPLAY"));
    assert!(!stderr.contains("panicked"));
    assert!(!stderr.contains("RUST_BACKTRACE"));
    assert!(workspace.path().join(".aicore").is_dir());
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
