use super::*;

impl MemoryKernel {
    pub fn verify_ledger_consistency(&self) -> MemoryAuditReport {
        let mut issues = Vec::new();

        for event in &self.events {
            match event.event_kind {
                MemoryEventKind::Accepted => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "accepted event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    if !self
                        .records
                        .iter()
                        .any(|record| record.memory_id == memory_id)
                    {
                        issues.push(format!(
                            "accepted event {} points to missing memory {}",
                            event.event_id, memory_id
                        ));
                    }

                    if let Some(proposal_id) = event.proposal_id.as_deref() {
                        match self
                            .proposals
                            .iter()
                            .find(|proposal| proposal.proposal_id == proposal_id)
                        {
                            Some(proposal) if proposal.status == MemoryProposalStatus::Accepted => {
                            }
                            Some(_) => issues.push(format!(
                                "accepted event {} points to non-accepted proposal {}",
                                event.event_id, proposal_id
                            )),
                            None => issues.push(format!(
                                "accepted event {} points to missing proposal {}",
                                event.event_id, proposal_id
                            )),
                        }
                    }
                }
                MemoryEventKind::Proposed => {
                    let Some(proposal_id) = event.proposal_id.as_deref() else {
                        issues.push(format!(
                            "proposed event {} missing proposal_id",
                            event.event_id
                        ));
                        continue;
                    };

                    if !self
                        .proposals
                        .iter()
                        .any(|proposal| proposal.proposal_id == proposal_id)
                    {
                        issues.push(format!(
                            "proposed event {} points to missing proposal {}",
                            event.event_id, proposal_id
                        ));
                    }
                }
                MemoryEventKind::Rejected => {
                    let Some(proposal_id) = event.proposal_id.as_deref() else {
                        issues.push(format!(
                            "rejected event {} missing proposal_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .proposals
                        .iter()
                        .find(|proposal| proposal.proposal_id == proposal_id)
                    {
                        Some(proposal) if proposal.status == MemoryProposalStatus::Rejected => {}
                        Some(_) => issues.push(format!(
                            "rejected event {} points to non-rejected proposal {}",
                            event.event_id, proposal_id
                        )),
                        None => issues.push(format!(
                            "rejected event {} points to missing proposal {}",
                            event.event_id, proposal_id
                        )),
                    }
                }
                MemoryEventKind::Corrected => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "corrected event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    let Some(record) = self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    else {
                        issues.push(format!(
                            "corrected event {} points to missing memory {}",
                            event.event_id, memory_id
                        ));
                        continue;
                    };

                    if record.status != MemoryStatus::Active {
                        issues.push(format!(
                            "corrected event {} points to non-active memory {}",
                            event.event_id, memory_id
                        ));
                    }

                    let old_memory_id = event
                        .reason
                        .as_deref()
                        .and_then(|reason| reason.strip_prefix("supersedes "));
                    match old_memory_id {
                        Some(old_memory_id) => {
                            if !self.edges.iter().any(|edge| {
                                edge.from_memory_id == memory_id
                                    && edge.to_memory_id == old_memory_id
                                    && edge.relation == "supersedes"
                            }) {
                                issues.push(format!(
                                    "corrected event {} missing supersedes edge {} -> {}",
                                    event.event_id, memory_id, old_memory_id
                                ));
                            }
                        }
                        None => issues.push(format!(
                            "corrected event {} missing supersedes reason",
                            event.event_id
                        )),
                    }
                }
                MemoryEventKind::Archived => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "archived event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    {
                        Some(record) if record.status == MemoryStatus::Archived => {}
                        Some(_) => issues.push(format!(
                            "archived event {} points to non-archived memory {}",
                            event.event_id, memory_id
                        )),
                        None => issues.push(format!(
                            "archived event {} points to missing memory {}",
                            event.event_id, memory_id
                        )),
                    }
                }
                MemoryEventKind::Forgotten => {
                    let Some(memory_id) = event.memory_id.as_deref() else {
                        issues.push(format!(
                            "forgotten event {} missing memory_id",
                            event.event_id
                        ));
                        continue;
                    };

                    match self
                        .records
                        .iter()
                        .find(|record| record.memory_id == memory_id)
                    {
                        Some(record) if record.status == MemoryStatus::Forgotten => {}
                        Some(_) => issues.push(format!(
                            "forgotten event {} points to non-forgotten memory {}",
                            event.event_id, memory_id
                        )),
                        None => issues.push(format!(
                            "forgotten event {} points to missing memory {}",
                            event.event_id, memory_id
                        )),
                    }
                }
            }
        }

        for proposal in &self.proposals {
            if !self.events.iter().any(|event| {
                event.event_kind == MemoryEventKind::Proposed
                    && event.proposal_id.as_deref() == Some(proposal.proposal_id.as_str())
            }) {
                issues.push(format!(
                    "proposal {} missing proposed event",
                    proposal.proposal_id
                ));
            }
        }

        MemoryAuditReport {
            ok: issues.is_empty(),
            checked_events: self.events.len(),
            issues,
        }
    }
}
