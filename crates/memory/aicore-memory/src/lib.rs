#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryType {
    Core,
    Permanent { requires_approval: bool },
    Working,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryStatus {
    Proposed,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryProposal {
    pub id: String,
    pub memory_type: MemoryType,
    pub status: MemoryStatus,
    pub source_excerpt: String,
    pub normalized_memory: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryKernel {
    proposals: Vec<MemoryProposal>,
}

impl MemoryKernel {
    pub fn new() -> Self {
        Self {
            proposals: Vec::new(),
        }
    }

    pub fn submit(&mut self, proposal: MemoryProposal) {
        self.proposals.push(proposal);
    }

    pub fn proposals(&self) -> &[MemoryProposal] {
        &self.proposals
    }
}

pub fn default_memory_kernel() -> MemoryKernel {
    let mut kernel = MemoryKernel::new();
    kernel.submit(MemoryProposal {
        id: "mem_001".to_string(),
        memory_type: MemoryType::Core,
        status: MemoryStatus::Proposed,
        source_excerpt: "User prefers Chinese UI.".to_string(),
        normalized_memory: "User prefers Chinese user-facing interaction.".to_string(),
    });
    kernel
}

#[cfg(test)]
mod tests {
    use super::{MemoryKernel, MemoryProposal, MemoryStatus, MemoryType, default_memory_kernel};

    #[test]
    fn stores_memory_proposal() {
        let mut kernel = MemoryKernel::new();
        kernel.submit(MemoryProposal {
            id: "mem_001".to_string(),
            memory_type: MemoryType::Core,
            status: MemoryStatus::Proposed,
            source_excerpt: "原始输入".to_string(),
            normalized_memory: "Normalized memory entry.".to_string(),
        });

        assert_eq!(kernel.proposals().len(), 1);
    }

    #[test]
    fn preserves_source_excerpt_verbatim() {
        let kernel = default_memory_kernel();
        assert_eq!(
            kernel.proposals()[0].source_excerpt,
            "User prefers Chinese UI."
        );
    }

    #[test]
    fn uses_english_normalized_memory_by_default() {
        let kernel = default_memory_kernel();
        assert_eq!(
            kernel.proposals()[0].normalized_memory,
            "User prefers Chinese user-facing interaction."
        );
    }

    #[test]
    fn permanent_memory_can_require_approval() {
        let proposal = MemoryProposal {
            id: "mem_perm".to_string(),
            memory_type: MemoryType::Permanent {
                requires_approval: true,
            },
            status: MemoryStatus::Proposed,
            source_excerpt: "A permanent safety rule.".to_string(),
            normalized_memory: "Permanent safety rule.".to_string(),
        };

        assert_eq!(
            proposal.memory_type,
            MemoryType::Permanent {
                requires_approval: true,
            }
        );
    }
}
