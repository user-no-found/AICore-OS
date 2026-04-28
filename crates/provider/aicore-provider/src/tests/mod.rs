use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{
    InstanceRuntimeConfig, ModelBinding, ProviderProfileOverride, ProviderProfilesConfig,
};
use aicore_memory::{
    MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemoryType, RememberInput,
    SearchQuery,
};
use std::{
    env, fs,
    io::Write,
    process::{Command, Stdio},
};

use crate::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderAdapterStatus, ProviderApiMode,
    ProviderAuthMode, ProviderAvailability, ProviderEngineEvent, ProviderEngineEventKind,
    ProviderEngineManager, ProviderEngineMessage, ProviderEngineRequest, ProviderInvoker,
    ProviderKind, ProviderProfile, ProviderRegistry, ProviderResolver, ProviderRuntime,
    ProviderRuntimeResolveInput, ProviderRuntimeResolver,
};

mod engine;
mod invoker;
mod prompt;
mod resolver;
mod runtime;

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

fn auth_pool_with_disabled_entry() -> GlobalAuthPool {
    GlobalAuthPool::new(vec![AuthEntry {
        auth_ref: AuthRef::new("auth.dummy.main"),
        provider: "dummy".to_string(),
        kind: AuthKind::ApiKey,
        secret_ref: SecretRef::new("secret://auth.dummy.main"),
        capabilities: vec![AuthCapability::Chat],
        enabled: false,
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

fn runtime_config_openai() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.openai.main"),
            model: "gpt-4.1".to_string(),
        },
        fallback: None,
    }
}

fn auth_pool_for_provider(provider: &str) -> GlobalAuthPool {
    GlobalAuthPool::new(vec![AuthEntry {
        auth_ref: AuthRef::new("auth.test.main"),
        provider: provider.to_string(),
        kind: AuthKind::ApiKey,
        secret_ref: SecretRef::new("secret://auth.test.main"),
        capabilities: vec![AuthCapability::Chat],
        enabled: true,
    }])
}

fn runtime_for_model(model: &str) -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.test.main"),
            model: model.to_string(),
        },
        fallback: None,
    }
}

fn resolve_runtime(provider: &str, model: &str) -> crate::ProviderRuntime {
    resolve_model(provider, model).runtime
}

fn resolve_model(provider: &str, model: &str) -> crate::ResolvedModel {
    let auth_pool = auth_pool_for_provider(provider);
    let runtime = runtime_for_model(model);
    let registry = ProviderRegistry::builtin();

    ProviderRuntimeResolver::resolve(ProviderRuntimeResolveInput {
        auth_pool: &auth_pool,
        runtime: &runtime,
        registry: &registry,
    })
    .expect("runtime should resolve")
    .resolved_model
}

fn python3_available() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_fake_worker(content: &str) -> Option<Vec<ProviderEngineEvent>> {
    if !python3_available() {
        eprintln!("python3 unavailable; skipping fake worker smoke");
        return None;
    }

    let request = ProviderEngineRequest {
        protocol_version: "provider.engine.v1".to_string(),
        invocation_id: "inv-fake".to_string(),
        provider_id: "dummy".to_string(),
        adapter_id: "dummy".to_string(),
        engine_id: "python.fake".to_string(),
        api_mode: "dummy".to_string(),
        model: "dummy/default-chat".to_string(),
        base_url: None,
        credential_lease_ref: None,
        messages: vec![ProviderEngineMessage {
            role: "user".to_string(),
            content: content.to_string(),
        }],
        tools_json: None,
        parameters_json: None,
        stream: false,
        timeout_ms: None,
    };
    let request_json = serde_json::to_string(&request).expect("request should serialize");
    let python_root = format!("{}/python", env!("CARGO_MANIFEST_DIR"));
    let mut child = Command::new("python3")
        .arg("-m")
        .arg("aicore_provider_engine.worker")
        .arg("--engine")
        .arg("fake")
        .env("PYTHONPATH", python_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("fake worker should spawn");

    child
        .stdin
        .as_mut()
        .expect("stdin should be available")
        .write_all(format!("{request_json}\n").as_bytes())
        .expect("request should be written");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("worker should finish");
    assert!(
        output.status.success(),
        "worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(!stdout.contains("sk-live-secret-value"));
    assert!(!stderr.contains("sk-live-secret-value"));

    Some(
        stdout
            .lines()
            .map(|line| serde_json::from_str(line).expect("event line should parse"))
            .collect(),
    )
}

fn run_sdk_worker_with_env(
    engine: &str,
    api_mode: &str,
    extra_env: &[(&str, &str)],
) -> Option<(Vec<ProviderEngineEvent>, String, String)> {
    if !python3_available() {
        eprintln!("python3 unavailable; skipping SDK worker smoke");
        return None;
    }

    let request = ProviderEngineRequest {
        protocol_version: "provider.engine.v1".to_string(),
        invocation_id: format!("inv-{engine}"),
        provider_id: engine.to_string(),
        adapter_id: engine.to_string(),
        engine_id: format!("python.{engine}"),
        api_mode: api_mode.to_string(),
        model: "test-model".to_string(),
        base_url: None,
        credential_lease_ref: Some("env:AICORE_PROVIDER_TEST_SECRET".to_string()),
        messages: vec![ProviderEngineMessage {
            role: "user".to_string(),
            content: "ping".to_string(),
        }],
        tools_json: None,
        parameters_json: None,
        stream: false,
        timeout_ms: None,
    };
    let request_json = serde_json::to_string(&request).expect("request should serialize");
    let python_root = format!("{}/python", env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new("python3");
    command
        .arg("-m")
        .arg("aicore_provider_engine.worker")
        .arg("--engine")
        .arg(engine)
        .env("PYTHONPATH", python_root)
        .env("AICORE_PROVIDER_TEST_SECRET", "sk-live-secret-value")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }

    let mut child = command.spawn().expect("SDK worker should spawn");
    child
        .stdin
        .as_mut()
        .expect("stdin should be available")
        .write_all(format!("{request_json}\n").as_bytes())
        .expect("request should be written");
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("worker should finish");
    assert!(
        output.status.success(),
        "worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    let events = stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("event line should parse"))
        .collect();

    Some((events, stdout, stderr))
}

fn temp_paths(name: &str) -> MemoryPaths {
    let root = env::temp_dir().join(format!("aicore-provider-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    MemoryPaths::new(root)
}

fn global_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}
