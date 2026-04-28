use aicore_memory::MemoryAuditReport;

use crate::config_store::{real_memory_kernel, real_memory_paths};
use crate::names::memory_type_name;
use crate::terminal::emit_cli_panel_body;

pub(crate) fn print_memory_status() -> Result<(), String> {
    let paths = real_memory_paths()?;
    let kernel =
        aicore_memory::MemoryKernel::open(paths.clone()).map_err(crate::errors::memory_error)?;

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

pub(crate) fn print_memory_audit() -> Result<(), String> {
    let kernel = real_memory_kernel()?;
    let report = kernel.verify_ledger_consistency();

    render_memory_audit(&report);
    Ok(())
}

pub(crate) fn print_memory_proposals() -> Result<(), String> {
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

pub(crate) fn render_memory_audit(report: &MemoryAuditReport) {
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
