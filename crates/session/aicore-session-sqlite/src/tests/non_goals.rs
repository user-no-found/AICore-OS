use crate::schema;

#[test]
fn p4_does_not_add_query_or_event_query_symbols() {
    let public_surface = [
        include_str!("../store/mod.rs"),
        include_str!("../store/active_turn_writer.rs"),
        include_str!("../store/pending_input_writer.rs"),
        include_str!("../store/stop_writer.rs"),
        include_str!("../store/approval_writer.rs"),
        include_str!("../store/reader.rs"),
    ]
    .join("\n")
    .to_lowercase();

    for forbidden in [
        "query_gateway",
        "event_query",
        "session_query",
        "replay_query",
    ] {
        assert!(
            !public_surface.contains(forbidden),
            "P4.1 must not add query runtime symbol: {forbidden}"
        );
    }
}

#[test]
fn p4_does_not_add_provider_tool_team_or_memory_runtime_entrypoints() {
    let public_surface = [
        include_str!("../store/mod.rs"),
        include_str!("../store/active_turn_writer.rs"),
        include_str!("../store/pending_input_writer.rs"),
        include_str!("../store/stop_writer.rs"),
        include_str!("../store/approval_writer.rs"),
        include_str!("../store/writer.rs"),
    ]
    .join("\n")
    .to_lowercase();

    for forbidden in [
        "provider_runtime",
        "model_runtime",
        "execute_tool",
        "tool_registry",
        "team_agent",
        "memory_proposal_runtime",
        "daemon",
        "scheduler",
    ] {
        assert!(
            !public_surface.contains(forbidden),
            "P4.1 must not add runtime entrypoint: {forbidden}"
        );
    }
}

#[test]
fn p4_schema_keeps_forbidden_raw_fields_outside_test_guards() {
    let schema = schema::schema_sql().to_lowercase();
    for forbidden in super::FORBIDDEN_FIELDS {
        assert!(
            !schema.contains(forbidden),
            "forbidden schema token leaked: {forbidden}"
        );
    }
}
