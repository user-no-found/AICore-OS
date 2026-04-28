use super::*;

impl MemoryKernel {
    pub fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, MemoryError> {
        let candidate_ids = block_on(async {
            store::search_index_candidates(&self.paths.db_path, &query.text, query.limit).await
        })?;

        Ok(match candidate_ids {
            Some(candidate_ids) if !query.text.is_empty() && !candidate_ids.is_empty() => {
                filter_records_by_ids(&self.records, &query, &candidate_ids)
            }
            _ => filter_records(&self.records, &query),
        })
    }

    pub fn build_memory_context_pack(
        &self,
        query: SearchQuery,
        token_budget: usize,
    ) -> Vec<MemoryRecord> {
        build_memory_pack(&self.records, &query, token_budget)
    }

    pub fn records(&self) -> &[MemoryRecord] {
        &self.records
    }

    pub fn proposals(&self) -> &[MemoryProposal] {
        &self.proposals
    }

    pub fn events(&self) -> &[MemoryEvent] {
        &self.events
    }

    pub fn edges(&self) -> &[MemoryEdge] {
        &self.edges
    }

    pub fn projection_state(&self) -> &ProjectionState {
        &self.projection_state
    }
}
