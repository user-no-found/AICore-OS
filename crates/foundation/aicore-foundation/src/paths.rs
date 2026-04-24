#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AicorePaths {
    pub home_root: String,
    pub state_root: String,
    pub components_root: String,
    pub instances_root: String,
    pub config_root: String,
}

impl AicorePaths {
    pub fn new(home_root: impl Into<String>) -> Self {
        let home_root = home_root.into();
        let state_root = format!("{home_root}/.aicore");

        Self {
            home_root,
            components_root: format!("{state_root}/components"),
            instances_root: format!("{state_root}/instances"),
            config_root: format!("{state_root}/config"),
            state_root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AicorePaths;

    #[test]
    fn builds_expected_default_layout() {
        let paths = AicorePaths::new("/home/demo");

        assert_eq!(paths.state_root, "/home/demo/.aicore");
        assert_eq!(paths.components_root, "/home/demo/.aicore/components");
        assert_eq!(paths.instances_root, "/home/demo/.aicore/instances");
        assert_eq!(paths.config_root, "/home/demo/.aicore/config");
    }
}
