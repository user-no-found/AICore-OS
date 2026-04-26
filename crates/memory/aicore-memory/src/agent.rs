use crate::types::{
    MemoryAgentOutput, MemoryProposal, MemoryProposalStatus, MemoryRequestedOutput, MemorySource,
    MemoryType, MemoryWorkBatch,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct RuleBasedMemoryAgent;

impl RuleBasedMemoryAgent {
    pub fn analyze(batch: &MemoryWorkBatch) -> MemoryAgentOutput {
        if !batch
            .requested_outputs
            .iter()
            .any(|requested| matches!(requested, MemoryRequestedOutput::Proposals))
        {
            return MemoryAgentOutput {
                proposals: Vec::new(),
                corrections: Vec::new(),
                archive_suggestions: Vec::new(),
            };
        }

        let mut proposals = Vec::new();

        for excerpt in &batch.raw_excerpts {
            if let Some(content) = extract_remember_content(excerpt) {
                proposals.push(build_proposal(batch, MemoryType::Core, &content, &content));
            }

            if excerpt.contains("已完成 P") {
                proposals.push(build_proposal(batch, MemoryType::Status, excerpt, excerpt));
            }

            if excerpt.contains("纠正") || excerpt.contains("不是") || excerpt.contains("你看错了")
            {
                proposals.push(build_proposal(batch, MemoryType::Working, excerpt, excerpt));
            }
        }

        MemoryAgentOutput {
            proposals: dedupe_proposals(proposals),
            corrections: Vec::new(),
            archive_suggestions: Vec::new(),
        }
    }
}

fn extract_remember_content(excerpt: &str) -> Option<String> {
    excerpt
        .find("记住")
        .map(|index| {
            excerpt[index + "记住".len()..]
                .trim_start_matches(['：', ':', ' ', '\t'])
                .trim()
                .to_string()
        })
        .filter(|content| !content.is_empty())
}

fn build_proposal(
    batch: &MemoryWorkBatch,
    memory_type: MemoryType,
    content: &str,
    localized_summary: &str,
) -> MemoryProposal {
    MemoryProposal {
        proposal_id: format!(
            "agent_prop_{}_{}",
            memory_type_name(&memory_type),
            normalize(content)
        ),
        memory_type,
        scope: batch.scope.clone(),
        source: MemorySource::RuleBasedAgent,
        status: MemoryProposalStatus::Open,
        content: content.to_string(),
        content_language: infer_language(content).to_string(),
        normalized_content: content.to_string(),
        normalized_language: infer_language(content).to_string(),
        localized_summary: localized_summary.to_string(),
        created_at: "0".to_string(),
    }
}

fn dedupe_proposals(proposals: Vec<MemoryProposal>) -> Vec<MemoryProposal> {
    let mut deduped = Vec::new();

    for proposal in proposals {
        if deduped.iter().any(|existing: &MemoryProposal| {
            existing.memory_type == proposal.memory_type
                && existing.normalized_content == proposal.normalized_content
        }) {
            continue;
        }
        deduped.push(proposal);
    }

    deduped
}

fn infer_language(content: &str) -> &'static str {
    if content.is_ascii() { "en" } else { "zh-CN" }
}

fn normalize(content: &str) -> String {
    content
        .chars()
        .map(|ch| if ch.is_whitespace() { '_' } else { ch })
        .collect()
}

fn memory_type_name(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Core => "core",
        MemoryType::Working => "working",
        MemoryType::Status => "status",
        MemoryType::Decision => "decision",
    }
}
