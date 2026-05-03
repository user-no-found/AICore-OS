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
fn writes_fnos_native_package_source() {
    let root = TestDir::new("fpk-root");
    let output = Command::new(env!("CARGO_BIN_EXE_aicore-web"))
        .arg("--fpk-root")
        .arg(root.path())
        .output()
        .expect("aicore-web fpk source writer should run");

    assert!(output.status.success());
    assert!(root.path().join("manifest").is_file());
    assert!(root.path().join("cmd/main").is_file());
    assert!(root.path().join("cmd/install_init").is_file());
    assert!(root.path().join("scripts/package.sh").is_file());
    assert!(root.path().join("config/privilege").is_file());
    assert!(root.path().join("config/resource").is_file());
    assert!(root.path().join("wizard/.keep").is_file());
    assert!(root.path().join("app/ui/config").is_file());
    assert!(root.path().join("app/www/index.html").is_file());
    assert!(root.path().join("app/www/assets/app.js").is_file());
    assert!(root.path().join("app/www/assets/app.css").is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.path().join("cmd/main"))
            .unwrap()
            .permissions()
            .mode();
        assert_ne!(mode & 0o111, 0);
    }

    let manifest = std::fs::read_to_string(root.path().join("manifest")).unwrap();
    assert!(manifest.contains("appname"));
    assert!(manifest.contains("aicore-web"));
    assert!(manifest.contains("desktop_uidir"));
    assert!(manifest.contains("platform"));
    assert!(manifest.contains("service_port"));
    assert!(!manifest.contains("placeholder"));

    let main = std::fs::read_to_string(root.path().join("cmd/main")).unwrap();
    assert!(main.contains("TRIM_APPDEST"));
    assert!(main.contains("TRIM_PKGHOME"));
    assert!(main.contains("TRIM_PKGVAR"));
    assert!(main.contains("SCRIPT_DIR"));
    assert!(main.contains("PKG_ROOT"));
    assert!(main.contains("resolve_app_bin"));
    assert!(main.contains("APP_DEST"));
    assert!(main.contains("target/server/aicore-web"));
    assert!(main.contains("app/server/aicore-web"));
    assert!(main.contains("TRIM_SERVICE_PORT"));
    assert!(main.contains("AICORE_WEB_HOST"));
    assert!(main.contains("0.0.0.0"));
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
