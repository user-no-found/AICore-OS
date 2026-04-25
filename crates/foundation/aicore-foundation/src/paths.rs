use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AicoreLayout {
    pub home_root: PathBuf,
    pub state_root: PathBuf,
    pub main_root: PathBuf,
    pub instances_root: PathBuf,
    pub config_root: PathBuf,
    pub secrets_root: PathBuf,
    pub components_root: PathBuf,
    pub run_root: PathBuf,
    pub logs_root: PathBuf,
}

impl AicoreLayout {
    pub fn new(home_root: impl Into<PathBuf>) -> Self {
        let home_root = home_root.into();
        let state_root = home_root.join(".aicore");

        Self {
            home_root: home_root.clone(),
            main_root: state_root.join("main"),
            instances_root: state_root.join("instances"),
            config_root: state_root.join("config"),
            secrets_root: state_root.join("secrets"),
            components_root: state_root.join("components"),
            run_root: state_root.join("run"),
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
        assert_eq!(layout.run_root, PathBuf::from("/home/demo/.aicore/run"));
        assert_eq!(layout.logs_root, PathBuf::from("/home/demo/.aicore/logs"));
    }
}
