use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const MANIFEST: &str = include_str!("../packaging/fnos/manifest.json");
const START_SH: &str = include_str!("../packaging/fnos/scripts/start.sh");
const PACKAGE_SH: &str = include_str!("../packaging/fnos/scripts/package.sh");
const CONFIG: &str = include_str!("../packaging/fnos/config/fnos.sample.toml");

pub fn write_package_skeleton(root: &Path) -> Result<(), String> {
    write(root.join("manifest.json"), MANIFEST)?;
    write_executable(root.join("scripts/start.sh"), START_SH)?;
    write_executable(root.join("scripts/package.sh"), PACKAGE_SH)?;
    write(root.join("config/fnos.sample.toml"), CONFIG)?;
    write(
        root.join("web/index.html"),
        include_str!("../web/dist/index.html"),
    )?;
    write(
        root.join("web/assets/app.js"),
        include_str!("../web/dist/assets/app.js"),
    )?;
    write(
        root.join("web/assets/app.css"),
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
    fn manifest_is_explicitly_placeholder() {
        assert!(super::MANIFEST.contains("aicore-web"));
        assert!(super::MANIFEST.contains("fnos_spec_status"));
        assert!(super::MANIFEST.contains("placeholder"));
    }
}
