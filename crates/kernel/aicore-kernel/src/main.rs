use std::io::Read;

use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    InstalledManifestRegistry, KERNEL_RUNTIME_BINARY_NAME, KernelHandlerRegistry,
    KernelInvocationEnvelope, KernelInvocationLedger, KernelInvocationRuntime, KernelPayload,
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
    println!("protocol: stdio_jsonl");
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
        println!(
            "{}",
            serde_json::json!({"event": "kernel.invocation.result", "payload": payload})
        );
        eprintln!("读取 kernel runtime stdin 失败: {error}");
        return 1;
    }

    let request: serde_json::Value = stdin
        .lines()
        .find(|line| !line.trim().is_empty())
        .and_then(|line| serde_json::from_str(line).ok())
        .unwrap_or_else(|| serde_json::json!({}));
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

    if !layout.bin_root.join("aicore-foundation").exists() {
        let envelope = envelope(
            instance_id,
            capability,
            operation,
            invocation_id,
            KernelPayload::Empty,
        );
        let payload = failure_payload(
            &layout,
            &envelope,
            "foundation_runtime_binary_missing",
            "Foundation runtime binary missing",
        );
        println!(
            "{}",
            serde_json::json!({"event": "kernel.invocation.result", "payload": payload})
        );
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
    let output = runtime.invoke_with_ledger(
        envelope(
            instance_id,
            capability,
            operation,
            invocation_id,
            KernelPayload::Empty,
        ),
        &ledger,
    );
    let payload = kernel_invocation_result_public_json(&output);
    println!(
        "{}",
        serde_json::json!({"event": "kernel.invocation.result", "payload": payload})
    );
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
        }
    })
}
