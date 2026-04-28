use aicore_memory::{MemoryPermanence, MemoryType, RememberInput};

use crate::config_store::{global_main_memory_scope, real_memory_kernel};
use crate::errors::memory_error;
use crate::terminal::emit_cli_panel_body;

pub(crate) fn print_memory_remember(content: &str) -> Result<(), String> {
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

pub(crate) fn print_memory_accept(proposal_id: &str) -> Result<(), String> {
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

pub(crate) fn print_memory_reject(proposal_id: &str) -> Result<(), String> {
    let mut kernel = real_memory_kernel()?;
    kernel
        .reject_proposal(proposal_id, "user", Some("cli reject"))
        .map_err(memory_error)?;

    emit_cli_panel_body("记忆提案已拒绝：", &format!("- proposal: {proposal_id}"));

    Ok(())
}
