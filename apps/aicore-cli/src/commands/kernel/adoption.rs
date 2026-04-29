pub(crate) fn extract_local_flag(args: &[String]) -> (bool, Vec<String>) {
    let mut is_local = false;
    let mut stripped = Vec::with_capacity(args.len());
    for arg in args {
        if arg == "--local" {
            is_local = true;
        } else {
            stripped.push(arg.clone());
        }
    }
    (is_local, stripped)
}

pub(crate) fn build_local_direct_json(
    operation: &str,
    success: bool,
    fields: serde_json::Value,
) -> serde_json::Value {
    serde_json::json!({
        "event": "direct.command.result",
        "operation": operation,
        "success": success,
        "execution_path": "local_direct",
        "kernel_invocation_path": "not_used",
        "ledger_appended": false,
        "fields": fields
    })
}

pub(crate) fn emit_local_direct_json(operation: &str, success: bool, fields: serde_json::Value) {
    let json = build_local_direct_json(operation, success, fields);
    println!(
        "{}",
        serde_json::to_string(&json).expect("local direct result should encode")
    );
}

pub(crate) fn adopt_readonly(
    operation: &str,
    args: &[String],
    run_local_direct: impl FnOnce() -> i32,
) -> i32 {
    let (is_local, stripped_args) = extract_local_flag(args);
    if is_local {
        run_local_direct()
    } else {
        super::invoke::print_kernel_invoke_readonly(operation, &stripped_args)
    }
}

#[cfg(test)]
mod adoption_tests {
    use super::*;

    #[test]
    fn extract_local_flag_detects_local() {
        let (is_local, stripped) = extract_local_flag(&[
            "--local".to_string(),
            "arg1".to_string(),
            "arg2".to_string(),
        ]);
        assert!(is_local);
        assert_eq!(stripped, vec!["arg1", "arg2"]);
    }

    #[test]
    fn extract_local_flag_detects_no_local() {
        let (is_local, stripped) = extract_local_flag(&["arg1".to_string(), "arg2".to_string()]);
        assert!(!is_local);
        assert_eq!(stripped, vec!["arg1", "arg2"]);
    }

    #[test]
    fn extract_local_flag_filters_local_at_any_position() {
        let (is_local, stripped) = extract_local_flag(&[
            "arg1".to_string(),
            "--local".to_string(),
            "arg2".to_string(),
        ]);
        assert!(is_local);
        assert_eq!(stripped, vec!["arg1", "arg2"]);
    }

    #[test]
    fn extract_local_flag_dedupes_multiple_local() {
        let (is_local, stripped) = extract_local_flag(&[
            "--local".to_string(),
            "--local".to_string(),
            "arg1".to_string(),
        ]);
        assert!(is_local);
        assert_eq!(stripped, vec!["arg1"]);
    }

    #[test]
    fn local_direct_json_has_stable_schema() {
        let json =
            build_local_direct_json("cli.status", true, serde_json::json!({"field": "value"}));

        assert_eq!(
            json.get("event").unwrap().as_str().unwrap(),
            "direct.command.result"
        );
        assert_eq!(
            json.get("operation").unwrap().as_str().unwrap(),
            "cli.status"
        );
        assert_eq!(json.get("success").unwrap().as_bool().unwrap(), true);
        assert_eq!(
            json.get("execution_path").unwrap().as_str().unwrap(),
            "local_direct"
        );
        assert_eq!(
            json.get("kernel_invocation_path")
                .unwrap()
                .as_str()
                .unwrap(),
            "not_used"
        );
        assert_eq!(
            json.get("ledger_appended").unwrap().as_bool().unwrap(),
            false
        );
        assert!(json.get("fields").unwrap().get("field").is_some());
    }

    #[test]
    fn local_direct_json_failure_schema() {
        let json =
            build_local_direct_json("cli.status", false, serde_json::json!({"error": "failed"}));

        assert_eq!(json.get("success").unwrap().as_bool().unwrap(), false);
        assert_eq!(
            json.get("fields")
                .unwrap()
                .get("error")
                .unwrap()
                .as_str()
                .unwrap(),
            "failed"
        );
    }
}

#[cfg(test)]
#[path = "adoption_matrix.rs"]
mod adoption_matrix;

#[cfg(test)]
pub(crate) use adoption_matrix::{
    KernelInvocationAdoptionClass, kernel_invocation_adoption_matrix,
};
