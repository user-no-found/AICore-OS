pub fn exported_memory_proposal_symbols() -> &'static [&'static str] {
    &[
        "InMemoryMemoryProposalRuntime",
        "create_proposal",
        "review_proposal",
        "build_review_card",
        "record_user_decision",
        "create_memory_agent_write_request",
        "get_proposal",
        "list_pending_reviews",
        "snapshot",
    ]
}
