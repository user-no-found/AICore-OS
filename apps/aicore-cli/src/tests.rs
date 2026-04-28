use aicore_memory::ProjectionState;

use crate::{
    commands::{
        kernel::{KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix},
        memory::wiki::wiki_projection_status_lines,
    },
    run_from_args,
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
fn kernel_invocation_adoption_matrix_marks_config_validate_readonly_as_kernel_native() {
    let matrix = kernel_invocation_adoption_matrix();
    let entry = matrix
        .iter()
        .find(|entry| entry.command == "aicore-cli kernel invoke-readonly config.validate")
        .expect("config.validate readonly adoption entry should exist");

    assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
    assert_eq!(entry.operation, "config.validate");
    assert!(entry.route_runtime_used);
    assert!(entry.invocation_runtime_used);
    assert!(entry.ledger_used);
    assert!(entry.structured_result_envelope_used);
    assert!(!entry.direct_local_execution_allowed_for_now);
    assert!(!entry.future_migration_required);
}

#[test]
fn kernel_invocation_adoption_matrix_marks_auth_model_service_readonly_as_kernel_native() {
    let matrix = kernel_invocation_adoption_matrix();
    for command in [
        "aicore-cli kernel invoke-readonly auth.list",
        "aicore-cli kernel invoke-readonly model.show",
        "aicore-cli kernel invoke-readonly service.list",
    ] {
        let entry = matrix
            .iter()
            .find(|entry| entry.command == command)
            .unwrap_or_else(|| panic!("{command} adoption entry should exist"));

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(entry.structured_result_envelope_used);
        assert!(!entry.direct_local_execution_allowed_for_now);
        assert!(!entry.future_migration_required);
    }
}

#[test]
fn kernel_invocation_adoption_matrix_marks_runtime_instance_status_readonly_as_kernel_native() {
    let matrix = kernel_invocation_adoption_matrix();
    for command in [
        "aicore-cli kernel invoke-readonly runtime.smoke",
        "aicore-cli kernel invoke-readonly instance.list",
        "aicore-cli kernel invoke-readonly cli.status",
    ] {
        let entry = matrix
            .iter()
            .find(|entry| entry.command == command)
            .unwrap_or_else(|| panic!("{command} adoption entry should exist"));

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(entry.structured_result_envelope_used);
        assert!(!entry.direct_local_execution_allowed_for_now);
        assert!(!entry.future_migration_required);
    }
}

#[test]
fn kernel_invocation_adoption_matrix_marks_provider_smoke_readonly_as_kernel_native() {
    let matrix = kernel_invocation_adoption_matrix();
    let entry = matrix
        .iter()
        .find(|entry| entry.command == "aicore-cli kernel invoke-readonly provider.smoke")
        .expect("provider.smoke readonly adoption entry should exist");

    assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
    assert_eq!(entry.operation, "provider.smoke");
    assert!(entry.route_runtime_used);
    assert!(entry.invocation_runtime_used);
    assert!(entry.ledger_used);
    assert!(entry.structured_result_envelope_used);
    assert!(!entry.direct_local_execution_allowed_for_now);
    assert!(!entry.future_migration_required);
}

#[test]
fn kernel_invocation_adoption_matrix_marks_agent_smoke_readonly_as_kernel_native() {
    let matrix = kernel_invocation_adoption_matrix();
    for (command, operation) in [
        (
            "aicore-cli kernel invoke-readonly agent.smoke",
            "agent.smoke",
        ),
        (
            "aicore-cli kernel invoke-readonly agent.session_smoke",
            "agent.session_smoke",
        ),
    ] {
        let entry = matrix
            .iter()
            .find(|entry| entry.command == command)
            .unwrap_or_else(|| panic!("{command} adoption entry should exist"));

        assert_eq!(entry.class, KernelInvocationAdoptionClass::KernelNativeNow);
        assert_eq!(entry.operation, operation);
        assert!(entry.route_runtime_used);
        assert!(entry.invocation_runtime_used);
        assert!(entry.ledger_used);
        assert!(entry.structured_result_envelope_used);
        assert!(!entry.direct_local_execution_allowed_for_now);
        assert!(!entry.future_migration_required);
    }
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
fn kernel_invocation_adoption_matrix_marks_direct_agent_smoke_as_retained_direct_path() {
    let matrix = kernel_invocation_adoption_matrix();
    for command in [
        "aicore-cli agent smoke <内容>",
        "aicore-cli agent session-smoke <第一轮内容> <第二轮内容>",
    ] {
        let entry = matrix
            .iter()
            .find(|entry| entry.command == command)
            .unwrap_or_else(|| panic!("{command} adoption entry should exist"));

        assert_eq!(
            entry.class,
            KernelInvocationAdoptionClass::AllowedLocalDirectCommand
        );
        assert!(entry.direct_local_execution_allowed_for_now);
        assert!(!entry.invocation_runtime_used);
    }
}

#[test]
fn kernel_invocation_adoption_matrix_marks_direct_provider_smoke_as_retained_direct_path() {
    let matrix = kernel_invocation_adoption_matrix();
    let entry = matrix
        .iter()
        .find(|entry| entry.command == "aicore-cli provider smoke")
        .expect("provider smoke direct path entry should exist");

    assert_eq!(
        entry.class,
        KernelInvocationAdoptionClass::AllowedLocalDirectCommand
    );
    assert!(entry.direct_local_execution_allowed_for_now);
    assert!(!entry.invocation_runtime_used);
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
