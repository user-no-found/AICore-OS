use super::support::*;

#[test]
fn cli_kernel_route_smoke_routes_existing_operation() {
    let home = temp_root("kernel-route-existing");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "route", "memory.search"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核路由决策"));
    assert!(stdout.contains("decision：routed"));
    assert!(stdout.contains("operation：memory.search"));
    assert!(stdout.contains("component：aicore-cli"));
    assert!(stdout.contains("app：aicore-cli"));
    assert!(stdout.contains("capability：memory.search"));
    assert!(stdout.contains("contract：kernel.app.v1"));
}

#[test]
fn cli_kernel_route_smoke_reports_missing_operation() {
    let home = temp_root("kernel-route-missing");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "route", "unknown.operation"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核路由失败"));
    assert!(stdout.contains("reason：missing capability"));
    assert!(stdout.contains("operation：unknown.operation"));
}

#[test]
fn cli_kernel_route_smoke_outputs_chinese_summary() {
    let home = temp_root("kernel-route-chinese");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("provider.smoke", "provider.smoke")],
    );

    let output = run_cli_with_env(
        &["kernel", "route", "provider.smoke"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("内核路由决策"));
    assert!(stdout.contains("不会执行 handler"));
}

#[test]
fn cli_kernel_route_smoke_json_outputs_valid_json() {
    let home = temp_root("kernel-route-json");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "route", "memory.search"],
        &[
            ("HOME", home.to_str().expect("home path should be utf-8")),
            ("AICORE_TERMINAL", "json"),
        ],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    let events = assert_json_lines(&stdout);
    assert_has_json_event(&events, "block.panel");
    assert!(stdout.contains("memory.search"));
    assert!(!stdout.contains('╭'));
    assert!(!stdout.contains("\u{1b}["));
}

#[test]
fn cli_kernel_route_smoke_does_not_execute_handler() {
    let home = temp_root("kernel-route-no-handler");
    seed_route_manifest(
        &home,
        "aicore-cli.toml",
        "aicore-cli",
        &[("memory.search", "memory.search")],
    );

    let output = run_cli_with_env(
        &["kernel", "route", "memory.search"],
        &[("HOME", home.to_str().expect("home path should be utf-8"))],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(stdout.contains("handler executed：false"));
    assert!(!stdout.contains("records:"));
    assert!(!stdout.contains("memory root"));
}
