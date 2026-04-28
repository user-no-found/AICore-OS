#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub(super) fn is_executable_file(path: &std::path::Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        return std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub(super) fn binary_health(path: &std::path::Path) -> &'static str {
    if !path.exists() {
        "missing"
    } else if is_executable_file(path) {
        "ok"
    } else {
        "not_executable"
    }
}
