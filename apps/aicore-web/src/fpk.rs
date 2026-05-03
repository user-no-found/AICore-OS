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
        assert!(!super::MANIFEST.contains("placeholder"));
    }

    #[test]
    fn lifecycle_script_runs_host_native_server() {
        assert!(super::MAIN.contains("TRIM_PKGHOME"));
        assert!(super::MAIN.contains("TRIM_PKGVAR"));
        assert!(super::MAIN.contains("SCRIPT_DIR"));
        assert!(super::MAIN.contains("app/server/aicore-web"));
        assert!(super::MAIN.contains("--host"));
        assert!(super::MAIN.contains("--port"));
    }
}
