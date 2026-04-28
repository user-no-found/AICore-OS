#[cfg(unix)]
pub(crate) use std::os::unix::fs::PermissionsExt;
pub(crate) use std::{fs, path::PathBuf, process::Command};

pub(crate) use aicore_memory::{
    MemoryAgentOutput, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryProposal,
    MemoryProposalStatus, MemoryRequestedOutput, MemoryScope, MemorySource, MemoryTrigger,
    MemoryType, MemoryWorkBatch, RememberInput, RuleBasedMemoryAgent,
};

pub(crate) fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("aicore-cli-p46-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp root should be removable");
    }
    root
}

pub(crate) fn run_cli_with_config_root(args: &[&str], root: &PathBuf) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_aicore-cli"))
        .args(args)
        .env("AICORE_CONFIG_ROOT", root)
        .output()
        .expect("aicore-cli should run")
}

pub(crate) fn run_cli_with_config_root_and_env(
    args: &[&str],
    root: &PathBuf,
    envs: &[(&str, &str)],
) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aicore-cli"));
    command.args(args).env("AICORE_CONFIG_ROOT", root);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("aicore-cli should run")
}

pub(crate) fn run_cli_with_env(args: &[&str], envs: &[(&str, &str)]) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_aicore-cli"));
    command.args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("aicore-cli should run")
}

pub(crate) fn assert_json_lines(stdout: &str) -> Vec<serde_json::Value> {
    let lines = stdout.lines().collect::<Vec<_>>();
    assert!(!lines.is_empty(), "json output should contain lines");
    lines
        .into_iter()
        .map(|line| serde_json::from_str(line).expect("stdout line should be valid json"))
        .collect()
}

pub(crate) fn assert_has_json_event(events: &[serde_json::Value], event_name: &str) {
    assert!(
        events.iter().any(|event| event["event"] == event_name),
        "json output should contain {event_name}"
    );
}

pub(crate) fn memory_paths_for_root(root: &PathBuf) -> MemoryPaths {
    MemoryPaths::new(root.join("instances").join("global-main").join("memory"))
}

pub(crate) fn seed_open_proposal(root: &PathBuf, memory_type: MemoryType, content: &str) -> String {
    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .submit_agent_output(MemoryAgentOutput {
            proposals: vec![MemoryProposal {
                proposal_id: "agent_prop_seed".to_string(),
                memory_type,
                scope: MemoryScope::GlobalMain {
                    instance_id: "global-main".to_string(),
                },
                source: MemorySource::RuleBasedAgent,
                status: MemoryProposalStatus::Rejected,
                content: content.to_string(),
                content_language: if content.is_ascii() {
                    "en".to_string()
                } else {
                    "zh-CN".to_string()
                },
                normalized_content: content.to_string(),
                normalized_language: if content.is_ascii() {
                    "en".to_string()
                } else {
                    "zh-CN".to_string()
                },
                localized_summary: content.to_string(),
                created_at: "0".to_string(),
            }],
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        })
        .expect("agent output should be stored")
        .into_iter()
        .next()
        .expect("proposal id should exist")
}

pub(crate) fn global_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

pub(crate) fn seed_rule_based_proposal(
    root: &PathBuf,
    trigger: MemoryTrigger,
    excerpt: &str,
) -> String {
    let output = RuleBasedMemoryAgent::analyze(&MemoryWorkBatch {
        instance_id: "global-main".to_string(),
        scope: global_scope(),
        trigger,
        recent_events_summary: String::new(),
        raw_excerpts: vec![excerpt.to_string()],
        existing_memory_hits: Vec::new(),
        token_budget: 1024,
        requested_outputs: vec![MemoryRequestedOutput::Proposals],
    });

    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .submit_agent_output(output)
        .expect("agent output should be stored")
        .into_iter()
        .next()
        .expect("proposal id should exist")
}

pub(crate) fn seed_memory_record(
    root: &PathBuf,
    memory_type: MemoryType,
    permanence: MemoryPermanence,
    content: &str,
) -> String {
    let mut kernel =
        MemoryKernel::open(memory_paths_for_root(root)).expect("memory kernel should open");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type,
            permanence,
            scope: global_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed")
}

pub(crate) fn seed_route_manifest(
    home: &PathBuf,
    file_name: &str,
    app_id: &str,
    capabilities: &[(&str, &str)],
) {
    let manifests = home.join(".aicore").join("share").join("manifests");
    fs::create_dir_all(&manifests).expect("manifest dir should be creatable");
    let mut content = format!(
        "component_id = \"{app_id}\"\napp_id = \"{app_id}\"\nkind = \"app\"\nentrypoint = \"{}\"\ncontract_version = \"kernel.app.v1\"\n",
        home.join(".aicore").join("bin").join(app_id).display()
    );
    for (id, operation) in capabilities {
        content.push_str(&format!(
            "\n[[capabilities]]\nid = \"{id}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n"
        ));
    }
    fs::write(manifests.join(file_name), content).expect("manifest should be writable");
}

pub(crate) fn seed_process_smoke_manifest(home: &PathBuf) {
    let manifests = home.join(".aicore").join("share").join("manifests");
    fs::create_dir_all(&manifests).expect("manifest dir should be creatable");
    let entrypoint = env!("CARGO_BIN_EXE_aicore-cli");
    let content = format!(
        "component_id = \"aicore-component-smoke\"\napp_id = \"aicore-cli\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"stdio_jsonl\"\nargs = [\"__component-smoke-stdio\"]\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"component.process.smoke\"\noperation = \"component.process.smoke\"\nvisibility = \"diagnostic\"\n",
        entrypoint.replace('"', "\\\"")
    );
    fs::write(manifests.join("aicore-component-smoke.toml"), content)
        .expect("process smoke manifest should be writable");
}

pub(crate) fn seed_config_validate_manifest(home: &PathBuf) {
    let manifests = home.join(".aicore").join("share").join("manifests");
    fs::create_dir_all(&manifests).expect("manifest dir should be creatable");
    let entrypoint = env!("CARGO_BIN_EXE_aicore-cli");
    let content = format!(
        "component_id = \"aicore-config-validate\"\napp_id = \"aicore-cli\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"stdio_jsonl\"\nargs = [\"__component-config-validate-stdio\"]\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"config.validate\"\noperation = \"config.validate\"\nvisibility = \"user\"\n",
        entrypoint.replace('"', "\\\"")
    );
    fs::write(manifests.join("aicore-config-validate.toml"), content)
        .expect("config validate manifest should be writable");
}

pub(crate) fn seed_auth_model_service_manifests(home: &PathBuf) {
    for (file_name, component_id, operation, arg) in [
        (
            "aicore-auth-list.toml",
            "aicore-auth-list",
            "auth.list",
            "__component-auth-list-stdio",
        ),
        (
            "aicore-model-show.toml",
            "aicore-model-show",
            "model.show",
            "__component-model-show-stdio",
        ),
        (
            "aicore-service-list.toml",
            "aicore-service-list",
            "service.list",
            "__component-service-list-stdio",
        ),
    ] {
        seed_readonly_component_manifest(home, file_name, component_id, operation, arg);
    }
}

fn seed_readonly_component_manifest(
    home: &PathBuf,
    file_name: &str,
    component_id: &str,
    operation: &str,
    arg: &str,
) {
    let manifests = home.join(".aicore").join("share").join("manifests");
    fs::create_dir_all(&manifests).expect("manifest dir should be creatable");
    let entrypoint = env!("CARGO_BIN_EXE_aicore-cli");
    let content = format!(
        "component_id = \"{component_id}\"\napp_id = \"aicore-cli\"\nkind = \"app\"\nentrypoint = \"{}\"\ninvocation_mode = \"local_process\"\ntransport = \"stdio_jsonl\"\nargs = [\"{arg}\"]\ncontract_version = \"kernel.app.v1\"\n\n[[capabilities]]\nid = \"{operation}\"\noperation = \"{operation}\"\nvisibility = \"user\"\n",
        entrypoint.replace('"', "\\\"")
    );
    fs::write(manifests.join(file_name), content).expect("readonly manifest should be writable");
}

pub(crate) fn seed_global_runtime_metadata(home: &PathBuf) {
    let foundation = home.join(".aicore").join("runtime").join("foundation");
    let kernel = home.join(".aicore").join("runtime").join("kernel");
    let bin = home.join(".aicore").join("bin");

    fs::create_dir_all(&foundation).expect("foundation runtime dir should be creatable");
    fs::create_dir_all(&kernel).expect("kernel runtime dir should be creatable");
    fs::create_dir_all(&bin).expect("bin dir should be creatable");

    fs::write(foundation.join("install.toml"), "layer = \"foundation\"\n")
        .expect("foundation install metadata should be writable");
    fs::write(kernel.join("install.toml"), "layer = \"kernel\"\n")
        .expect("kernel install metadata should be writable");
    fs::write(
        kernel.join("version.toml"),
        "contract_version = \"kernel.runtime.v1\"\n",
    )
    .expect("kernel version metadata should be writable");
}

pub(crate) fn seed_foundation_runtime_binary(home: &PathBuf) {
    seed_executable(
        &home.join(".aicore").join("bin").join("aicore-foundation"),
        "#!/bin/sh\necho foundation-runtime-ok\n",
    );
}

pub(crate) fn seed_kernel_runtime_binary_fixture(home: &PathBuf) {
    let script = r#"#!/bin/sh
request=$(cat)
mkdir -p "$HOME/.aicore/state/kernel"
if printf '%s' "$request" | grep -q 'component.process.smoke'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.process.accepted","timestamp":"0","invocation_id":"invoke.fixture.process","trace_id":"trace.default","instance_id":"global-main","operation":"component.process.smoke","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.process.route","timestamp":"0","invocation_id":"invoke.fixture.process","trace_id":"trace.default","instance_id":"global-main","operation":"component.process.smoke","stage":"route_decision_made","status":"ok","component_id":"aicore-component-smoke","app_id":"aicore-cli","capability_id":"component.process.smoke","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.process.handler","timestamp":"0","invocation_id":"invoke.fixture.process","trace_id":"trace.default","instance_id":"global-main","operation":"component.process.smoke","stage":"handler_executed","status":"ok","component_id":"aicore-component-smoke","app_id":"aicore-cli","capability_id":"component.process.smoke","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":false,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.process.event","timestamp":"0","invocation_id":"invoke.fixture.process","trace_id":"trace.default","instance_id":"global-main","operation":"component.process.smoke","stage":"event_generated","status":"ok","component_id":"aicore-component-smoke","app_id":"aicore-cli","capability_id":"component.process.smoke","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.process.completed","timestamp":"0","invocation_id":"invoke.fixture.process","trace_id":"trace.default","instance_id":"global-main","operation":"component.process.smoke","stage":"invocation_completed","status":"ok","component_id":"aicore-component-smoke","app_id":"aicore-cli","capability_id":"component.process.smoke","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.process","trace_id":"trace.default","operation":"component.process.smoke","status":"completed","route":{"component_id":"aicore-component-smoke","app_id":"aicore-cli","capability_id":"component.process.smoke","contract_version":"kernel.app.v1"},"handler":{"kind":"local_process","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"component.process.smoke","summary":"process smoke handled by installed kernel runtime binary","fields":{"operation":"component.process.smoke","ipc":"stdio_jsonl","component_process":"ok","kernel_invocation_path":"binary"}},"failure":{"stage":null,"reason":null}}}'
exit 0
fi
if printf '%s' "$request" | grep -q 'config.validate'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.config.accepted","timestamp":"0","invocation_id":"invoke.fixture.config","trace_id":"trace.default","instance_id":"global-main","operation":"config.validate","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.config.route","timestamp":"0","invocation_id":"invoke.fixture.config","trace_id":"trace.default","instance_id":"global-main","operation":"config.validate","stage":"route_decision_made","status":"ok","component_id":"aicore-config-validate","app_id":"aicore-cli","capability_id":"config.validate","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.config.handler","timestamp":"0","invocation_id":"invoke.fixture.config","trace_id":"trace.default","instance_id":"global-main","operation":"config.validate","stage":"handler_executed","status":"ok","component_id":"aicore-config-validate","app_id":"aicore-cli","capability_id":"config.validate","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":false,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.config.event","timestamp":"0","invocation_id":"invoke.fixture.config","trace_id":"trace.default","instance_id":"global-main","operation":"config.validate","stage":"event_generated","status":"ok","component_id":"aicore-config-validate","app_id":"aicore-cli","capability_id":"config.validate","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.config.completed","timestamp":"0","invocation_id":"invoke.fixture.config","trace_id":"trace.default","instance_id":"global-main","operation":"config.validate","stage":"invocation_completed","status":"ok","component_id":"aicore-config-validate","app_id":"aicore-cli","capability_id":"config.validate","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.config","trace_id":"trace.default","operation":"config.validate","status":"completed","route":{"component_id":"aicore-config-validate","app_id":"aicore-cli","capability_id":"config.validate","contract_version":"kernel.app.v1"},"handler":{"kind":"local_process","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"config.validate","summary":"配置校验通过","fields":{"operation":"config.validate","valid":"true","config_root":"fixture-config-root","checked_files":"auth.toml, services.toml, providers.toml, instances/global-main/runtime.toml","auth_pool_present":"true","runtime_config_present":"true","service_profiles_present":"true","provider_profiles_present":"true","error_count":"0","warning_count":"0","diagnostics":"配置校验通过","kernel_invocation_path":"binary"}},"failure":{"stage":null,"reason":null}}}'
exit 0
fi
if printf '%s' "$request" | grep -q 'auth.list'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.auth.accepted","timestamp":"0","invocation_id":"invoke.fixture.auth","trace_id":"trace.default","instance_id":"global-main","operation":"auth.list","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.auth.route","timestamp":"0","invocation_id":"invoke.fixture.auth","trace_id":"trace.default","instance_id":"global-main","operation":"auth.list","stage":"route_decision_made","status":"ok","component_id":"aicore-auth-list","app_id":"aicore-cli","capability_id":"auth.list","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.auth.handler","timestamp":"0","invocation_id":"invoke.fixture.auth","trace_id":"trace.default","instance_id":"global-main","operation":"auth.list","stage":"handler_executed","status":"ok","component_id":"aicore-auth-list","app_id":"aicore-cli","capability_id":"auth.list","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":false,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.auth.event","timestamp":"0","invocation_id":"invoke.fixture.auth","trace_id":"trace.default","instance_id":"global-main","operation":"auth.list","stage":"event_generated","status":"ok","component_id":"aicore-auth-list","app_id":"aicore-cli","capability_id":"auth.list","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.auth.completed","timestamp":"0","invocation_id":"invoke.fixture.auth","trace_id":"trace.default","instance_id":"global-main","operation":"auth.list","stage":"invocation_completed","status":"ok","component_id":"aicore-auth-list","app_id":"aicore-cli","capability_id":"auth.list","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.auth","trace_id":"trace.default","operation":"auth.list","status":"completed","route":{"component_id":"aicore-auth-list","app_id":"aicore-cli","capability_id":"auth.list","contract_version":"kernel.app.v1"},"handler":{"kind":"local_process","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"auth.list","summary":"认证池读取完成：2 条 auth_ref","fields":{"operation":"auth.list","auth_count":"2","entries":"[{\"auth_ref\":\"auth.dummy.main\",\"provider\":\"dummy\",\"kind\":\"api-key\",\"enabled\":true,\"capabilities\":[\"chat\"],\"secret\":\"configured\"}]","kernel_invocation_path":"binary"}},"failure":{"stage":null,"reason":null}}}'
exit 0
fi
if printf '%s' "$request" | grep -q 'model.show'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.model.accepted","timestamp":"0","invocation_id":"invoke.fixture.model","trace_id":"trace.default","instance_id":"global-main","operation":"model.show","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.model.completed","timestamp":"0","invocation_id":"invoke.fixture.model","trace_id":"trace.default","instance_id":"global-main","operation":"model.show","stage":"invocation_completed","status":"ok","component_id":"aicore-model-show","app_id":"aicore-cli","capability_id":"model.show","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.model","trace_id":"trace.default","operation":"model.show","status":"completed","route":{"component_id":"aicore-model-show","app_id":"aicore-cli","capability_id":"model.show","contract_version":"kernel.app.v1"},"handler":{"kind":"local_process","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"model.show","summary":"实例模型配置读取完成","fields":{"operation":"model.show","primary_model":"dummy/default-chat","primary_auth_ref":"auth.dummy.main","provider":"dummy","provider_kind":"api-key","runtime_config_present":"true","kernel_invocation_path":"binary"}},"failure":{"stage":null,"reason":null}}}'
exit 0
fi
if printf '%s' "$request" | grep -q 'service.list'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.service.accepted","timestamp":"0","invocation_id":"invoke.fixture.service","trace_id":"trace.default","instance_id":"global-main","operation":"service.list","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.service.completed","timestamp":"0","invocation_id":"invoke.fixture.service","trace_id":"trace.default","instance_id":"global-main","operation":"service.list","stage":"invocation_completed","status":"ok","component_id":"aicore-service-list","app_id":"aicore-cli","capability_id":"service.list","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"local_process","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.service","trace_id":"trace.default","operation":"service.list","status":"completed","route":{"component_id":"aicore-service-list","app_id":"aicore-cli","capability_id":"service.list","contract_version":"kernel.app.v1"},"handler":{"kind":"local_process","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"service.list","summary":"服务角色配置读取完成：3 个角色","fields":{"operation":"service.list","service_count":"3","services":"[{\"role\":\"search\",\"mode\":\"explicit\",\"auth_ref\":\"auth.openrouter.search\",\"model\":\"perplexity/sonar\",\"enabled\":true}]","kernel_invocation_path":"binary"}},"failure":{"stage":null,"reason":null}}}'
exit 0
fi
if printf '%s' "$request" | grep -q 'provider.smoke'; then
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.accepted","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"provider.smoke","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.route","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"provider.smoke","stage":"route_decision_made","status":"ok","component_id":"aicore","app_id":"aicore","capability_id":"provider.smoke","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.lookup","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"provider.smoke","stage":"handler_lookup_failed","status":"failed","component_id":"aicore","app_id":"aicore","capability_id":"provider.smoke","contract_version":"kernel.app.v1","failure_stage":"handler_lookup","failure_reason":"missing readonly handler","handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.failed","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"provider.smoke","stage":"invocation_failed","status":"failed","component_id":"aicore","app_id":"aicore","capability_id":"provider.smoke","contract_version":"kernel.app.v1","failure_stage":"handler_lookup","failure_reason":"missing readonly handler","handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.binary","trace_id":"trace.default","operation":"provider.smoke","status":"failed","route":{"component_id":"aicore","app_id":"aicore","capability_id":"provider.smoke","contract_version":"kernel.app.v1"},"handler":{"kind":null,"invocation_mode":"in_process","transport":"unsupported","process_exit_code":null,"executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":4},"result":{"kind":null,"summary":null,"fields":{}},"failure":{"stage":"handler_lookup","reason":"missing readonly handler"}}}'
exit 1
fi
cat >> "$HOME/.aicore/state/kernel/invocation-ledger.jsonl" <<'LEDGER'
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.accepted","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"runtime.status","stage":"accepted","status":"ok","component_id":null,"app_id":null,"capability_id":null,"contract_version":null,"failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.route","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"runtime.status","stage":"route_decision_made","status":"ok","component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":null,"handler_executed":false,"event_generated":false,"spawned_process":false,"called_real_component":false,"transport":null,"process_exit_code":null}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.handler","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"runtime.status","stage":"handler_executed","status":"ok","component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"kernel_runtime_binary","handler_executed":true,"event_generated":false,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.event","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"runtime.status","stage":"event_generated","status":"ok","component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"kernel_runtime_binary","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
{"schema_version":"aicore.kernel.invocation_ledger.v1","record_id":"ledger.fixture.completed","timestamp":"0","invocation_id":"invoke.fixture.binary","trace_id":"trace.default","instance_id":"global-main","operation":"runtime.status","stage":"invocation_completed","status":"ok","component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1","failure_stage":null,"failure_reason":null,"handler_kind":"kernel_runtime_binary","handler_executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"transport":"stdio_jsonl","process_exit_code":0}
LEDGER
printf '%s\n' '{"event":"kernel.invocation.result","schema_version":"aicore.kernel.runtime_binary.response.v1","protocol":"stdio_jsonl","protocol_version":"aicore.kernel.runtime_binary.stdio_jsonl.v1","contract_version":"kernel.runtime.v1","payload":{"invocation_id":"invoke.fixture.binary","trace_id":"trace.default","operation":"runtime.status","status":"completed","route":{"component_id":"aicore","app_id":"aicore","capability_id":"runtime.status","contract_version":"kernel.app.v1"},"handler":{"kind":"kernel_runtime_binary","invocation_mode":"local_process","transport":"stdio_jsonl","process_exit_code":0,"executed":true,"event_generated":true,"spawned_process":true,"called_real_component":false,"first_party_in_process_adapter":false},"ledger":{"appended":true,"path":"fixture-ledger","records":5},"result":{"kind":"runtime.status","summary":"runtime status from binary fixture","fields":{"global_root":"fixture-root","foundation_installed":"yes","kernel_installed":"yes","contract_version":"kernel.runtime.v1","manifest_count":"1","capability_count":"2","event_ledger_path":"fixture-ledger","bin_path":"fixture-bin","bin_path_status":"active","foundation_runtime_binary":"installed","kernel_runtime_binary":"installed","kernel_invocation_path":"binary_fixture"}},"failure":{"stage":null,"reason":null}}}'
"#;
    seed_executable(
        &home.join(".aicore").join("bin").join("aicore-kernel"),
        script,
    );
}

pub(crate) fn seed_executable(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("binary parent should be creatable");
    }
    fs::write(path, content).expect("binary fixture should be writable");
    #[cfg(unix)]
    {
        let mut permissions = fs::metadata(path)
            .expect("binary fixture metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("binary fixture should be executable");
    }
}

pub(crate) fn extract_json_string(record: &str, key: &str) -> String {
    let marker = format!("\"{key}\":\"");
    let start = record.find(&marker).expect("key should exist") + marker.len();
    let tail = &record[start..];
    let end = tail.find('"').expect("value should end");
    tail[..end].to_string()
}

pub(crate) fn ledger_stages(ledger: &str) -> Vec<String> {
    ledger
        .lines()
        .map(|record| extract_json_string(record, "stage"))
        .collect()
}
