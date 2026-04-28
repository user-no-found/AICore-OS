use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    KernelHandlerError, KernelHandlerRegistry, KernelHandlerResult, KernelInvocationEnvelope,
    KernelInvocationLedger, KernelInvocationRuntime, KernelInvocationStatus, KernelPayload,
    KernelRouteRuntimeOutput, KernelRuntimeBinaryClient, format_contract,
};
use aicore_terminal::{TerminalConfig, TerminalMode};

use crate::terminal::{cli_row, emit_cli_panel};

use super::payload::{
    emit_kernel_invocation_payload_json, event_payload_summary, kernel_invocation_payload_rows,
    payload_status,
};

pub(crate) fn print_kernel_invoke_smoke(operation: &str) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
    let registry =
        match aicore_kernel::InstalledManifestRegistry::load_from_dir(&layout.manifests_root) {
            Ok(registry) => registry,
            Err(error) => {
                emit_cli_panel(
                    "内核调用失败",
                    vec![
                        cli_row("invocation", "failed"),
                        cli_row("route", "failed"),
                        cli_row("reason", "manifest registry load failed"),
                        cli_row("operation", operation),
                        cli_row("detail", error),
                        cli_row("handler executed", "false"),
                        cli_row("event generated", "false"),
                        cli_row("ledger appended", "false"),
                        cli_row("ledger path", ledger_path.display().to_string()),
                        cli_row("ledger records", "0"),
                    ],
                );
                return 1;
            }
        };
    let handlers = KernelHandlerRegistry::new().with_handler("memory.search", kernel_smoke_handler);
    let runtime = KernelInvocationRuntime::new(registry, handlers);
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let output = runtime.invoke_with_ledger(
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty),
        &ledger,
    );

    if output.status == KernelInvocationStatus::Completed {
        let route = output
            .route
            .as_ref()
            .expect("completed invocation must route");
        let event = output
            .event
            .as_ref()
            .expect("completed invocation must emit event");
        emit_cli_panel(
            "内核调用 Smoke",
            vec![
                cli_row("invocation", "completed"),
                cli_row("route", "routed"),
                cli_row("operation", operation),
                cli_row("component", route.component_id.as_str()),
                cli_row("app", route.app_id.as_str()),
                cli_row("capability", route.capability_id.as_str()),
                cli_row("contract", format_contract(&route.contract_version)),
                cli_row(
                    "handler kind",
                    output.handler_kind.as_deref().unwrap_or("-"),
                ),
                cli_row("handler executed", output.handler_executed.to_string()),
                cli_row("event generated", output.event_generated.to_string()),
                cli_row("event type", format!("{:?}", event.event_type)),
                cli_row("event payload", event_payload_summary(&event.payload)),
                cli_row("ledger appended", output.ledger_appended.to_string()),
                cli_row(
                    "ledger path",
                    output.ledger_path.as_deref().unwrap_or("-").to_string(),
                ),
                cli_row("ledger records", output.ledger_record_count.to_string()),
                cli_row("spawned process", output.spawned_process.to_string()),
                cli_row(
                    "called real component",
                    output.called_real_component.to_string(),
                ),
                cli_row("说明", "只执行 in-process smoke handler，不启动组件进程"),
            ],
        );
        return 0;
    }

    let route_status = if output.route_decision_made {
        "routed"
    } else {
        "failed"
    };
    let mut rows = vec![
        cli_row("invocation", "failed"),
        cli_row("route", route_status),
        cli_row("operation", operation),
        cli_row(
            "failure stage",
            output.failure_stage.as_deref().unwrap_or("-"),
        ),
        cli_row(
            "reason",
            output
                .failure_reason
                .as_deref()
                .unwrap_or("unknown failure"),
        ),
        cli_row("handler executed", output.handler_executed.to_string()),
        cli_row("event generated", output.event_generated.to_string()),
        cli_row("ledger appended", output.ledger_appended.to_string()),
        cli_row(
            "ledger path",
            output.ledger_path.as_deref().unwrap_or("-").to_string(),
        ),
        cli_row("ledger records", output.ledger_record_count.to_string()),
    ];
    if let Some(route) = output.route.as_ref() {
        rows.push(cli_row("component", route.component_id.as_str()));
        rows.push(cli_row("capability", route.capability_id.as_str()));
    }
    emit_cli_panel("内核调用失败", rows);
    1
}

pub(crate) fn print_kernel_invoke_readonly(operation: &str, args: &[String]) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let payload = readonly_payload(operation, args);
    let invocation =
        KernelRuntimeBinaryClient::new(layout).invoke_readonly_with_payload(operation, payload);
    let output = invocation.payload;
    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_kernel_invocation_payload_json(&output);
        return if invocation.exit_success { 0 } else { 1 };
    }

    let title = if payload_status(&output) == Some("completed") {
        "内核只读调用"
    } else {
        "内核只读调用失败"
    };
    emit_cli_panel(title, kernel_invocation_payload_rows(&output, operation));
    if invocation.exit_success { 0 } else { 1 }
}

fn kernel_smoke_handler(
    envelope: &KernelInvocationEnvelope,
    _route: &KernelRouteRuntimeOutput,
) -> Result<KernelHandlerResult, KernelHandlerError> {
    Ok(KernelHandlerResult::summary(format!(
        "smoke handled {}",
        envelope.operation
    )))
}

fn readonly_payload(operation: &str, args: &[String]) -> KernelPayload {
    match operation {
        "agent.smoke" => {
            let content = args
                .first()
                .map(String::as_str)
                .unwrap_or("agent smoke demo input");
            KernelPayload::JsonSummary(serde_json::json!({ "content": content }).to_string())
        }
        "agent.session_smoke" => {
            let first = args
                .first()
                .map(String::as_str)
                .unwrap_or("first demo input");
            let second = args
                .get(1)
                .map(String::as_str)
                .unwrap_or("second demo input");
            KernelPayload::JsonSummary(
                serde_json::json!({ "first": first, "second": second }).to_string(),
            )
        }
        "memory.search" => memory_search_payload(args),
        "memory.wiki_page" => {
            let page = args.first().map(String::as_str).unwrap_or("index");
            KernelPayload::JsonSummary(serde_json::json!({ "page": page }).to_string())
        }
        _ => KernelPayload::Empty,
    }
}

fn memory_search_payload(args: &[String]) -> KernelPayload {
    let query = args.first().map(String::as_str).unwrap_or("");
    let mut payload = serde_json::json!({ "query": query });
    let object = payload
        .as_object_mut()
        .expect("memory search payload should be an object");
    let mut index = 1usize;
    while index < args.len() {
        let Some(value) = args.get(index + 1) else {
            object.insert(
                "invalid_filter".to_string(),
                serde_json::Value::String(args[index].clone()),
            );
            break;
        };
        match args[index].as_str() {
            "--type" => {
                object.insert("type".to_string(), serde_json::Value::String(value.clone()));
            }
            "--source" => {
                object.insert(
                    "source".to_string(),
                    serde_json::Value::String(value.clone()),
                );
            }
            "--permanence" => {
                object.insert(
                    "permanence".to_string(),
                    serde_json::Value::String(value.clone()),
                );
            }
            "--limit" => {
                object.insert(
                    "limit".to_string(),
                    serde_json::Value::String(value.clone()),
                );
            }
            _ => {
                object.insert(
                    "invalid_filter".to_string(),
                    serde_json::Value::String(args[index].clone()),
                );
            }
        }
        index += 2;
    }
    KernelPayload::JsonSummary(payload.to_string())
}
