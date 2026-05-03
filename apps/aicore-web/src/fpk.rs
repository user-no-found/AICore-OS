use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const MANIFEST: &str = include_str!("../packaging/fnos/package/manifest");
const PRIVILEGE: &str = include_str!("../packaging/fnos/package/config/privilege");
const RESOURCE: &str = include_str!("../packaging/fnos/package/config/resource");
const MAIN: &str = include_str!("../packaging/fnos/package/cmd/main");
const UI_CONFIG: &str = include_str!("../packaging/fnos/package/app/ui/config");
const PACKAGE_SH: &str = include_str!("../packaging/fnos/scripts/package.sh");
const LIFECYCLE_HOOKS: &[&str] = &[
    "install_init",
    "install_callback",
    "uninstall_init",
    "uninstall_callback",
    "config_init",
    "config_callback",
    "upgrade_init",
    "upgrade_callback",
];

pub fn write_package_source(root: &Path) -> Result<(), String> {
    write(root.join("manifest"), MANIFEST)?;
    write(root.join("config/privilege"), PRIVILEGE)?;
    write(root.join("config/resource"), RESOURCE)?;
    write(root.join("app/ui/config"), UI_CONFIG)?;
    write(root.join("wizard/.keep"), "")?;
    write_executable(root.join("cmd/main"), MAIN)?;
    for hook in LIFECYCLE_HOOKS {
        write_executable(root.join("cmd").join(hook), "#!/bin/bash\nexit 0\n")?;
    }
    write_executable(root.join("scripts/package.sh"), PACKAGE_SH)?;
    write(
        root.join("app/www/index.html"),
        include_str!("../web/dist/index.html"),
    )?;
    write(
        root.join("app/www/assets/app.js"),
        include_str!("../web/dist/assets/app.js"),
    )?;
    write(
        root.join("app/www/assets/app.css"),
        include_str!("../web/dist/assets/app.css"),
    )
}

fn write_executable(path: impl AsRef<Path>, content: &str) -> Result<(), String> {
    write(&path, content)?;
    #[cfg(unix)]
    {
        let path = path.as_ref();
        let mut permissions = fs::metadata(path)
            .map_err(|error| format!("读取 {} 权限失败：{error}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .map_err(|error| format!("设置 {} 可执行权限失败：{error}", path.display()))?;
    }
    Ok(())
}

fn write(path: impl AsRef<Path>, content: &str) -> Result<(), String> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建目录 {} 失败：{error}", parent.display()))?;
    }
    fs::write(path, content).map_err(|error| format!("写入 {} 失败：{error}", path.display()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn manifest_uses_fnos_native_fields() {
        assert!(super::MANIFEST.contains("appname"));
        assert!(super::MANIFEST.contains("aicore-web"));
        assert!(super::MANIFEST.contains("desktop_uidir"));
        assert!(super::MANIFEST.contains("platform"));
        assert!(super::MANIFEST.contains("service_port"));
        assert!(!super::MANIFEST.contains("placeholder"));
    }

    #[test]
    fn manifest_version_uses_manual_fpk_sequence() {
        let version_line = super::MANIFEST
            .lines()
            .find(|line| line.trim_start().starts_with("version"))
            .expect("manifest should define version");
        let version = version_line
            .split_once('=')
            .expect("version line should use key-value form")
            .1
            .trim();
        let patch = version
            .strip_prefix("0.0.")
            .expect("FPK version should use 0.0.x");

        assert!(!patch.is_empty());
        assert!(patch.chars().all(|value| value.is_ascii_digit()));
        assert!(!version.contains('-'));
    }

    #[test]
    fn package_script_keeps_manifest_version_without_generated_suffix() {
        assert!(super::PACKAGE_SH.contains("read_fpk_version"));
        assert!(super::PACKAGE_SH.contains("0.0.x"));
        assert!(!super::PACKAGE_SH.contains("AICORE_WEB_FPK_VERSION"));
        assert!(!super::PACKAGE_SH.contains("rev-parse"));
        assert!(!super::PACKAGE_SH.contains("sed -i"));
    }

    #[test]
    fn lifecycle_script_runs_host_native_server() {
        assert!(super::MAIN.contains("TRIM_APPDEST"));
        assert!(super::MAIN.contains("TRIM_PKGHOME"));
        assert!(super::MAIN.contains("TRIM_PKGVAR"));
        assert!(super::MAIN.contains("SCRIPT_DIR"));
        assert!(super::MAIN.contains("PKG_ROOT"));
        assert!(super::MAIN.contains("APP_DEST"));
        assert!(super::MAIN.contains("resolve_app_bin"));
        assert!(super::MAIN.contains("target/server/aicore-web"));
        assert!(super::MAIN.contains("app/server/aicore-web"));
        assert!(super::MAIN.contains("server/aicore-web"));
        assert!(super::MAIN.contains("TRIM_SERVICE_PORT"));
        assert!(super::MAIN.contains("--host"));
        assert!(super::MAIN.contains("--port"));
    }

    #[test]
    fn packaged_web_shell_has_static_first_paint_and_runtime_render() {
        let index = include_str!("../web/dist/index.html");
        let js = include_str!("../web/dist/assets/app.js");
        assert!(index.contains("静态首屏"));
        assert!(index.contains("./assets/app.js"));
        assert!(index.contains("./assets/app.css"));
        assert!(js.contains("createApp"));
        assert!(!js.contains("template:"));
    }
}
