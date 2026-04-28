use super::*;

impl MemoryKernel {
    pub fn core_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.core_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn status_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.status_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_index_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_index_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_core_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_core_md).map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_decisions_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_decisions_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn wiki_status_markdown(&self) -> Result<String, MemoryError> {
        fs::read_to_string(&self.paths.wiki_status_md)
            .map_err(|error| MemoryError(error.to_string()))
    }

    pub fn set_projection_failure_for_tests(&mut self, should_fail: bool) {
        self.projection_should_fail_for_tests = should_fail;
    }

    #[cfg(test)]
    pub fn set_write_failure_for_tests(&mut self, should_fail: bool) {
        self.write_should_fail_for_tests = should_fail;
    }

    #[cfg(test)]
    pub fn delete_record_for_tests(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        block_on(async { store::delete_record_for_tests(&self.paths.db_path, memory_id).await })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn delete_proposal_for_tests(&mut self, proposal_id: &str) -> Result<(), MemoryError> {
        block_on(async {
            store::delete_proposal_for_tests(&self.paths.db_path, proposal_id).await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn delete_edge_for_tests(
        &mut self,
        from_memory_id: &str,
        to_memory_id: &str,
        relation: &str,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::delete_edge_for_tests(
                &self.paths.db_path,
                from_memory_id,
                to_memory_id,
                relation,
            )
            .await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn force_record_status_for_tests(
        &mut self,
        memory_id: &str,
        status: MemoryStatus,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::force_record_status_for_tests(&self.paths.db_path, memory_id, status).await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn force_normalized_content_for_tests(
        &mut self,
        memory_id: &str,
        normalized_content: &str,
    ) -> Result<(), MemoryError> {
        block_on(async {
            store::force_normalized_content_for_tests(
                &self.paths.db_path,
                memory_id,
                normalized_content,
            )
            .await
        })?;
        self.refresh_cache()
    }

    #[cfg(test)]
    pub fn search_index_available_for_tests(&self) -> Result<bool, MemoryError> {
        block_on(async { store::search_index_available(&self.paths.db_path).await })
    }

    #[cfg(test)]
    pub fn drop_search_index_for_tests(&mut self) -> Result<(), MemoryError> {
        block_on(async { store::drop_search_index_for_tests(&self.paths.db_path).await })?;
        Ok(())
    }
}
