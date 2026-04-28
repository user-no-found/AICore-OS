pub(super) use std::{collections::HashSet, env, fs, thread};

pub(super) use crate::{
    MemoryAgentOutput, MemoryEventKind, MemoryKernel, MemoryPaths, MemoryPermanence,
    MemoryProposal, MemoryProposalStatus, MemoryRequestedOutput, MemoryScope, MemorySource,
    MemoryStatus, MemoryTrigger, MemoryType, MemoryWorkBatch, RememberInput, RuleBasedMemoryAgent,
    SearchQuery, blocks_secret, build_core_projection_for_tests,
    build_decisions_projection_for_tests, build_memory_pack_for_tests,
    build_permanent_projection_for_tests, build_status_projection_for_tests,
};

pub(super) fn temp_paths(name: &str) -> MemoryPaths {
    let root = env::temp_dir().join(format!("aicore-memory-tests-{name}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp memory root should be removable");
    }
    MemoryPaths::new(root)
}

pub(super) fn global_scope() -> MemoryScope {
    MemoryScope::GlobalMain {
        instance_id: "global-main".to_string(),
    }
}

pub(super) fn write_lock_file(paths: &MemoryPaths, created_at: &str, operation: &str) {
    fs::create_dir_all(&paths.root).expect("memory root should be creatable");
    fs::write(
        &paths.lock_path,
        format!("pid=999999\ncreated_at={created_at}\noperation={operation}\n"),
    )
    .expect("lock file should be writable");
}

pub(super) fn work_batch(trigger: MemoryTrigger, excerpts: Vec<&str>) -> MemoryWorkBatch {
    MemoryWorkBatch {
        instance_id: "global-main".to_string(),
        scope: global_scope(),
        trigger,
        recent_events_summary: String::new(),
        raw_excerpts: excerpts.into_iter().map(ToString::to_string).collect(),
        existing_memory_hits: Vec::new(),
        token_budget: 1024,
        requested_outputs: vec![MemoryRequestedOutput::Proposals],
    }
}

pub(super) fn agent_proposal(memory_type: MemoryType, content: &str) -> MemoryProposal {
    MemoryProposal {
        proposal_id: format!("agent_prop_{content}"),
        memory_type,
        scope: global_scope(),
        source: MemorySource::RuleBasedAgent,
        status: MemoryProposalStatus::Rejected,
        content: content.to_string(),
        content_language: if content.is_ascii() {
            "en".to_string()
        } else {
            "zh-CN".to_string()
        },
        normalized_content: content.to_string(),
        normalized_language: if content.is_ascii() {
            "en".to_string()
        } else {
            "zh-CN".to_string()
        },
        localized_summary: content.to_string(),
        created_at: "0".to_string(),
    }
}
