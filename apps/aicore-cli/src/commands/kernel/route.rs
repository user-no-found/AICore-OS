use aicore_foundation::AicoreLayout;
use aicore_kernel::{
    KernelRouteRuntime, KernelRouteRuntimeError, KernelRouteRuntimeInput, format_contract,
};

use crate::terminal::{cli_row, emit_cli_panel};

pub(crate) fn print_kernel_route(operation: &str) -> i32 {
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
