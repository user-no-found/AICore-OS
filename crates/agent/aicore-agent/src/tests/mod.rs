#![allow(clippy::needless_pass_by_value)]

use std::{env, fs};

use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{InstanceRuntimeConfig, ModelBinding};
use aicore_kernel::default_runtime;
use aicore_kernel::{GatewaySource, InterruptMode, TransportEnvelope};
use aicore_memory::{MemoryKernel, MemoryPaths, MemoryPermanence, MemoryType, RememberInput};

use super::{
    AgentSessionContinuationPolicy, AgentSessionRunner, AgentSessionStopReason,
    AgentTurnFailureStage, AgentTurnInput, AgentTurnOutcome, AgentTurnRunner,
};

mod session;
mod surface;
mod turn;

fn temp_paths(name: &str) -> MemoryPaths {
    let root = env::temp_dir().join(format!("aicore-agent-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    MemoryPaths::new(root)
}

fn global_scope() -> aicore_memory::MemoryScope {
    aicore_memory::MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

fn auth_pool() -> GlobalAuthPool {
    GlobalAuthPool::new(vec![
        AuthEntry {
            auth_ref: AuthRef::new("auth.dummy.main"),
            provider: "dummy".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.dummy.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openai.main"),
            provider: "openai".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openai.main"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
    ])
}

fn auth_pool_search_only() -> GlobalAuthPool {
    GlobalAuthPool::new(vec![AuthEntry {
        auth_ref: AuthRef::new("auth.search.only"),
        provider: "dummy".to_string(),
        kind: AuthKind::ApiKey,
        secret_ref: SecretRef::new("secret://auth.search.only"),
        capabilities: vec![AuthCapability::Search],
        enabled: true,
    }])
}

fn runtime_config() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.dummy.main"),
            model: "dummy/default-chat".to_string(),
        },
        fallback: None,
    }
}

fn runtime_config_openrouter() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: None,
    }
}

fn runtime_config_missing_auth() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.missing"),
            model: "openai/gpt-5".to_string(),
        },
        fallback: None,
    }
}

fn runtime_config_search_only() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.search.only"),
            model: "dummy/default-chat".to_string(),
        },
        fallback: None,
    }
}

fn base_input(user_input: &str) -> AgentTurnInput {
    AgentTurnInput {
        instance_id: "global-main".to_string(),
        transport_envelope: TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        interrupt_mode: InterruptMode::Queue,
        scope: global_scope(),
        user_input: user_input.to_string(),
        memory_query: None,
        memory_limit: Some(8),
        memory_token_budget: 128,
        system_rules: "You are the AICore instance runtime.".to_string(),
        include_debug_prompt: true,
    }
}
