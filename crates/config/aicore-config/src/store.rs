use std::{fs, path::Path};

use aicore_auth::GlobalAuthPool;

use crate::parse::{parse_auth_pool, parse_provider_profiles, parse_runtime, parse_services};
use crate::render::{render_auth_pool, render_provider_profiles, render_runtime, render_services};
use crate::{
    ConfigError, ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig,
    ProviderProfilesConfig,
};

impl ConfigStore {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn ensure_default_files(&self) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;

        if !self.paths.auth_toml.exists() {
            self.write_file(
                &self.paths.auth_toml,
                &render_auth_pool(&GlobalAuthPool::new(Vec::new())),
            )?;
        }

        if !self.paths.services_toml.exists() {
            self.write_file(
                &self.paths.services_toml,
                &render_services(&GlobalServiceProfiles {
                    profiles: Vec::new(),
                }),
            )?;
        }

        if !self.paths.providers_toml.exists() {
            self.write_file(
                &self.paths.providers_toml,
                &render_provider_profiles(&ProviderProfilesConfig {
                    profiles: Vec::new(),
                }),
            )?;
        }

        Ok(())
    }

    pub fn save_auth_pool(&self, pool: &GlobalAuthPool) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(&self.paths.auth_toml, &render_auth_pool(pool))
    }

    pub fn load_auth_pool(&self) -> Result<GlobalAuthPool, ConfigError> {
        let Some(contents) = self.read_file_if_exists(&self.paths.auth_toml)? else {
            return Ok(GlobalAuthPool::new(Vec::new()));
        };

        parse_auth_pool(&contents)
    }

    pub fn save_services(&self, services: &GlobalServiceProfiles) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(&self.paths.services_toml, &render_services(services))
    }

    pub fn load_services(&self) -> Result<GlobalServiceProfiles, ConfigError> {
        let Some(contents) = self.read_file_if_exists(&self.paths.services_toml)? else {
            return Ok(GlobalServiceProfiles {
                profiles: Vec::new(),
            });
        };

        parse_services(&contents)
    }

    pub fn save_provider_profiles(
        &self,
        providers: &ProviderProfilesConfig,
    ) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(
            &self.paths.providers_toml,
            &render_provider_profiles(providers),
        )
    }

    pub fn load_provider_profiles(&self) -> Result<ProviderProfilesConfig, ConfigError> {
        let Some(contents) = self.read_file_if_exists(&self.paths.providers_toml)? else {
            return Ok(ProviderProfilesConfig {
                profiles: Vec::new(),
            });
        };

        parse_provider_profiles(&contents)
    }

    pub fn save_instance_runtime(&self, config: &InstanceRuntimeConfig) -> Result<(), ConfigError> {
        self.ensure_root_dirs()?;
        self.write_file(
            &self.paths.runtime_toml_for(&config.instance_id),
            &render_runtime(config),
        )
    }

    pub fn load_instance_runtime(
        &self,
        instance_id: &str,
    ) -> Result<InstanceRuntimeConfig, ConfigError> {
        let path = self.paths.runtime_toml_for(instance_id);
        let contents = self.read_file_if_exists(&path)?.ok_or_else(|| {
            ConfigError::Io(format!("missing runtime config: {}", path.display()))
        })?;

        parse_runtime(&contents)
    }

    fn ensure_root_dirs(&self) -> Result<(), ConfigError> {
        fs::create_dir_all(&self.paths.root).map_err(io_error)?;
        fs::create_dir_all(&self.paths.instances_dir).map_err(io_error)?;
        Ok(())
    }

    fn read_file_if_exists(&self, path: &Path) -> Result<Option<String>, ConfigError> {
        match fs::read_to_string(path) {
            Ok(contents) => Ok(Some(contents)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error(error)),
        }
    }

    fn write_file(&self, path: &Path, contents: &str) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }

        fs::write(path, contents).map_err(io_error)
    }
}

pub(crate) fn io_error(error: std::io::Error) -> ConfigError {
    ConfigError::Io(error.to_string())
}
