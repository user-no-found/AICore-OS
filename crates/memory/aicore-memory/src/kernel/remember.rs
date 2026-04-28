use super::*;

impl MemoryKernel {
    pub fn remember_user_explicit(
        &mut self,
        input: RememberInput,
    ) -> Result<MemoryId, MemoryError> {
        let _guard = self.acquire_write_guard("remember_user_explicit")?;
        self.maybe_fail_write_for_tests()?;
        let timestamp = now_string();
        let memory_id = next_id("mem");
        let event_id = next_id("evt");
        let content_language = infer_language(&input.content).to_string();
        let normalized = input.content.clone();

        let record = MemoryRecord {
            memory_id: memory_id.clone(),
            record_version: 1,
            memory_type: input.memory_type,
            status: MemoryStatus::Active,
            permanence: input.permanence,
            scope: input.scope.clone(),
            content: input.content,
            content_language: content_language.clone(),
            normalized_content: normalized,
            normalized_language: content_language,
            localized_summary: input.localized_summary,
            source: MemorySource::UserExplicit,
            evidence_json: "[]".to_string(),
            state_key: input.state_key,
            state_version: 1,
            current_state: input.current_state,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };

        let event = MemoryEvent {
            event_id,
            event_kind: MemoryEventKind::Accepted,
            memory_id: Some(memory_id.clone()),
            proposal_id: None,
            scope: input.scope,
            actor: "user".to_string(),
            reason: Some("remember".to_string()),
            evidence_json: "[]".to_string(),
            created_at: timestamp,
        };

        block_on(async {
            store::insert_record_and_event(&self.paths.db_path, &record, &event).await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(memory_id)
    }

    pub fn correct_by_user(
        &mut self,
        old_memory_id: &str,
        new_content: &str,
    ) -> Result<MemoryId, MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == old_memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {old_memory_id}")))?;

        self.correct_by_user_with_version(old_memory_id, expected_version, new_content)
    }

    pub fn correct_by_user_with_version(
        &mut self,
        old_memory_id: &str,
        expected_version: i64,
        new_content: &str,
    ) -> Result<MemoryId, MemoryError> {
        let _guard = self.acquire_write_guard("correct_by_user_with_version")?;
        self.maybe_fail_write_for_tests()?;
        let old_record = self
            .records
            .iter()
            .find(|record| record.memory_id == old_memory_id)
            .cloned()
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {old_memory_id}")))?;

        let timestamp = now_string();
        let new_memory_id = next_id("mem");
        let content_language = infer_language(new_content).to_string();
        let record = MemoryRecord {
            memory_id: new_memory_id.clone(),
            record_version: 1,
            memory_type: old_record.memory_type,
            status: MemoryStatus::Active,
            permanence: old_record.permanence,
            scope: old_record.scope.clone(),
            content: new_content.to_string(),
            content_language: content_language.clone(),
            normalized_content: new_content.to_string(),
            normalized_language: content_language,
            localized_summary: new_content.to_string(),
            source: MemorySource::UserCorrection,
            evidence_json: "[]".to_string(),
            state_key: old_record.state_key,
            state_version: old_record.state_version + 1,
            current_state: old_record.current_state,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let event = MemoryEvent {
            event_id: next_id("evt"),
            event_kind: MemoryEventKind::Corrected,
            memory_id: Some(new_memory_id.clone()),
            proposal_id: None,
            scope: old_record.scope,
            actor: "user".to_string(),
            reason: Some(format!("supersedes {old_memory_id}")),
            evidence_json: "[]".to_string(),
            created_at: timestamp,
        };

        block_on(async {
            store::supersede_record(
                &self.paths.db_path,
                old_memory_id,
                expected_version,
                &record,
                &event,
            )
            .await
        })?;
        self.refresh_cache()?;
        self.rebuild_projections_after_commit()?;

        Ok(new_memory_id)
    }

    pub fn archive(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {memory_id}")))?;

        self.archive_with_version(memory_id, expected_version)
    }

    pub fn archive_with_version(
        &mut self,
        memory_id: &str,
        expected_version: i64,
    ) -> Result<(), MemoryError> {
        let _guard = self.acquire_write_guard("archive_with_version")?;
        self.maybe_fail_write_for_tests()?;
        self.update_status(
            memory_id,
            expected_version,
            MemoryStatus::Archived,
            MemoryEventKind::Archived,
        )
    }

    pub fn forget(&mut self, memory_id: &str) -> Result<(), MemoryError> {
        let expected_version = self
            .records
            .iter()
            .find(|record| record.memory_id == memory_id)
            .map(|record| record.record_version)
            .ok_or_else(|| MemoryError(format!("unknown memory_id: {memory_id}")))?;

        self.forget_with_version(memory_id, expected_version)
    }

    pub fn forget_with_version(
        &mut self,
        memory_id: &str,
        expected_version: i64,
    ) -> Result<(), MemoryError> {
        let _guard = self.acquire_write_guard("forget_with_version")?;
        self.maybe_fail_write_for_tests()?;
        self.update_status(
            memory_id,
            expected_version,
            MemoryStatus::Forgotten,
            MemoryEventKind::Forgotten,
        )
    }
}
