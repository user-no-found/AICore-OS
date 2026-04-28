use std::path::PathBuf;

use crate::ConfigPaths;

impl ConfigPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();

        Self {
            auth_toml: root.join("auth.toml"),
            services_toml: root.join("services.toml"),
            providers_toml: root.join("providers.toml"),
            instances_dir: root.join("instances"),
            root,
        }
    }

    pub fn runtime_toml_for(&self, instance_id: &str) -> PathBuf {
        self.instances_dir.join(instance_id).join("runtime.toml")
    }
}
