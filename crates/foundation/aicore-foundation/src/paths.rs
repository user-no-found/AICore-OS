use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AicoreLayout {
    pub home_root: PathBuf,
    pub state_root: PathBuf,
    pub bin_root: PathBuf,
    pub runtime_root: PathBuf,
    pub runtime_foundation_root: PathBuf,
    pub runtime_kernel_root: PathBuf,
    pub main_root: PathBuf,
    pub instances_root: PathBuf,
    pub config_root: PathBuf,
    pub secrets_root: PathBuf,
    pub components_root: PathBuf,
    pub share_root: PathBuf,
    pub manifests_root: PathBuf,
    pub contracts_root: PathBuf,
    pub schemas_root: PathBuf,
    pub kernel_state_root: PathBuf,
    pub run_root: PathBuf,
    pub cache_root: PathBuf,
    pub logs_root: PathBuf,
}

impl AicoreLayout {
    pub fn new(home_root: impl Into<PathBuf>) -> Self {
        let home_root = home_root.into();
        let state_root = home_root.join(".aicore");

        Self {
            home_root: home_root.clone(),
            bin_root: state_root.join("bin"),
            runtime_root: state_root.join("runtime"),
            runtime_foundation_root: state_root.join("runtime/foundation"),
            runtime_kernel_root: state_root.join("runtime/kernel"),
            main_root: state_root.join("main"),
            instances_root: state_root.join("instances"),
            config_root: state_root.join("config"),
            secrets_root: state_root.join("secrets"),
            components_root: state_root.join("components"),
            share_root: state_root.join("share"),
            manifests_root: state_root.join("share/manifests"),
            contracts_root: state_root.join("share/contracts"),
            schemas_root: state_root.join("share/schemas"),
            kernel_state_root: state_root.join("state/kernel"),
            run_root: state_root.join("run"),
            cache_root: state_root.join("cache"),
            logs_root: state_root.join("logs"),
            state_root,
        }
    }

    pub fn from_system_home() -> Self {
        let home_root = env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/home/unknown"));
        Self::new(home_root)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::AicoreLayout;

    #[test]
    fn builds_expected_default_layout() {
        let layout = AicoreLayout::new("/home/demo");

        assert_eq!(layout.state_root, PathBuf::from("/home/demo/.aicore"));
        assert_eq!(layout.bin_root, PathBuf::from("/home/demo/.aicore/bin"));
        assert_eq!(
            layout.runtime_root,
            PathBuf::from("/home/demo/.aicore/runtime")
        );
        assert_eq!(
            layout.runtime_foundation_root,
            PathBuf::from("/home/demo/.aicore/runtime/foundation")
        );
        assert_eq!(
            layout.runtime_kernel_root,
            PathBuf::from("/home/demo/.aicore/runtime/kernel")
        );
        assert_eq!(layout.main_root, PathBuf::from("/home/demo/.aicore/main"));
        assert_eq!(
            layout.components_root,
            PathBuf::from("/home/demo/.aicore/components")
        );
        assert_eq!(
            layout.instances_root,
            PathBuf::from("/home/demo/.aicore/instances")
        );
        assert_eq!(
            layout.config_root,
            PathBuf::from("/home/demo/.aicore/config")
        );
        assert_eq!(
            layout.secrets_root,
            PathBuf::from("/home/demo/.aicore/secrets")
        );
        assert_eq!(layout.share_root, PathBuf::from("/home/demo/.aicore/share"));
        assert_eq!(
            layout.manifests_root,
            PathBuf::from("/home/demo/.aicore/share/manifests")
        );
        assert_eq!(
            layout.contracts_root,
            PathBuf::from("/home/demo/.aicore/share/contracts")
        );
        assert_eq!(
            layout.schemas_root,
            PathBuf::from("/home/demo/.aicore/share/schemas")
        );
        assert_eq!(
            layout.kernel_state_root,
            PathBuf::from("/home/demo/.aicore/state/kernel")
        );
        assert_eq!(layout.run_root, PathBuf::from("/home/demo/.aicore/run"));
        assert_eq!(layout.cache_root, PathBuf::from("/home/demo/.aicore/cache"));
        assert_eq!(layout.logs_root, PathBuf::from("/home/demo/.aicore/logs"));
    }
}
