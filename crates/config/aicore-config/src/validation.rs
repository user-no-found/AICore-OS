use aicore_auth::{AuthRef, GlobalAuthPool};

use crate::render::render_service_role;
use crate::{
    ConfigError, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ServiceProfileMode,
};

impl ConfigStore {
    pub fn validate_runtime_config(
        config: &InstanceRuntimeConfig,
        auth_pool: &GlobalAuthPool,
    ) -> Result<(), ConfigError> {
        if !auth_pool_contains(auth_pool, &config.primary.auth_ref) {
            return Err(ConfigError::Validation(format!(
                "primary auth_ref not found: {}",
                config.primary.auth_ref.as_str()
            )));
        }

        if let Some(fallback) = &config.fallback {
            if !auth_pool_contains(auth_pool, &fallback.auth_ref) {
                return Err(ConfigError::Validation(format!(
                    "fallback auth_ref not found: {}",
                    fallback.auth_ref.as_str()
                )));
            }
        }

        Ok(())
    }

    pub fn validate_service_profiles(
        services: &GlobalServiceProfiles,
        auth_pool: &GlobalAuthPool,
    ) -> Result<(), ConfigError> {
        for profile in &services.profiles {
            if profile.mode != ServiceProfileMode::Explicit {
                continue;
            }

            let auth_ref = profile.auth_ref.as_ref().ok_or_else(|| {
                ConfigError::Validation(format!(
                    "explicit service profile missing auth_ref: {}",
                    render_service_role(&profile.role)
                ))
            })?;

            let model = profile.model.as_ref().ok_or_else(|| {
                ConfigError::Validation(format!(
                    "explicit service profile missing model: {}",
                    render_service_role(&profile.role)
                ))
            })?;

            if model.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "explicit service profile missing model: {}",
                    render_service_role(&profile.role)
                )));
            }

            if !auth_pool_contains(auth_pool, auth_ref) {
                return Err(ConfigError::Validation(format!(
                    "explicit service profile auth_ref not found: {}",
                    auth_ref.as_str()
                )));
            }
        }

        Ok(())
    }
}

pub(crate) fn auth_pool_contains(auth_pool: &GlobalAuthPool, auth_ref: &AuthRef) -> bool {
    auth_pool
        .entries()
        .iter()
        .any(|entry| entry.auth_ref == *auth_ref)
}
