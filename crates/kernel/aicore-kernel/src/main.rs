use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    InstalledManifestRegistry, KERNEL_RUNTIME_BINARY_NAME, KernelHandlerRegistry,
    KernelInvocationEnvelope, KernelInvocationLedger, KernelInvocationRuntime, KernelPayload,
    RUNTIME_BINARY_CONTRACT_VERSION, RUNTIME_BINARY_PROTOCOL, RUNTIME_BINARY_PROTOCOL_VERSION,
    RUNTIME_BINARY_REQUEST_SCHEMA_VERSION, RUNTIME_BINARY_RESPONSE_SCHEMA_VERSION,
    RuntimeStatusSnapshot, kernel_invocation_result_public_json,
    runtime_status_handler_for_layout_with_invocation_path,
};

fn main() {
    std::process::exit(run(std::env::args().skip(1).collect()));
}

fn run(args: Vec<String>) -> i32 {
    match args.as_slice() {
        [] => print_status(),
        [arg] if arg == "--status" => print_status(),
        [arg] if arg == "--invoke-stdio-jsonl" => invoke_stdio_jsonl(),
        _ => {
            eprintln!("用法：aicore-kernel --status | --invoke-stdio-jsonl");
            1
        }
    }
}

fn print_status() -> i32 {
    let layout = AicoreLayout::from_system_home();
    let snapshot = RuntimeStatusSnapshot::load_with_invocation_path(&layout, "binary");
    println!("AICore Kernel Runtime");
    println!("status: ok");
    println!("global root: {}", snapshot.global_root);
    println!("protocol: {RUNTIME_BINARY_PROTOCOL}");
    println!("protocol version: {RUNTIME_BINARY_PROTOCOL_VERSION}");
    println!("contract version: {RUNTIME_BINARY_CONTRACT_VERSION}");
    println!(
        "foundation runtime binary: {}",
        if snapshot.foundation_runtime_binary_installed {
            "installed"
        } else {
            "missing"
        }
    );
    println!(
        "kernel runtime binary: {}",
        if snapshot.kernel_runtime_binary_installed {
            "installed"
        } else {
            "missing"
        }
    );
    println!(
        "foundation runtime health: {}",
        binary_health(Path::new(&snapshot.foundation_runtime_binary_path))
    );
    println!(
        "kernel runtime health: {}",
        binary_health(Path::new(&snapshot.kernel_runtime_binary_path))
    );
    0
}

fn invoke_stdio_jsonl() -> i32 {
    let layout = AicoreLayout::from_system_home();
    let mut stdin = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut stdin) {
        let envelope = KernelInvocationEnvelope::new(
            "global-main",
            "runtime.status",
            "runtime.status",
            KernelPayload::Empty,
        );
        let payload = failure_payload(
            &layout,
            &envelope,
            "kernel_runtime_ipc_read",
            &format!("读取 kernel runtime stdin 失败: {error}"),
        );
        println!("{}", runtime_binary_result_event(payload));
        eprintln!("读取 kernel runtime stdin 失败: {error}");
        return 1;
    }

    let request_line = stdin.lines().find(|line| !line.trim().is_empty());
    let request: serde_json::Value =
        match request_line.and_then(|line| serde_json::from_str(line).ok()) {
            Some(request) => request,
            None => {
                let envelope = KernelInvocationEnvelope::new(
                    "global-main",
                    "runtime.status",
                    "runtime.status",
                    KernelPayload::Empty,
                );
                let payload = failure_payload(
                    &layout,
                    &envelope,
                    "kernel_runtime_malformed_jsonl",
                    "kernel runtime request is not valid JSONL",
                );
                println!("{}", runtime_binary_result_event(payload));
                return 1;
            }
        };
    let operation = request
        .get("operation")
        .and_then(|value| value.as_str())
        .unwrap_or("runtime.status");
    let instance_id = request
        .get("instance_id")
        .and_then(|value| value.as_str())
        .unwrap_or("global-main");
    let capability = request
        .get("capability")
        .and_then(|value| value.as_str())
        .unwrap_or(operation);
    let invocation_id = request
        .get("invocation_id")
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned);

    let payload = request
        .get("payload")
        .and_then(|value| value.as_str())
        .and_then(|value| value.strip_prefix("json:"))
        .map(|value| KernelPayload::JsonSummary(value.to_string()))
        .unwrap_or(KernelPayload::Empty);

    let envelope = envelope(
        instance_id,
        capability,
        operation,
        invocation_id.clone(),
        payload,
    );

    if request
        .get("schema_version")
        .and_then(|value| value.as_str())
        != Some(RUNTIME_BINARY_REQUEST_SCHEMA_VERSION)
        || request.get("protocol").and_then(|value| value.as_str()) != Some(RUNTIME_BINARY_PROTOCOL)
        || request
            .get("protocol_version")
            .and_then(|value| value.as_str())
            != Some(RUNTIME_BINARY_PROTOCOL_VERSION)
    {
        let payload = failure_payload(
            &layout,
            &envelope,
            "kernel_runtime_protocol_version_mismatch",
            "kernel runtime request protocol version mismatch",
        );
        println!("{}", runtime_binary_result_event(payload));
        return 1;
    }

    if request
        .get("contract_version")
        .and_then(|value| value.as_str())
        != Some(RUNTIME_BINARY_CONTRACT_VERSION)
    {
        let payload = failure_payload(
            &layout,
            &envelope,
            "kernel_runtime_contract_version_mismatch",
            "kernel runtime request contract version mismatch",
        );
        println!("{}", runtime_binary_result_event(payload));
        return 1;
    }

    let foundation_binary = layout.bin_root.join("aicore-foundation");
    if !foundation_binary.exists() {
        let payload = failure_payload(
            &layout,
            &envelope,
            "foundation_runtime_binary_missing",
            "Foundation runtime binary missing",
        );
        println!("{}", runtime_binary_result_event(payload));
        return 1;
    }
    if !is_executable_file(&foundation_binary) {
        let payload = failure_payload(
            &layout,
            &envelope,
            "foundation_runtime_binary_not_executable",
            "Foundation runtime binary is not executable",
        );
        println!("{}", runtime_binary_result_event(payload));
        return 1;
    }

    let registry = InstalledManifestRegistry::load_from_dir(&layout.manifests_root)
        .unwrap_or_else(|_| InstalledManifestRegistry::from_manifests(Vec::new()));
    let handlers = KernelHandlerRegistry::new().with_handler(
        "runtime.status",
        runtime_status_handler_for_layout_with_invocation_path(layout.clone(), "binary"),
    );
    let runtime = KernelInvocationRuntime::new(registry, handlers);
    let ledger =
        KernelInvocationLedger::new(layout.kernel_state_root.join("invocation-ledger.jsonl"));
    let output = runtime.invoke_with_ledger(envelope, &ledger);
    let payload = kernel_invocation_result_public_json(&output);
    println!("{}", runtime_binary_result_event(payload));
    match output.status {
        aicore_kernel::KernelInvocationStatus::Completed => 0,
        aicore_kernel::KernelInvocationStatus::Failed => 1,
    }
}

fn envelope(
    instance_id: &str,
    capability: &str,
    operation: &str,
    invocation_id: Option<String>,
    payload: KernelPayload,
) -> KernelInvocationEnvelope {
    let envelope = KernelInvocationEnvelope::new(instance_id, capability, operation, payload);
    if let Some(invocation_id) = invocation_id {
        envelope.with_invocation_id(invocation_id)
    } else {
        envelope
    }
}

fn failure_payload(
    layout: &AicoreLayout,
    envelope: &KernelInvocationEnvelope,
    stage: &str,
    reason: &str,
) -> serde_json::Value {
    serde_json::json!({
        "invocation_id": envelope.invocation_id,
        "trace_id": envelope.trace_context.trace_id,
        "operation": envelope.operation,
        "status": "failed",
        "route": serde_json::Value::Null,
        "handler": {
            "kind": KERNEL_RUNTIME_BINARY_NAME,
            "invocation_mode": "local_process",
            "transport": "stdio_jsonl",
            "process_exit_code": serde_json::Value::Null,
            "executed": false,
            "event_generated": false,
            "spawned_process": true,
            "called_real_component": false,
            "first_party_in_process_adapter": false,
        },
        "ledger": {
            "appended": false,
            "path": layout.kernel_state_root.join("invocation-ledger.jsonl").display().to_string(),
            "records": 0,
        },
        "result": {
            "kind": serde_json::Value::Null,
            "summary": serde_json::Value::Null,
            "fields": {},
        },
        "failure": {
            "stage": stage,
            "reason": reason,
        },
        "runtime_binary": {
            "foundation_path": layout.bin_root.join("aicore-foundation").display().to_string(),
            "foundation_installed": layout.bin_root.join("aicore-foundation").exists(),
            "foundation_health": binary_health(&layout.bin_root.join("aicore-foundation")),
            "kernel_path": layout.bin_root.join("aicore-kernel").display().to_string(),
            "kernel_installed": layout.bin_root.join("aicore-kernel").exists(),
            "kernel_health": binary_health(&layout.bin_root.join("aicore-kernel")),
            "protocol": RUNTIME_BINARY_PROTOCOL,
            "protocol_version": RUNTIME_BINARY_PROTOCOL_VERSION,
            "contract_version": RUNTIME_BINARY_CONTRACT_VERSION,
            "in_process_fallback": false,
        }
    })
}

fn runtime_binary_result_event(payload: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "event": "kernel.invocation.result",
        "schema_version": RUNTIME_BINARY_RESPONSE_SCHEMA_VERSION,
        "protocol": RUNTIME_BINARY_PROTOCOL,
        "protocol_version": RUNTIME_BINARY_PROTOCOL_VERSION,
        "contract_version": RUNTIME_BINARY_CONTRACT_VERSION,
        "payload": payload,
    })
}

fn binary_health(path: &Path) -> &'static str {
    if !path.exists() {
        "missing"
    } else if is_executable_file(path) {
        "ok"
    } else {
        "not_executable"
    }
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        return std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}
