use aicore_auth::{AuthCapability, AuthKind, GlobalAuthPool};

use crate::{
    GlobalServiceProfiles, InstanceRuntimeConfig, ProviderProfilesConfig, ServiceProfileMode,
    ServiceRole,
};

pub(crate) fn render_auth_pool(pool: &GlobalAuthPool) -> String {
    let mut output = String::from(
        "# AICore OS auth pool
",
    );

    for entry in pool.entries() {
        output.push_str(
            "
[[auth]]
",
        );
        output.push_str(&format!(
            "auth_ref = {}
",
            render_string(entry.auth_ref.as_str())
        ));
        output.push_str(&format!(
            "provider = {}
",
            render_string(&entry.provider)
        ));
        output.push_str(&format!(
            "kind = {}
",
            render_string(render_auth_kind(&entry.kind))
        ));
        output.push_str(&format!(
            "secret_ref = {}
",
            render_string(entry.secret_ref.as_str())
        ));
        output.push_str(&format!(
            "capabilities = {}
",
            render_string_list(
                &entry
                    .capabilities
                    .iter()
                    .map(render_auth_capability)
                    .collect::<Vec<_>>()
            )
        ));
        output.push_str(&format!(
            "enabled = {}
",
            entry.enabled
        ));
    }

    output
}

pub(crate) fn render_services(services: &GlobalServiceProfiles) -> String {
    let mut output = String::from(
        "# AICore OS service profiles
",
    );

    for profile in &services.profiles {
        output.push_str(
            "
[[service]]
",
        );
        output.push_str(&format!(
            "role = {}
",
            render_string(render_service_role(&profile.role))
        ));
        output.push_str(&format!(
            "mode = {}
",
            render_string(render_service_profile_mode(&profile.mode))
        ));

        if let Some(auth_ref) = &profile.auth_ref {
            output.push_str(&format!(
                "auth_ref = {}
",
                render_string(auth_ref.as_str())
            ));
        }

        if let Some(model) = &profile.model {
            output.push_str(&format!(
                "model = {}
",
                render_string(model)
            ));
        }
    }

    output
}

pub(crate) fn render_provider_profiles(providers: &ProviderProfilesConfig) -> String {
    let mut output = String::from(
        "# AICore OS provider profiles
",
    );

    for profile in &providers.profiles {
        output.push_str(
            "
[[provider]]
",
        );
        output.push_str(&format!(
            "provider_id = {}
",
            render_string(&profile.provider_id)
        ));

        if let Some(base_url) = &profile.base_url {
            output.push_str(&format!(
                "base_url = {}
",
                render_string(base_url)
            ));
        }

        if let Some(api_mode) = &profile.api_mode {
            output.push_str(&format!(
                "api_mode = {}
",
                render_string(api_mode)
            ));
        }

        if let Some(engine_id) = &profile.engine_id {
            output.push_str(&format!(
                "engine_id = {}
",
                render_string(engine_id)
            ));
        }

        output.push_str(&format!(
            "enabled = {}
",
            profile.enabled
        ));
    }

    output
}

pub(crate) fn render_runtime(config: &InstanceRuntimeConfig) -> String {
    let mut output = String::from(
        "# AICore OS instance runtime
",
    );
    output.push_str(&format!(
        "instance_id = {}
",
        render_string(&config.instance_id)
    ));
    output.push_str(&format!(
        "primary_auth_ref = {}
",
        render_string(config.primary.auth_ref.as_str())
    ));
    output.push_str(&format!(
        "primary_model = {}
",
        render_string(&config.primary.model)
    ));

    if let Some(fallback) = &config.fallback {
        output.push_str(&format!(
            "fallback_auth_ref = {}
",
            render_string(fallback.auth_ref.as_str())
        ));
        output.push_str(&format!(
            "fallback_model = {}
",
            render_string(&fallback.model)
        ));
    }

    output
}

pub(crate) fn render_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

pub(crate) fn render_string_list(values: &[&str]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| render_string(value))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn render_auth_kind(kind: &AuthKind) -> &'static str {
    match kind {
        AuthKind::ApiKey => "api_key",
        AuthKind::OAuth => "oauth",
        AuthKind::Session => "session",
        AuthKind::Token => "token",
    }
}

pub(crate) fn render_auth_capability(capability: &AuthCapability) -> &'static str {
    match capability {
        AuthCapability::Chat => "chat",
        AuthCapability::Vision => "vision",
        AuthCapability::Search => "search",
        AuthCapability::Embedding => "embedding",
    }
}

pub(crate) fn render_service_role(role: &ServiceRole) -> &'static str {
    match role {
        ServiceRole::MemoryExtractor => "memory_extractor",
        ServiceRole::MemoryCurator => "memory_curator",
        ServiceRole::MemoryDreamer => "memory_dreamer",
        ServiceRole::EvolutionProposer => "evolution_proposer",
        ServiceRole::EvolutionReviewer => "evolution_reviewer",
        ServiceRole::Search => "search",
        ServiceRole::Tts => "tts",
        ServiceRole::ImageGeneration => "image_generation",
        ServiceRole::VideoGeneration => "video_generation",
        ServiceRole::Vision => "vision",
        ServiceRole::Reranker => "reranker",
    }
}

pub(crate) fn render_service_profile_mode(mode: &ServiceProfileMode) -> &'static str {
    match mode {
        ServiceProfileMode::InheritInstance => "inherit_instance",
        ServiceProfileMode::Explicit => "explicit",
        ServiceProfileMode::Disabled => "disabled",
    }
}
