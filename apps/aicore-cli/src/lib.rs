use std::{env, fs, io::Read, path::PathBuf};

use aicore_agent::{
    AgentSessionRunner, AgentSessionStopReason, AgentTurnInput, AgentTurnOutcome, AgentTurnRunner,
};
use aicore_auth::{AuthCapability, AuthEntry, AuthKind, AuthRef, GlobalAuthPool, SecretRef};
use aicore_config::{
    ConfigPaths, ConfigStore, GlobalServiceProfiles, InstanceRuntimeConfig, ModelBinding,
    ServiceProfile, ServiceProfileMode, ServiceRole,
};
use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    DeliveryIdentity, GatewaySource, InterruptMode, OutputTarget, TransportEnvelope,
    default_runtime,
};
use aicore_kernel::{
    KernelEventPayload, KernelHandlerError, KernelHandlerRegistry, KernelHandlerResult,
    KernelInvocationEnvelope, KernelInvocationLedger, KernelInvocationRuntime,
    KernelInvocationStatus, KernelPayload, KernelRouteRuntime, KernelRouteRuntimeError,
    KernelRouteRuntimeInput, KernelRouteRuntimeOutput, default_control_plane, format_contract,
    runtime_status_handler_for_layout,
};
use aicore_memory::{
    MemoryAuditReport, MemoryKernel, MemoryPaths, MemoryPermanence, MemoryScope, MemorySource,
    MemoryType, RememberInput, SearchQuery,
};
use aicore_provider::{
    ModelRequest, PromptBuildInput, PromptBuilder, ProviderError, ProviderInvoker, ProviderResolver,
};
use aicore_terminal::{Block, Document, TerminalConfig, TerminalMode, render_document};

pub fn run_from_args(args: Vec<String>) -> i32 {
    match args.as_slice() {
        [cmd] if cmd == "__component-smoke-stdio" => run_component_smoke_stdio(),
        [cmd] if cmd == "status" => {
            print_status();
            0
        }
        [group, action] if group == "instance" && action == "list" => {
            print_instance_list();
            0
        }
        [group, action] if group == "runtime" && action == "smoke" => {
            print_runtime_smoke();
            0
        }
        [group, action, operation] if group == "kernel" && action == "route" => {
            print_kernel_route(operation)
        }
        [group, action, operation] if group == "kernel" && action == "invoke-smoke" => {
            print_kernel_invoke_smoke(operation)
        }
        [group, action, operation] if group == "kernel" && action == "invoke-readonly" => {
            print_kernel_invoke_readonly(operation)
        }
        [group, action, operation] if group == "kernel" && action == "invoke-process-smoke" => {
            print_kernel_invoke_process_smoke(operation)
        }
        [group, action] if group == "config" && action == "smoke" => {
            run_config_command(print_config_smoke)
        }
        [group, action] if group == "config" && action == "path" => {
            run_config_command(print_config_path)
        }
        [group, action] if group == "config" && action == "init" => {
            run_config_command(print_config_init)
        }
        [group, action] if group == "config" && action == "validate" => {
            run_config_command(print_config_validate)
        }
        [group, action] if group == "auth" && action == "list" => {
            run_config_command(print_auth_list)
        }
        [group, action] if group == "model" && action == "show" => {
            run_config_command(print_model_show)
        }
        [group, action] if group == "service" && action == "list" => {
            run_config_command(print_service_list)
        }
        [group, action] if group == "provider" && action == "smoke" => {
            run_config_command(print_provider_smoke)
        }
        [group, action, content] if group == "agent" && action == "smoke" => {
            run_config_command_with_arg(content, print_agent_smoke)
        }
        [group, action, first, second] if group == "agent" && action == "session-smoke" => {
            run_config_command_with_two_args(first, second, print_agent_session_smoke)
        }
        [group, action] if group == "memory" && action == "status" => {
            run_memory_command(print_memory_status)
        }
        [group, action] if group == "memory" && action == "audit" => {
            run_memory_command(print_memory_audit)
        }
        [group, action] if group == "memory" && action == "proposals" => {
            run_memory_command(print_memory_proposals)
        }
        [group, action] if group == "memory" && action == "wiki" => {
            run_memory_command(print_memory_wiki_index)
        }
        [group, action, content] if group == "memory" && action == "remember" => {
            run_memory_command_with_arg(content, print_memory_remember)
        }
        [group, action, page] if group == "memory" && action == "wiki" => {
            run_memory_command_with_arg(page, print_memory_wiki_page)
        }
        [group, action, query, rest @ ..] if group == "memory" && action == "search" => {
            run_memory_search_command(query, rest)
        }
        [group, action, proposal_id] if group == "memory" && action == "accept" => {
            run_memory_command_with_arg(proposal_id, print_memory_accept)
        }
        [group, action, proposal_id] if group == "memory" && action == "reject" => {
            run_memory_command_with_arg(proposal_id, print_memory_reject)
        }
        [group, _] if group == "config" => {
            eprintln!("未知 config 命令。");
            eprintln!("可用命令：config smoke | config path | config init | config validate");
            1
        }
        [group, _] if group == "memory" => {
            eprintln!("未知 memory 命令。");
            eprintln!(
                "可用命令：memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
        [group, _] if group == "agent" => {
            eprintln!("未知 agent 命令。");
            eprintln!(
                "可用命令：agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容>"
            );
            1
        }
        [group, _] if group == "kernel" => {
            eprintln!("未知 kernel 命令。");
            eprintln!(
                "可用命令：kernel route <operation> | kernel invoke-smoke <operation> | kernel invoke-readonly <operation> | kernel invoke-process-smoke <operation>"
            );
            1
        }
        _ => {
            eprintln!("未知命令。");
            eprintln!(
                "可用命令：status | instance list | runtime smoke | kernel route <operation> | kernel invoke-smoke <operation> | kernel invoke-readonly <operation> | kernel invoke-process-smoke <operation> | config smoke | config path | config init | config validate | auth list | model show | service list | provider smoke | agent smoke <内容> | agent session-smoke <第一轮内容> <第二轮内容> | memory status | memory audit | memory proposals | memory wiki [page] | memory remember <内容> | memory search <关键词> | memory accept <proposal_id> | memory reject <proposal_id>"
            );
            1
        }
    }
}

fn print_status() {
    let control_plane = default_control_plane();
    let runtime = default_runtime();
    let control_summary = control_plane.summary();
    let main_instance = control_plane.main_instance_summary();
    let runtime_summary = runtime.summary();

    emit_cli_panel(
        "AICore CLI",
        vec![
            cli_row("主实例", main_instance.id.as_str()),
            cli_row("组件数量", control_summary.component_count.to_string()),
            cli_row("实例数量", control_summary.instance_count.to_string()),
            cli_row(
                "Runtime",
                format!(
                    "{}/{}",
                    runtime_summary.instance_id, runtime_summary.conversation_id
                ),
            ),
        ],
    );
}

fn emit_cli_panel(title: &str, rows: Vec<(String, String)>) {
    let body = rows
        .into_iter()
        .map(|(key, value)| format!("{key}：{value}"))
        .collect::<Vec<_>>()
        .join("\n");
    emit_cli_panel_body(title, &body);
}

fn emit_cli_panel_body(title: &str, body: &str) {
    emit_document(Document::new(vec![Block::panel(title, body)]));
}

fn emit_document(document: Document) {
    print!("{}", render_document(&document, &TerminalConfig::current()));
}

fn cli_row(key: impl Into<String>, value: impl Into<String>) -> (String, String) {
    (key.into(), value.into())
}

fn print_instance_list() {
    let control_plane = default_control_plane();
    let mut lines = Vec::new();

    for instance in control_plane.instance_registry().list() {
        let kind = match instance.kind {
            aicore_kernel::InstanceKind::GlobalMain => "global_main",
            aicore_kernel::InstanceKind::Workspace => "workspace",
        };

        lines.push(format!(
            "- {} [{}] {}",
            instance.id.as_str(),
            kind,
            instance.workspace_root.display()
        ));
    }

    emit_cli_panel_body("实例列表：", &lines.join("\n"));
}

fn print_kernel_route(operation: &str) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let registry =
        match aicore_kernel::InstalledManifestRegistry::load_from_dir(&layout.manifests_root) {
            Ok(registry) => registry,
            Err(error) => {
                emit_cli_panel(
                    "内核路由失败",
                    vec![
                        cli_row("decision", "route failed"),
                        cli_row("reason", "manifest registry load failed"),
                        cli_row("operation", operation),
                        cli_row("detail", error),
                        cli_row("handler executed", "false"),
                    ],
                );
                return 1;
            }
        };
    let runtime = KernelRouteRuntime::from_registry(registry);

    match runtime.route(KernelRouteRuntimeInput::new(operation)) {
        Ok(output) => {
            emit_cli_panel(
                "内核路由决策",
                vec![
                    cli_row("decision", "routed"),
                    cli_row("operation", output.operation.as_str()),
                    cli_row("component", output.component_id.as_str()),
                    cli_row("app", output.app_id.as_str()),
                    cli_row("capability", output.capability_id.as_str()),
                    cli_row("contract", format_contract(&output.contract_version)),
                    cli_row("visibility", output.visibility.as_str()),
                    cli_row("entrypoint", output.entrypoint.as_str()),
                    cli_row("invocation mode", output.invocation_mode.as_str()),
                    cli_row("transport", output.transport.as_str()),
                    cli_row(
                        "route reason",
                        format!("{:?}", output.decision.route_reason),
                    ),
                    cli_row("trace id", output.decision.request.trace_context.trace_id),
                    cli_row("handler executed", output.handler_executed.to_string()),
                    cli_row("说明", "只生成 route decision，不会执行 handler"),
                ],
            );
            0
        }
        Err(error) => {
            emit_kernel_route_error(operation, error);
            1
        }
    }
}

fn print_kernel_invoke_smoke(operation: &str) -> i32 {
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

fn print_kernel_invoke_readonly(operation: &str) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
    let registry =
        match aicore_kernel::InstalledManifestRegistry::load_from_dir(&layout.manifests_root) {
            Ok(registry) => registry,
            Err(error) => {
                emit_cli_panel(
                    "内核只读调用失败",
                    vec![
                        cli_row("invocation", "failed"),
                        cli_row("route", "failed"),
                        cli_row("reason", "manifest registry load failed"),
                        cli_row("operation", operation),
                        cli_row("detail", error),
                        cli_row("handler executed", "false"),
                        cli_row("event generated", "false"),
                        cli_row("first-party in-process adapter", "false"),
                        cli_row("ledger appended", "false"),
                        cli_row("ledger path", ledger_path.display().to_string()),
                        cli_row("ledger records", "0"),
                    ],
                );
                return 1;
            }
        };
    let handlers = KernelHandlerRegistry::new().with_handler(
        "runtime.status",
        runtime_status_handler_for_layout(layout.clone()),
    );
    let runtime = KernelInvocationRuntime::new(registry, handlers);
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let envelope =
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty);
    let invocation_id = envelope.invocation_id.clone();
    let output = runtime.invoke_with_ledger(envelope, &ledger);

    if output.status == KernelInvocationStatus::Completed {
        let route = output
            .route
            .as_ref()
            .expect("completed invocation must route");
        let result = output
            .result
            .as_ref()
            .expect("completed invocation must include result envelope");
        if TerminalConfig::current().mode == TerminalMode::Json {
            emit_kernel_invocation_result_json(&output);
            return 0;
        }
        emit_cli_panel(
            "内核只读调用",
            vec![
                cli_row("invocation", "completed"),
                cli_row("invocation id", result.invocation_id.as_str()),
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
                cli_row("result kind", result.result_kind.as_deref().unwrap_or("-")),
                cli_row("result summary", result.summary.as_str()),
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
                cli_row("first-party in-process adapter", "true"),
                cli_row(
                    "说明",
                    "通过 first-party in-process read-only adapter 执行，不启动组件进程",
                ),
            ],
        );
        return 0;
    }

    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_kernel_invocation_result_json(&output);
        return 1;
    }

    let route_status = if output.route_decision_made {
        "routed"
    } else {
        "failed"
    };
    let mut rows = vec![
        cli_row("invocation", "failed"),
        cli_row("invocation id", invocation_id),
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
        cli_row("first-party in-process adapter", "false"),
        cli_row("ledger appended", output.ledger_appended.to_string()),
        cli_row(
            "ledger path",
            output.ledger_path.as_deref().unwrap_or("-").to_string(),
        ),
        cli_row("ledger records", output.ledger_record_count.to_string()),
    ];
    if let Some(route) = output.route.as_ref() {
        rows.push(cli_row("component", route.component_id.as_str()));
        rows.push(cli_row("app", route.app_id.as_str()));
        rows.push(cli_row("capability", route.capability_id.as_str()));
    }
    emit_cli_panel("内核只读调用失败", rows);
    1
}

fn print_kernel_invoke_process_smoke(operation: &str) -> i32 {
    let layout = AicoreLayout::from_system_home();
    let ledger_path = layout.kernel_state_root.join("invocation-ledger.jsonl");
    let registry =
        match aicore_kernel::InstalledManifestRegistry::load_from_dir(&layout.manifests_root) {
            Ok(registry) => registry,
            Err(error) => {
                emit_cli_panel(
                    "内核组件进程调用失败",
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
    let runtime = KernelInvocationRuntime::new(registry, KernelHandlerRegistry::new());
    let ledger = KernelInvocationLedger::new(&ledger_path);
    let envelope =
        KernelInvocationEnvelope::new("global-main", operation, operation, KernelPayload::Empty);
    let invocation_id = envelope.invocation_id.clone();
    let output = runtime.invoke_with_ledger(envelope, &ledger);

    if TerminalConfig::current().mode == TerminalMode::Json {
        emit_kernel_invocation_result_json(&output);
        return if output.status == KernelInvocationStatus::Completed {
            0
        } else {
            1
        };
    }

    if output.status == KernelInvocationStatus::Completed {
        let route = output
            .route
            .as_ref()
            .expect("completed process invocation must route");
        let result = output
            .result
            .as_ref()
            .expect("completed process invocation must include result envelope");
        let mut rows = vec![
            cli_row("invocation", "completed"),
            cli_row("invocation id", result.invocation_id.as_str()),
            cli_row("route", "routed"),
            cli_row("operation", operation),
            cli_row("component", route.component_id.as_str()),
            cli_row("app", route.app_id.as_str()),
            cli_row("capability", route.capability_id.as_str()),
            cli_row("contract", format_contract(&route.contract_version)),
            cli_row("invocation mode", route.invocation_mode.as_str()),
            cli_row("transport", route.transport.as_str()),
            cli_row(
                "handler kind",
                output.handler_kind.as_deref().unwrap_or("-"),
            ),
            cli_row("handler executed", output.handler_executed.to_string()),
            cli_row("spawned process", output.spawned_process.to_string()),
            cli_row(
                "called real component",
                output.called_real_component.to_string(),
            ),
            cli_row("event generated", output.event_generated.to_string()),
            cli_row("result kind", result.result_kind.as_deref().unwrap_or("-")),
            cli_row("result summary", result.summary.as_str()),
            cli_row("ledger appended", output.ledger_appended.to_string()),
            cli_row(
                "ledger path",
                output.ledger_path.as_deref().unwrap_or("-").to_string(),
            ),
            cli_row("ledger records", output.ledger_record_count.to_string()),
            cli_row(
                "说明",
                "只验证 local process boundary，不代表业务组件已迁移",
            ),
        ];
        for (key, value) in &result.public_fields {
            rows.push(cli_row(format!("result.{key}"), value.as_str()));
        }
        emit_cli_panel("内核组件进程调用 Smoke", rows);
        return 0;
    }

    let route_status = if output.route_decision_made {
        "routed"
    } else {
        "failed"
    };
    let mut rows = vec![
        cli_row("invocation", "failed"),
        cli_row("invocation id", invocation_id),
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
        cli_row(
            "handler kind",
            output.handler_kind.as_deref().unwrap_or("-"),
        ),
        cli_row(
            "transport",
            output.transport.as_deref().unwrap_or("-").to_string(),
        ),
        cli_row("handler executed", output.handler_executed.to_string()),
        cli_row("spawned process", output.spawned_process.to_string()),
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
        rows.push(cli_row("app", route.app_id.as_str()));
        rows.push(cli_row("capability", route.capability_id.as_str()));
        rows.push(cli_row("invocation mode", route.invocation_mode.as_str()));
    }
    emit_cli_panel("内核组件进程调用失败", rows);
    1
}

fn run_component_smoke_stdio() -> i32 {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("component smoke stdin 读取失败: {error}");
        return 1;
    }
    let line = input
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("{}");
    let value: serde_json::Value =
        serde_json::from_str(line).unwrap_or_else(|_| serde_json::json!({}));
    let operation = value
        .get("operation")
        .and_then(|value| value.as_str())
        .unwrap_or("component.process.smoke");
    let invocation_id = value
        .get("invocation_id")
        .and_then(|value| value.as_str())
        .unwrap_or("-");
    let result = serde_json::json!({
        "result_kind": "component.process.smoke",
        "summary": format!("process smoke handled {operation}"),
        "fields": {
            "operation": operation,
            "ipc": "stdio_jsonl",
            "invocation_id": invocation_id,
            "component_process": "ok"
        }
    });
    println!(
        "{}",
        serde_json::to_string(&result).expect("component smoke result should encode")
    );
    0
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

fn emit_kernel_invocation_result_json(output: &aicore_kernel::KernelInvocationRuntimeOutput) {
    let payload = kernel_invocation_result_json(output);
    let payload = serde_json::to_string(&payload).expect("kernel invocation result should encode");
    emit_document(Document::new(vec![Block::structured_json(
        "kernel.invocation.result",
        &payload,
    )]));
}

fn kernel_invocation_result_json(
    output: &aicore_kernel::KernelInvocationRuntimeOutput,
) -> serde_json::Value {
    let result = output.result.as_ref();
    let route = result
        .and_then(|result| result.route.as_ref())
        .map(|route| {
            serde_json::json!({
                "component_id": route.component_id,
                "app_id": route.app_id,
                "capability_id": route.capability_id,
                "contract_version": route.contract_version,
            })
        })
        .unwrap_or(serde_json::Value::Null);
    let fields = result
        .map(|result| serde_json::json!(result.public_fields))
        .unwrap_or_else(|| serde_json::json!({}));

    serde_json::json!({
        "invocation_id": result.map(|result| result.invocation_id.as_str()),
        "trace_id": result.map(|result| result.trace_id.as_str()),
        "operation": result
            .map(|result| result.operation.as_str())
            .or_else(|| output.route.as_ref().map(|route| route.operation.as_str())),
        "status": match output.status {
            KernelInvocationStatus::Completed => "completed",
            KernelInvocationStatus::Failed => "failed",
        },
        "route": route,
        "handler": {
            "kind": output.handler_kind.as_deref(),
            "invocation_mode": output.route.as_ref().map(|route| route.invocation_mode.as_str()),
            "transport": output.transport.as_deref(),
            "process_exit_code": output.process_exit_code,
            "executed": output.handler_executed,
            "event_generated": output.event_generated,
            "spawned_process": output.spawned_process,
            "called_real_component": output.called_real_component,
            "first_party_in_process_adapter": result
                .and_then(|result| result.result_kind.as_deref())
                == Some("runtime.status"),
        },
        "ledger": {
            "appended": output.ledger_appended,
            "path": output.ledger_path.as_deref(),
            "records": output.ledger_record_count,
        },
        "result": {
            "kind": result.and_then(|result| result.result_kind.as_deref()),
            "summary": result.map(|result| result.summary.as_str()),
            "fields": fields,
        },
        "failure": {
            "stage": output.failure_stage.as_deref(),
            "reason": output.failure_reason.as_deref(),
        }
    })
}

fn event_payload_summary(payload: &KernelEventPayload) -> String {
    match payload {
        KernelEventPayload::Summary(summary) => summary.clone(),
        KernelEventPayload::Empty => "empty".to_string(),
    }
}

fn emit_kernel_route_error(operation: &str, error: KernelRouteRuntimeError) {
    let mut rows = vec![
        cli_row("decision", "route failed"),
        cli_row("reason", error.code()),
        cli_row("operation", operation),
        cli_row("detail", error.to_string()),
        cli_row("handler executed", "false"),
    ];

    if let KernelRouteRuntimeError::AmbiguousRoute { candidates, .. } = &error {
        rows.push(cli_row("candidates", candidates.join(", ")));
    }

    emit_cli_panel("内核路由失败", rows);
}

fn print_runtime_smoke() {
    let mut cli_runtime = default_runtime();
    let cli_ingress = cli_runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::Cli,
            platform: None,
            target_id: None,
            sender_id: None,
            is_group: false,
            mentioned_bot: false,
        },
        "hello",
        InterruptMode::Queue,
    );
    let cli_routed = cli_runtime.append_assistant_output("reply");
    let cli_first = cli_routed
        .events
        .first()
        .expect("runtime smoke must have at least one output");

    let mut external_runtime = default_runtime();
    external_runtime.handle_ingress(
        TransportEnvelope {
            source: GatewaySource::External,
            platform: Some("feishu".to_string()),
            target_id: Some("chat-1".to_string()),
            sender_id: Some("user-1".to_string()),
            is_group: true,
            mentioned_bot: true,
        },
        "hello from external",
        InterruptMode::Queue,
    );
    let external_routed = external_runtime.append_assistant_output("reply external");
    let external_origin = external_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::Origin)
        .expect("external origin smoke must include origin output");

    let mut follow_runtime = default_runtime();
    follow_runtime.follow_external(TransportEnvelope {
        source: GatewaySource::External,
        platform: Some("feishu".to_string()),
        target_id: Some("chat-2".to_string()),
        sender_id: Some("user-2".to_string()),
        is_group: true,
        mentioned_bot: true,
    });
    let follow_routed = follow_runtime.append_assistant_output("reply followed");
    let followed_external = follow_routed
        .events
        .iter()
        .find(|event| event.target == OutputTarget::FollowedExternal)
        .expect("follow smoke must include followed external output");

    let body = vec![
        "CLI 场景：".to_string(),
        format!("  接收决策：{:?}", cli_ingress.decision),
        format!("  账本消息数：{}", cli_runtime.summary().event_count),
        format!("  输出目标：{}", output_target_name(&cli_first.target)),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&cli_first.identity)
        ),
        "External Origin 场景：".to_string(),
        format!(
            "  输出目标：{}",
            output_target_name(&external_origin.target)
        ),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&external_origin.identity)
        ),
        "Follow 场景：".to_string(),
        format!(
            "  输出目标：{}",
            output_target_name(&followed_external.target)
        ),
        format!(
            "  投递身份：{}",
            delivery_identity_name(&followed_external.identity)
        ),
    ];

    emit_cli_panel_body("Runtime Smoke：", &body.join("\n"));
}

fn run_config_command(command: fn() -> Result<(), String>) -> i32 {
    match command() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_config_command_with_arg(arg: &str, command: fn(&str) -> Result<(), String>) -> i32 {
    match command(arg) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_config_command_with_two_args(
    first: &str,
    second: &str,
    command: fn(&str, &str) -> Result<(), String>,
) -> i32 {
    match command(first, second) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("配置命令失败：{error}");
            1
        }
    }
}

fn run_memory_command(command: fn() -> Result<(), String>) -> i32 {
    match command() {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn run_memory_command_with_arg(arg: &str, command: fn(&str) -> Result<(), String>) -> i32 {
    match command(arg) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn run_memory_search_command(query: &str, args: &[String]) -> i32 {
    match parse_memory_search_options(args).and_then(|options| print_memory_search(query, options))
    {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("记忆命令失败：{error}");
            1
        }
    }
}

fn print_config_smoke() -> Result<(), String> {
    let store = prepare_demo_config_store("config-smoke")?;

    let auth_toml_exists = store.paths.auth_toml.exists();
    let services_toml_exists = store.paths.services_toml.exists();
    let runtime_toml_exists = store.paths.runtime_toml_for("global-main").exists();

    let loaded_auth_pool = store.load_auth_pool().map_err(config_error)?;
    let loaded_runtime = store
        .load_instance_runtime("global-main")
        .map_err(config_error)?;
    let loaded_services = store.load_services().map_err(config_error)?;

    if !(auth_toml_exists && services_toml_exists && runtime_toml_exists) {
        return Err("默认配置文件未完整创建".to_string());
    }

    if loaded_auth_pool != demo_auth_pool() {
        return Err("认证池读取结果与写入内容不一致".to_string());
    }

    if loaded_runtime != demo_runtime_config() {
        return Err("实例运行配置读取结果与写入内容不一致".to_string());
    }

    if loaded_services != demo_service_profiles() {
        return Err("服务角色配置读取结果与写入内容不一致".to_string());
    }

    ConfigStore::validate_runtime_config(&loaded_runtime, &loaded_auth_pool)
        .map_err(config_error)?;
    ConfigStore::validate_service_profiles(&loaded_services, &loaded_auth_pool)
        .map_err(config_error)?;

    emit_cli_panel_body(
        "配置 Smoke Test：",
        "- 默认配置文件：通过\n- 认证池保存/读取：通过\n- 实例运行配置保存/读取：通过\n- 服务角色配置保存/读取：通过\n- 配置校验：通过",
    );

    Ok(())
}

fn print_config_path() -> Result<(), String> {
    let paths = real_config_paths()?;

    emit_cli_panel(
        "配置路径",
        vec![
            cli_row("root", paths.root.display().to_string()),
            cli_row("auth.toml", paths.auth_toml.display().to_string()),
            cli_row("services.toml", paths.services_toml.display().to_string()),
            cli_row("instances", paths.instances_dir.display().to_string()),
            cli_row(
                "global-main runtime",
                paths.runtime_toml_for("global-main").display().to_string(),
            ),
        ],
    );

    Ok(())
}

fn print_config_init() -> Result<(), String> {
    let store = real_config_store()?;
    let status = initialize_real_config(&store)?;

    emit_cli_panel(
        "配置初始化",
        vec![
            cli_row("root", store.paths.root.display().to_string()),
            cli_row("auth.toml", init_status_name(status.auth_created)),
            cli_row("services.toml", init_status_name(status.services_created)),
            cli_row(
                "global-main runtime.toml",
                init_status_name(status.runtime_created),
            ),
        ],
    );

    Ok(())
}

fn print_config_validate() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = store.load_auth_pool().map_err(config_error)?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let services = store.load_services().map_err(config_error)?;

    ConfigStore::validate_runtime_config(&runtime, &auth_pool).map_err(config_error)?;
    ConfigStore::validate_service_profiles(&services, &auth_pool).map_err(config_error)?;

    emit_cli_panel(
        "配置校验",
        vec![
            cli_row("认证池", "已读取"),
            cli_row("实例运行配置", "通过"),
            cli_row("服务角色配置", "通过"),
        ],
    );

    Ok(())
}

fn print_auth_list() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;

    let mut rows = Vec::new();
    for entry in auth_pool.available_entries() {
        rows.push(cli_row("auth_ref", entry.auth_ref.as_str()));
        rows.push(cli_row("provider", entry.provider.as_str()));
        rows.push(cli_row("kind", auth_kind_name(&entry.kind)));
        rows.push(cli_row("enabled", entry.enabled.to_string()));
        rows.push(cli_row(
            "capabilities",
            entry
                .capabilities
                .iter()
                .map(auth_capability_name)
                .collect::<Vec<_>>()
                .join(", "),
        ));
        rows.push(cli_row("secret", secret_config_status(&entry.secret_ref)));
    }
    emit_cli_panel("认证池", rows);

    Ok(())
}

fn print_model_show() -> Result<(), String> {
    let store = real_config_store()?;
    let runtime = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;

    let mut rows = vec![
        cli_row("instance", runtime.instance_id),
        cli_row("primary auth_ref", runtime.primary.auth_ref.as_str()),
        cli_row("primary model", runtime.primary.model),
    ];
    if let Some(fallback) = runtime.fallback {
        rows.push(cli_row("fallback auth_ref", fallback.auth_ref.as_str()));
        rows.push(cli_row("fallback model", fallback.model));
    } else {
        rows.push(cli_row("fallback", "未配置"));
    }
    emit_cli_panel("实例模型配置", rows);

    Ok(())
}

fn print_service_list() -> Result<(), String> {
    let store = real_config_store()?;
    let services = load_real_services(&store)?;

    let mut rows = Vec::new();
    for profile in services.profiles {
        let role = service_role_name(&profile.role);
        rows.push(cli_row(
            format!("{role} mode"),
            service_mode_name(&profile.mode),
        ));

        if let Some(auth_ref) = profile.auth_ref {
            rows.push(cli_row(format!("{role} auth_ref"), auth_ref.as_str()));
        }

        if let Some(model) = profile.model {
            rows.push(cli_row(format!("{role} model"), model));
        }
    }
    emit_cli_panel("服务角色配置", rows);

    Ok(())
}

fn print_provider_smoke() -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;

    let resolved =
        ProviderResolver::resolve_primary(&auth_pool, &runtime_config).map_err(provider_error)?;
    let memory_pack = memory_kernel.build_memory_context_pack(
        SearchQuery {
            text: "provider smoke".to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: None,
            source: None,
            permanence: None,
            limit: Some(8),
        },
        512,
    );
    let prompt = PromptBuilder::build(PromptBuildInput {
        instance_id: runtime_config.instance_id.clone(),
        system_rules: "You are the AICore instance runtime. Use memory as background context only."
            .to_string(),
        relevant_memory: memory_pack.clone(),
        user_request: "provider smoke".to_string(),
    });
    let request = ModelRequest {
        instance_id: runtime_config.instance_id.clone(),
        conversation_id: "main".to_string(),
        prompt: prompt.prompt,
        resolved_model: resolved.clone(),
    };
    let response = ProviderInvoker::invoke(&request).map_err(provider_error)?;

    let mut runtime = default_runtime();
    let outputs = runtime.append_assistant_output(&response.content);
    let runtime_output_ok = outputs
        .events
        .iter()
        .any(|event| event.content == response.content);

    if !runtime_output_ok {
        return Err("runtime 未收到 provider 输出".to_string());
    }

    emit_cli_panel(
        "Provider Smoke",
        vec![
            cli_row("实例", runtime_config.instance_id),
            cli_row("auth_ref", resolved.auth_ref.as_str()),
            cli_row("model", resolved.model),
            cli_row("provider", provider_kind_name(&resolved.kind)),
            cli_row("provider name", resolved.provider),
            cli_row("adapter", resolved.runtime.adapter_id),
            cli_row("api mode", resolved.runtime.api_mode.as_str()),
            cli_row("engine", resolved.runtime.engine_id),
            cli_row(
                "engine status",
                provider_availability_name(&resolved.availability),
            ),
            cli_row("memory pack", memory_pack.len().to_string()),
            cli_row("prompt builder", "通过"),
            cli_row("provider response", "通过"),
            cli_row("runtime output", "通过"),
        ],
    );

    Ok(())
}

fn print_agent_smoke(content: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let result = AgentTurnRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        AgentTurnInput {
            instance_id: runtime_config.instance_id.clone(),
            transport_envelope: TransportEnvelope {
                source: GatewaySource::Cli,
                platform: None,
                target_id: None,
                sender_id: None,
                is_group: false,
                mentioned_bot: false,
            },
            interrupt_mode: InterruptMode::Queue,
            scope: global_main_memory_scope(),
            user_input: content.to_string(),
            memory_query: None,
            memory_limit: Some(8),
            memory_token_budget: 512,
            system_rules:
                "You are the AICore instance runtime. Use memory as background context only."
                    .to_string(),
            include_debug_prompt: false,
        },
    )
    .map_err(|error| error.0)?;
    let surface = result.to_conversation_surface();
    let turn = &surface.latest_turn;

    if matches!(result.outcome, AgentTurnOutcome::Failed) {
        let stage = turn
            .failure_stage
            .as_ref()
            .map(agent_turn_failure_stage_name)
            .unwrap_or("unknown");
        let message = turn.error_message.as_deref().unwrap_or("未知错误");
        return Err(format!("Agent Turn 失败：阶段={stage}，错误={message}"));
    }

    emit_cli_panel(
        "Agent Loop",
        vec![
            cli_row("status", "通过"),
            cli_row("实例", runtime_config.instance_id),
            cli_row("outcome", agent_turn_outcome_name(&turn.outcome)),
            cli_row("memory pack", format!("{} 条", turn.memory_count)),
            cli_row("prompt builder", "通过"),
            cli_row("ingress source", turn.accepted_source.as_str()),
            cli_row("provider invoked", bool_status_name(turn.provider_invoked)),
            cli_row(
                "provider",
                turn.provider_kind.as_deref().unwrap_or("<none>"),
            ),
            cli_row(
                "provider name",
                turn.provider_name.as_deref().unwrap_or("<none>"),
            ),
            cli_row(
                "assistant output present",
                bool_status_name(turn.assistant_output_present),
            ),
            cli_row(
                "failure stage",
                turn.failure_stage
                    .as_ref()
                    .map(agent_turn_failure_stage_name)
                    .unwrap_or("<none>"),
            ),
            cli_row("runtime output", "已追加"),
            cli_row("conversation", surface.conversation_id),
            cli_row("event count", turn.event_count.to_string()),
            cli_row("queue len", turn.queue_len.to_string()),
        ],
    );

    Ok(())
}

fn print_agent_session_smoke(first: &str, second: &str) -> Result<(), String> {
    let store = real_config_store()?;
    let auth_pool = load_real_auth_pool(&store)?;
    let runtime_config = store
        .load_instance_runtime("global-main")
        .map_err(map_runtime_load_error)?;
    let memory_kernel = real_memory_kernel()?;
    let mut runtime = default_runtime();

    let session = AgentSessionRunner::run(
        &mut runtime,
        &memory_kernel,
        &auth_pool,
        &runtime_config,
        vec![
            AgentTurnInput {
                instance_id: runtime_config.instance_id.clone(),
                transport_envelope: TransportEnvelope {
                    source: GatewaySource::Cli,
                    platform: None,
                    target_id: None,
                    sender_id: None,
                    is_group: false,
                    mentioned_bot: false,
                },
                interrupt_mode: InterruptMode::Queue,
                scope: global_main_memory_scope(),
                user_input: first.to_string(),
                memory_query: None,
                memory_limit: Some(8),
                memory_token_budget: 512,
                system_rules:
                    "You are the AICore instance runtime. Use memory as background context only."
                        .to_string(),
                include_debug_prompt: false,
            },
            AgentTurnInput {
                instance_id: runtime_config.instance_id.clone(),
                transport_envelope: TransportEnvelope {
                    source: GatewaySource::Cli,
                    platform: None,
                    target_id: None,
                    sender_id: None,
                    is_group: false,
                    mentioned_bot: false,
                },
                interrupt_mode: InterruptMode::Queue,
                scope: global_main_memory_scope(),
                user_input: second.to_string(),
                memory_query: None,
                memory_limit: Some(8),
                memory_token_budget: 512,
                system_rules:
                    "You are the AICore instance runtime. Use memory as background context only."
                        .to_string(),
                include_debug_prompt: false,
            },
        ],
    )
    .map_err(|error| error.0)?;

    let surface = session.surface();

    let mut rows = vec![
        cli_row("status", "通过"),
        cli_row("conversation", surface.conversation_id.as_str()),
        cli_row("turns", surface.turn_count.to_string()),
        cli_row(
            "completed all inputs",
            bool_status_name(surface.completed_all_inputs),
        ),
        cli_row(
            "stop reason",
            surface
                .stop_reason
                .as_ref()
                .map(agent_session_stop_reason_name)
                .unwrap_or("<none>"),
        ),
        cli_row(
            "latest outcome",
            surface
                .latest_turn
                .as_ref()
                .map(|turn| agent_turn_outcome_name(&turn.outcome))
                .unwrap_or("<none>"),
        ),
        cli_row("conversation status", surface.conversation_status.as_str()),
        cli_row("event count", surface.event_count.to_string()),
        cli_row("queue len", surface.queue_len.to_string()),
    ];
    for (index, turn) in surface.turns.iter().enumerate() {
        rows.push(cli_row(
            format!("turn {} outcome", index + 1),
            agent_turn_outcome_name(&turn.outcome),
        ));
        rows.push(cli_row(
            format!("turn {} provider invoked", index + 1),
            bool_status_name(turn.provider_invoked),
        ));
        rows.push(cli_row(
            format!("turn {} assistant output present", index + 1),
            bool_status_name(turn.assistant_output_present),
        ));
        rows.push(cli_row(
            format!("turn {} failure stage", index + 1),
            turn.failure_stage
                .as_ref()
                .map(agent_turn_failure_stage_name)
                .unwrap_or("<none>"),
        ));
    }
    emit_cli_panel("Agent Session", rows);

    Ok(())
}

fn agent_turn_outcome_name(outcome: &AgentTurnOutcome) -> &'static str {
    match outcome {
        AgentTurnOutcome::Completed => "completed",
        AgentTurnOutcome::Queued => "queued",
        AgentTurnOutcome::AppendedContext => "appended_context",
        AgentTurnOutcome::Interrupted => "interrupted",
        AgentTurnOutcome::Failed => "failed",
    }
}

fn agent_turn_failure_stage_name(stage: &aicore_agent::AgentTurnFailureStage) -> &'static str {
    match stage {
        aicore_agent::AgentTurnFailureStage::ProviderResolve => "provider_resolve",
        aicore_agent::AgentTurnFailureStage::ProviderInvoke => "provider_invoke",
        aicore_agent::AgentTurnFailureStage::RuntimeAppend => "runtime_append",
    }
}

fn agent_session_stop_reason_name(reason: &AgentSessionStopReason) -> &'static str {
    match reason {
        AgentSessionStopReason::Failed => "failed",
        AgentSessionStopReason::Queued => "queued",
        AgentSessionStopReason::AppendedContext => "appended_context",
        AgentSessionStopReason::Interrupted => "interrupted",
    }
}

fn bool_status_name(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn print_memory_status() -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel = MemoryKernel::open(paths.clone()).map_err(memory_error)?;

    let body = [
        "- instance: global-main".to_string(),
        format!("- root: {}", paths.root.display()),
        format!("- records: {}", kernel.records().len()),
        format!("- proposals: {}", kernel.proposals().len()),
        format!("- events: {}", kernel.events().len()),
        format!("- projection stale: {}", kernel.projection_state().stale),
        format!(
            "- projection warning: {}",
            kernel
                .projection_state()
                .warning
                .as_deref()
                .unwrap_or("<none>")
        ),
        format!(
            "- last rebuild at: {}",
            kernel
                .projection_state()
                .last_rebuild_at
                .as_deref()
                .unwrap_or("<none>")
        ),
    ]
    .join("\n");

    emit_cli_panel_body("Memory Status：", &body);

    Ok(())
}

fn print_memory_audit() -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let report = kernel.verify_ledger_consistency();

    render_memory_audit(&report);
    Ok(())
}

fn print_memory_proposals() -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let proposals = kernel.list_open_proposals();

    if proposals.is_empty() {
        emit_cli_panel_body("Memory Proposals：", "暂无待审阅记忆提案。");
        return Ok(());
    }

    let mut lines = Vec::new();
    for proposal in proposals {
        let display_text = if !proposal.localized_summary.is_empty() {
            proposal.localized_summary
        } else if !proposal.content.is_empty() {
            proposal.content
        } else {
            proposal.normalized_content
        };
        lines.push(format!(
            "- {} [{}] {}",
            proposal.proposal_id,
            memory_type_name(&proposal.memory_type),
            display_text
        ));
    }

    emit_cli_panel_body("Memory Proposals：", &lines.join("\n"));

    Ok(())
}

fn print_memory_wiki_index() -> Result<(), String> {
    print_memory_wiki_page("index")
}

fn print_memory_wiki_page(page: &str) -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel = MemoryKernel::open(paths.clone()).map_err(memory_error)?;
    let (page_name, page_path) = resolve_memory_wiki_page(&paths, page)?;

    if !page_path.exists() {
        return Err("缺少 Wiki Projection，请先写入记忆或重建 projection。".to_string());
    }

    let content = fs::read_to_string(&page_path)
        .map_err(|error| format!("无法读取 Wiki Projection {}: {error}", page_path.display()))?;

    let mut metadata = wiki_projection_status_lines(kernel.projection_state());
    metadata.push(format!("- page: {page_name}"));
    metadata.push(format!("- path: {}", page_path.display()));

    emit_document(Document::new(vec![
        Block::panel("记忆 Wiki Projection：", &metadata.join("\n")),
        Block::markdown(&content),
    ]));

    Ok(())
}

fn print_memory_remember(content: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_main_memory_scope(),
            content: content.to_string(),
            localized_summary: content.to_string(),
            state_key: None,
            current_state: None,
        })
        .map_err(memory_error)?;

    emit_cli_panel_body(
        "记忆已写入：",
        &[
            format!("- id: {memory_id}"),
            "- type: core".to_string(),
            "- status: active".to_string(),
        ]
        .join("\n"),
    );

    Ok(())
}

fn print_memory_accept(proposal_id: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    let memory_id = kernel
        .accept_proposal(proposal_id, "user", Some("cli accept"))
        .map_err(memory_error)?;

    emit_cli_panel_body(
        "记忆提案已接受：",
        &[
            format!("- proposal: {proposal_id}"),
            format!("- memory: {memory_id}"),
        ]
        .join("\n"),
    );

    Ok(())
}

fn print_memory_reject(proposal_id: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    kernel
        .reject_proposal(proposal_id, "user", Some("cli reject"))
        .map_err(memory_error)?;

    emit_cli_panel_body("记忆提案已拒绝：", &format!("- proposal: {proposal_id}"));

    Ok(())
}

fn print_memory_search(query: &str, options: MemorySearchOptions) -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let results = kernel
        .search(SearchQuery {
            text: query.to_string(),
            scope: Some(global_main_memory_scope()),
            memory_type: options.memory_type,
            source: options.source,
            permanence: options.permanence,
            limit: options.limit,
        })
        .map_err(memory_error)?;

    let mut lines = Vec::new();
    if results.is_empty() {
        lines.push("- 无匹配记忆".to_string());
    } else {
        for result in results {
            let record = result.record;
            lines.push(format!(
                "- {} [{}] {}",
                record.memory_id,
                memory_type_name(&record.memory_type),
                record.content
            ));
            lines.push(format!("  source: {}", memory_source_name(&record.source)));
            lines.push(format!(
                "  permanence: {}",
                memory_permanence_name(&record.permanence)
            ));
            lines.push(format!("  score: {}", result.score));
            lines.push(format!("  matched: {}", result.matched_fields.join(",")));
        }
    }

    emit_cli_panel_body("记忆搜索：", &lines.join("\n"));

    Ok(())
}

fn render_memory_audit(report: &MemoryAuditReport) {
    let mut lines = vec![
        format!("- checked events: {}", report.checked_events),
        format!("- status: {}", if report.ok { "ok" } else { "failed" }),
    ];

    if !report.ok {
        for issue in &report.issues {
            lines.push(format!("- issue: {issue}"));
        }
    }

    emit_cli_panel_body("Memory Audit：", &lines.join("\n"));
}

fn output_target_name(target: &OutputTarget) -> &'static str {
    match target {
        OutputTarget::Origin => "origin",
        OutputTarget::ActiveViews => "active-views",
        OutputTarget::FollowedExternal => "followed-external",
    }
}

fn delivery_identity_name(identity: &DeliveryIdentity) -> String {
    match identity {
        DeliveryIdentity::ActiveViews => "active-views".to_string(),
        DeliveryIdentity::External {
            platform,
            target_id,
        } => {
            format!("external:{platform}:{target_id}")
        }
    }
}

fn prepare_demo_config_store(command_name: &str) -> Result<ConfigStore, String> {
    let root = demo_config_root(command_name);
    reset_demo_root(&root)?;

    let store = ConfigStore::new(ConfigPaths::new(&root));
    store.ensure_default_files().map_err(config_error)?;

    let auth_pool = demo_auth_pool();
    let runtime = demo_runtime_config();
    let services = demo_service_profiles();

    store.save_auth_pool(&auth_pool).map_err(config_error)?;
    store
        .save_instance_runtime(&runtime)
        .map_err(config_error)?;
    store.save_services(&services).map_err(config_error)?;

    Ok(store)
}

fn real_config_store() -> Result<ConfigStore, String> {
    Ok(ConfigStore::new(real_config_paths()?))
}

fn real_memory_kernel() -> Result<MemoryKernel, String> {
    MemoryKernel::open(real_memory_paths()?).map_err(memory_error)
}

fn load_real_auth_pool(store: &ConfigStore) -> Result<GlobalAuthPool, String> {
    if !store.paths.auth_toml.exists() {
        return Err("缺少认证池配置，请先运行 config init。".to_string());
    }

    store.load_auth_pool().map_err(config_error)
}

fn load_real_services(store: &ConfigStore) -> Result<GlobalServiceProfiles, String> {
    if !store.paths.services_toml.exists() {
        return Err("缺少服务角色配置，请先运行 config init。".to_string());
    }

    store.load_services().map_err(config_error)
}

fn real_config_paths() -> Result<ConfigPaths, String> {
    Ok(ConfigPaths::new(resolve_real_config_root()?))
}

fn real_memory_paths() -> Result<MemoryPaths, String> {
    Ok(MemoryPaths::new(
        resolve_real_config_root()?
            .join("instances")
            .join("global-main")
            .join("memory"),
    ))
}

fn resolve_memory_wiki_page(
    paths: &MemoryPaths,
    page: &str,
) -> Result<(&'static str, PathBuf), String> {
    if page.contains('/') || page.contains('\\') || page.contains("..") {
        return Err("不允许读取任意 Wiki 路径。".to_string());
    }

    let normalized = page.trim_end_matches(".md");

    match normalized {
        "index" => Ok(("index", paths.wiki_index_md.clone())),
        "core" => Ok(("core", paths.wiki_core_md.clone())),
        "decisions" => Ok(("decisions", paths.wiki_decisions_md.clone())),
        "status" => Ok(("status", paths.wiki_status_md.clone())),
        _ => Err(format!("未知 Wiki 页面：{page}")),
    }
}

fn wiki_projection_status_lines(state: &aicore_memory::ProjectionState) -> Vec<String> {
    let mut lines = Vec::new();
    if state.stale {
        lines.push("Projection 状态：stale".to_string());
    }
    if let Some(warning) = state.warning.as_deref() {
        lines.push(format!("Projection warning：{warning}"));
    }
    lines
}

fn resolve_real_config_root() -> Result<PathBuf, String> {
    if let Some(root) = env::var_os("AICORE_CONFIG_ROOT") {
        return Ok(PathBuf::from(root));
    }

    let home = env::var_os("HOME")
        .ok_or_else(|| "无法确定配置根目录，请设置 HOME 或 AICORE_CONFIG_ROOT。".to_string())?;

    Ok(PathBuf::from(home).join(".aicore").join("config"))
}

fn demo_config_root(command_name: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "aicore-cli-p45-{command_name}-{}",
        std::process::id()
    ))
}

fn reset_demo_root(root: &PathBuf) -> Result<(), String> {
    if root.exists() {
        fs::remove_dir_all(root)
            .map_err(|error| format!("无法清理临时配置目录 {}: {error}", root.display()))?;
    }

    Ok(())
}

struct InitStatus {
    auth_created: bool,
    services_created: bool,
    runtime_created: bool,
}

fn initialize_real_config(store: &ConfigStore) -> Result<InitStatus, String> {
    let auth_created = write_auth_pool_if_missing(store, &demo_auth_pool())?;
    let services_created = write_services_if_missing(store, &demo_service_profiles())?;
    let runtime_created = write_runtime_if_missing(store, &demo_runtime_config())?;

    Ok(InitStatus {
        auth_created,
        services_created,
        runtime_created,
    })
}

fn write_auth_pool_if_missing(store: &ConfigStore, pool: &GlobalAuthPool) -> Result<bool, String> {
    if store.paths.auth_toml.exists() {
        return Ok(false);
    }

    store.save_auth_pool(pool).map_err(config_error)?;
    Ok(true)
}

fn write_services_if_missing(
    store: &ConfigStore,
    services: &GlobalServiceProfiles,
) -> Result<bool, String> {
    if store.paths.services_toml.exists() {
        return Ok(false);
    }

    store.save_services(services).map_err(config_error)?;
    Ok(true)
}

fn write_runtime_if_missing(
    store: &ConfigStore,
    runtime: &InstanceRuntimeConfig,
) -> Result<bool, String> {
    let runtime_path = store.paths.runtime_toml_for(&runtime.instance_id);
    if runtime_path.exists() {
        return Ok(false);
    }

    store.save_instance_runtime(runtime).map_err(config_error)?;
    Ok(true)
}

fn demo_auth_pool() -> GlobalAuthPool {
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
            capabilities: vec![AuthCapability::Chat, AuthCapability::Vision],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openai.backup"),
            provider: "openai".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openai.backup"),
            capabilities: vec![AuthCapability::Chat],
            enabled: true,
        },
        AuthEntry {
            auth_ref: AuthRef::new("auth.openrouter.search"),
            provider: "openrouter".to_string(),
            kind: AuthKind::ApiKey,
            secret_ref: SecretRef::new("secret://auth.openrouter.search"),
            capabilities: vec![AuthCapability::Search],
            enabled: true,
        },
    ])
}

fn demo_runtime_config() -> InstanceRuntimeConfig {
    InstanceRuntimeConfig {
        instance_id: "global-main".to_string(),
        primary: ModelBinding {
            auth_ref: AuthRef::new("auth.dummy.main"),
            model: "dummy/default-chat".to_string(),
        },
        fallback: Some(ModelBinding {
            auth_ref: AuthRef::new("auth.openrouter.main"),
            model: "openai/gpt-5".to_string(),
        }),
    }
}

fn demo_service_profiles() -> GlobalServiceProfiles {
    GlobalServiceProfiles {
        profiles: vec![
            ServiceProfile {
                role: ServiceRole::MemoryDreamer,
                mode: ServiceProfileMode::InheritInstance,
                auth_ref: None,
                model: None,
            },
            ServiceProfile {
                role: ServiceRole::EvolutionReviewer,
                mode: ServiceProfileMode::Disabled,
                auth_ref: None,
                model: None,
            },
            ServiceProfile {
                role: ServiceRole::Search,
                mode: ServiceProfileMode::Explicit,
                auth_ref: Some(AuthRef::new("auth.openrouter.search")),
                model: Some("perplexity/sonar".to_string()),
            },
        ],
    }
}

fn config_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) => format!("I/O 错误：{message}"),
        aicore_config::ConfigError::Parse(message) => format!("配置解析错误：{message}"),
        aicore_config::ConfigError::Validation(message) => {
            format!("配置校验错误：{message}")
        }
    }
}

fn memory_error(error: aicore_memory::MemoryError) -> String {
    error.0
}

fn provider_error(error: ProviderError) -> String {
    match error {
        ProviderError::Resolve(message) => format!("provider 解析错误：{message}"),
        ProviderError::Invoke(message) => format!("provider 调用错误：{message}"),
    }
}

fn provider_kind_name(kind: &aicore_provider::ProviderKind) -> &'static str {
    match kind {
        aicore_provider::ProviderKind::Dummy => "dummy",
        aicore_provider::ProviderKind::OpenRouter => "openrouter",
        aicore_provider::ProviderKind::OpenAI => "openai",
        aicore_provider::ProviderKind::Anthropic => "anthropic",
        aicore_provider::ProviderKind::Kimi => "kimi",
        aicore_provider::ProviderKind::KimiCoding => "kimi-coding",
        aicore_provider::ProviderKind::DeepSeek => "deepseek",
        aicore_provider::ProviderKind::Glm => "glm",
        aicore_provider::ProviderKind::MiniMax => "minimax",
        aicore_provider::ProviderKind::MiniMaxOpenAI => "minimax-openai",
        aicore_provider::ProviderKind::OpenAICodexLogin => "openai-codex-login",
        aicore_provider::ProviderKind::CustomOpenAICompatible => "custom-openai-compatible",
        aicore_provider::ProviderKind::CustomAnthropicCompatible => "custom-anthropic-compatible",
        aicore_provider::ProviderKind::Xiaomi => "xiaomi",
    }
}

fn provider_availability_name(
    availability: &aicore_provider::ProviderAvailability,
) -> &'static str {
    match availability {
        aicore_provider::ProviderAvailability::Available => "available",
        aicore_provider::ProviderAvailability::AdapterUnavailable => "boundary",
    }
}

fn map_runtime_load_error(error: aicore_config::ConfigError) -> String {
    match error {
        aicore_config::ConfigError::Io(message) if message.contains("missing runtime config") => {
            "缺少 global-main runtime 配置，请先运行 config init 或配置模型。".to_string()
        }
        other => config_error(other),
    }
}

fn init_status_name(created: bool) -> &'static str {
    if created {
        "已创建"
    } else {
        "已存在，未覆盖"
    }
}

fn secret_config_status(secret_ref: &SecretRef) -> &'static str {
    if secret_ref.as_str().is_empty() {
        "missing"
    } else {
        "configured"
    }
}

fn auth_kind_name(kind: &AuthKind) -> &'static str {
    match kind {
        AuthKind::ApiKey => "api-key",
        AuthKind::OAuth => "oauth",
        AuthKind::Session => "session",
        AuthKind::Token => "token",
    }
}

fn auth_capability_name(capability: &AuthCapability) -> &'static str {
    match capability {
        AuthCapability::Chat => "chat",
        AuthCapability::Vision => "vision",
        AuthCapability::Search => "search",
        AuthCapability::Embedding => "embedding",
    }
}

fn service_role_name(role: &ServiceRole) -> &'static str {
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

fn service_mode_name(mode: &ServiceProfileMode) -> &'static str {
    match mode {
        ServiceProfileMode::InheritInstance => "inherit_instance",
        ServiceProfileMode::Explicit => "explicit",
        ServiceProfileMode::Disabled => "disabled",
    }
}

fn memory_type_name(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}

fn memory_source_name(source: &MemorySource) -> &'static str {
    match source {
        MemorySource::UserExplicit => "user_explicit",
        MemorySource::UserCorrection => "user_correction",
        MemorySource::AssistantSummary => "assistant_summary",
        MemorySource::RuleBasedAgent => "rule_based_agent",
    }
}

fn memory_permanence_name(permanence: &MemoryPermanence) -> &'static str {
    match permanence {
        MemoryPermanence::Standard => "standard",
        MemoryPermanence::Permanent => "permanent",
    }
}

fn global_main_memory_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

#[derive(Debug, Default)]
struct MemorySearchOptions {
    memory_type: Option<MemoryType>,
    source: Option<MemorySource>,
    permanence: Option<MemoryPermanence>,
    limit: Option<usize>,
}

fn parse_memory_search_options(args: &[String]) -> Result<MemorySearchOptions, String> {
    let mut options = MemorySearchOptions::default();
    let mut index = 0usize;

    while index < args.len() {
        match args[index].as_str() {
            "--type" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --type 参数值。".to_string())?;
                options.memory_type = Some(parse_memory_type_filter(value)?);
                index += 2;
            }
            "--source" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --source 参数值。".to_string())?;
                options.source = Some(parse_memory_source_filter(value)?);
                index += 2;
            }
            "--permanence" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --permanence 参数值。".to_string())?;
                options.permanence = Some(parse_memory_permanence_filter(value)?);
                index += 2;
            }
            "--limit" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "缺少 --limit 参数值。".to_string())?;
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "--limit 必须是正整数。".to_string())?;
                if parsed == 0 {
                    return Err("--limit 必须是正整数。".to_string());
                }
                options.limit = Some(parsed);
                index += 2;
            }
            other => {
                return Err(format!("未知参数：{other}"));
            }
        }
    }

    Ok(options)
}

fn parse_memory_type_filter(value: &str) -> Result<MemoryType, String> {
    match value {
        "core" => Ok(MemoryType::Core),
        "working" => Ok(MemoryType::Working),
        "status" => Ok(MemoryType::Status),
        "decision" => Ok(MemoryType::Decision),
        _ => Err(format!("无效的 --type：{value}")),
    }
}

fn parse_memory_source_filter(value: &str) -> Result<MemorySource, String> {
    match value {
        "user_explicit" => Ok(MemorySource::UserExplicit),
        "user_correction" => Ok(MemorySource::UserCorrection),
        "assistant_summary" => Ok(MemorySource::AssistantSummary),
        "rule_based_agent" => Ok(MemorySource::RuleBasedAgent),
        _ => Err(format!("无效的 --source：{value}")),
    }
}

fn parse_memory_permanence_filter(value: &str) -> Result<MemoryPermanence, String> {
    match value {
        "standard" => Ok(MemoryPermanence::Standard),
        "permanent" => Ok(MemoryPermanence::Permanent),
        _ => Err(format!("无效的 --permanence：{value}")),
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KernelInvocationAdoptionClass {
    KernelNativeNow,
    KernelDiagnostic,
    AllowedLocalDirectCommand,
    MustMigrateToKernelInvocationLater,
    NotKernelInvocationTarget,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KernelInvocationAdoptionEntry {
    command: &'static str,
    operation: &'static str,
    class: KernelInvocationAdoptionClass,
    manifest_capability_exists: bool,
    route_runtime_used: bool,
    invocation_runtime_used: bool,
    ledger_used: bool,
    structured_result_envelope_used: bool,
    direct_local_execution_allowed_for_now: bool,
    future_migration_required: bool,
    reason: &'static str,
}

#[cfg(test)]
fn kernel_invocation_adoption_matrix() -> &'static [KernelInvocationAdoptionEntry] {
    use KernelInvocationAdoptionClass::{
        AllowedLocalDirectCommand, KernelDiagnostic, KernelNativeNow,
        MustMigrateToKernelInvocationLater, NotKernelInvocationTarget,
    };

    &[
        KernelInvocationAdoptionEntry {
            command: "aicore-cli kernel route <operation>",
            operation: "<operation>",
            class: KernelDiagnostic,
            manifest_capability_exists: true,
            route_runtime_used: true,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "route decision diagnostic",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli kernel invoke-smoke <operation>",
            operation: "<operation>",
            class: KernelDiagnostic,
            manifest_capability_exists: true,
            route_runtime_used: true,
            invocation_runtime_used: true,
            ledger_used: true,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "dispatcher and ledger diagnostic",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli kernel invoke-readonly runtime.status",
            operation: "runtime.status",
            class: KernelNativeNow,
            manifest_capability_exists: true,
            route_runtime_used: true,
            invocation_runtime_used: true,
            ledger_used: true,
            structured_result_envelope_used: true,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: false,
            reason: "first-party readonly kernel-native path",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli status",
            operation: "system.status",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "local status surface retained until system status adoption",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli runtime smoke",
            operation: "runtime.smoke",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "runtime capability smoke should adopt invocation boundary",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli instance list",
            operation: "instance.list",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "local read surface retained until instance readonly adoption",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli config path",
            operation: "config.path",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "local config path read has low migration priority",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli config init",
            operation: "config.init",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "bootstrap write command needs explicit config contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli config validate",
            operation: "config.validate",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "config readonly validation should adopt invocation boundary later",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli auth list",
            operation: "auth.list",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "auth read surface needs secret-safe invocation boundary",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli model show",
            operation: "model.show",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "model binding read surface should adopt readonly contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli service list",
            operation: "service.list",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: true,
            reason: "service profile read surface should adopt readonly contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli provider smoke",
            operation: "provider.smoke",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "provider app capability must not remain direct long term",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli agent smoke <内容>",
            operation: "agent.smoke",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "agent capability should flow through runtime invocation",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli agent session-smoke <第一轮内容> <第二轮内容>",
            operation: "agent.session_smoke",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "agent session capability should flow through runtime invocation",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory status",
            operation: "memory.status",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory app read capability should adopt memory contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory search <关键词>",
            operation: "memory.search",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "current memory.search invoke-smoke is diagnostic only",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory proposals",
            operation: "memory.proposals",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory app read capability should adopt memory contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory audit",
            operation: "memory.audit",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory audit should adopt memory contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory wiki [page]",
            operation: "memory.wiki",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: true,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory projection read surface should adopt memory contract",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory remember <内容>",
            operation: "memory.remember",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory write capability needs memory contract and audit boundary",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory accept <proposal_id>",
            operation: "memory.accept",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory write capability needs memory contract and audit boundary",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore-cli memory reject <proposal_id>",
            operation: "memory.reject",
            class: MustMigrateToKernelInvocationLater,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: true,
            reason: "memory write capability needs memory contract and audit boundary",
        },
        KernelInvocationAdoptionEntry {
            command: "aicore top-level status",
            operation: "runtime.status",
            class: KernelNativeNow,
            manifest_capability_exists: true,
            route_runtime_used: true,
            invocation_runtime_used: true,
            ledger_used: true,
            structured_result_envelope_used: true,
            direct_local_execution_allowed_for_now: false,
            future_migration_required: false,
            reason: "top-level status consumes runtime.status result envelope",
        },
        KernelInvocationAdoptionEntry {
            command: "cargo foundation",
            operation: "foundation.install",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "local bootstrap/install workflow",
        },
        KernelInvocationAdoptionEntry {
            command: "cargo kernel",
            operation: "kernel.install",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "local kernel runtime install workflow",
        },
        KernelInvocationAdoptionEntry {
            command: "cargo core",
            operation: "core.workflow",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "local aggregate workflow",
        },
        KernelInvocationAdoptionEntry {
            command: "cargo app-*",
            operation: "app.install",
            class: AllowedLocalDirectCommand,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "local app binary and manifest install workflow",
        },
        KernelInvocationAdoptionEntry {
            command: "help / usage / unknown command",
            operation: "none",
            class: NotKernelInvocationTarget,
            manifest_capability_exists: false,
            route_runtime_used: false,
            invocation_runtime_used: false,
            ledger_used: false,
            structured_result_envelope_used: false,
            direct_local_execution_allowed_for_now: true,
            future_migration_required: false,
            reason: "usage and error text is not a kernel capability invocation",
        },
    ]
}

#[cfg(test)]
mod tests {
    use aicore_memory::ProjectionState;

    use super::{
        KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix, run_from_args,
        wiki_projection_status_lines,
    };

    #[test]
    fn rejects_unknown_command() {
        assert_eq!(run_from_args(vec!["unknown".to_string()]), 1);
    }

    #[test]
    fn rejects_unknown_config_command() {
        assert_eq!(
            run_from_args(vec!["config".to_string(), "unknown".to_string()]),
            1
        );
    }

    #[test]
    fn memory_wiki_warns_when_projection_stale() {
        let lines = wiki_projection_status_lines(&ProjectionState {
            stale: true,
            warning: None,
            last_rebuild_at: Some("123".to_string()),
        });

        assert!(lines.iter().any(|line| line == "Projection 状态：stale"));
    }

    #[test]
    fn memory_wiki_warns_when_projection_warning_exists() {
        let lines = wiki_projection_status_lines(&ProjectionState {
            stale: true,
            warning: Some("projection warning".to_string()),
            last_rebuild_at: Some("123".to_string()),
        });

        assert!(
            lines
                .iter()
                .any(|line| line == "Projection warning：projection warning")
        );
    }

    #[test]
    fn kernel_invocation_adoption_matrix_mentions_runtime_status() {
        let matrix = kernel_invocation_adoption_matrix();

        assert!(matrix.iter().any(|entry| {
            entry.command == "aicore-cli kernel invoke-readonly runtime.status"
                && entry.operation == "runtime.status"
        }));
    }

    #[test]
    fn kernel_invocation_adoption_matrix_marks_invoke_readonly_as_kernel_native() {
        let matrix = kernel_invocation_adoption_matrix();
        let entry = matrix
            .iter()
            .find(|entry| entry.command == "aicore-cli kernel invoke-readonly runtime.status")
            .expect("runtime.status readonly adoption entry should exist");

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(entry.structured_result_envelope_used);
        assert!(!entry.future_migration_required);
    }

    #[test]
    fn kernel_invocation_adoption_matrix_marks_invoke_smoke_as_diagnostic() {
        let matrix = kernel_invocation_adoption_matrix();
        let entry = matrix
            .iter()
            .find(|entry| entry.command == "aicore-cli kernel invoke-smoke <operation>")
            .expect("invoke-smoke adoption entry should exist");

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelDiagnostic);
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(!entry.structured_result_envelope_used);
        assert!(!entry.future_migration_required);
    }

    #[test]
    fn kernel_invocation_adoption_matrix_marks_direct_commands_explicitly() {
        let matrix = kernel_invocation_adoption_matrix();
        let config_path = matrix
            .iter()
            .find(|entry| entry.command == "aicore-cli config path")
            .expect("config path adoption entry should exist");
        let workflow = matrix
            .iter()
            .find(|entry| entry.command == "cargo foundation")
            .expect("cargo foundation adoption entry should exist");

        assert_eq!(
            config_path.class,
            KernelInvocationAdoptionClass::AllowedLocalDirectCommand
        );
        assert!(config_path.direct_local_execution_allowed_for_now);
        assert_eq!(
            workflow.class,
            KernelInvocationAdoptionClass::AllowedLocalDirectCommand
        );
        assert!(workflow.direct_local_execution_allowed_for_now);
        assert!(!workflow.future_migration_required);
    }

    #[test]
    fn kernel_invocation_adoption_matrix_marks_future_migration_targets() {
        let matrix = kernel_invocation_adoption_matrix();
        for command in [
            "aicore-cli provider smoke",
            "aicore-cli agent smoke <内容>",
            "aicore-cli memory search <关键词>",
            "aicore-cli memory remember <内容>",
        ] {
            let entry = matrix
                .iter()
                .find(|entry| entry.command == command)
                .unwrap_or_else(|| panic!("{command} adoption entry should exist"));

            assert_eq!(
                entry.class,
                KernelInvocationAdoptionClass::MustMigrateToKernelInvocationLater
            );
            assert!(entry.future_migration_required);
            assert!(!entry.invocation_runtime_used);
        }
    }

    #[test]
    fn adoption_matrix_marks_aicore_status_as_kernel_native() {
        let matrix = kernel_invocation_adoption_matrix();
        let entry = matrix
            .iter()
            .find(|entry| entry.command == "aicore top-level status")
            .expect("aicore top-level status entry should exist");

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
        assert_eq!(entry.operation, "runtime.status");
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(entry.structured_result_envelope_used);
        assert!(!entry.direct_local_execution_allowed_for_now);
        assert!(!entry.future_migration_required);
    }

    #[test]
    fn runtime_status_handler_not_owned_by_cli_private_path() {
        let source = include_str!("lib.rs");
        let forbidden = ["fn ", "kernel_runtime_status_handler("].concat();

        assert!(!source.contains(&forbidden));
        assert!(source.contains("runtime_status_handler_for_layout"));
    }
}
