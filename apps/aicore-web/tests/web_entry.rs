use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn prints_help_with_vue_and_rust_boundary() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-web"))
        .arg("--help")
        .output()
        .expect("aicore-web help should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("AICore Web"));
    assert!(stdout.contains("Vue3"));
    assert!(stdout.contains("Rust"));
    assert!(stdout.contains("不启动智能体运行时"));
}

#[test]
fn prints_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-web"))
        .arg("--version")
        .output()
        .expect("aicore-web version should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.starts_with("aicore-web "));
}

#[test]
fn writes_fpk_skeleton() {
    let root = TestDir::new("fpk-root");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-web"))
        .arg("--fpk-root")
        .arg(root.path())
        .output()
        .expect("aicore-web fpk skeleton should run");

    assert!(output.status.success());
    assert!(root.path().join("manifest.json").is_file());
    assert!(root.path().join("scripts/start.sh").is_file());
    assert!(root.path().join("scripts/package.sh").is_file());
    assert!(root.path().join("config/fnos.sample.toml").is_file());
    assert!(root.path().join("web/index.html").is_file());
    assert!(root.path().join("web/assets/app.js").is_file());
    assert!(root.path().join("web/assets/app.css").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.path().join("scripts/start.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0);
    }

    let manifest = std::fs::read_to_string(root.path().join("manifest.json")).unwrap();
    assert!(manifest.contains("\"ui\": \"vue3\""));
    assert!(manifest.contains("\"backend\": \"rust\""));
    assert!(manifest.contains("\"bind\": \"0.0.0.0\""));
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
            "aicore-web-it-{name}-{}-{unique}",
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
