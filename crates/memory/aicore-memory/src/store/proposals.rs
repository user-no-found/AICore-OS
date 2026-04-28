use super::*;
use crate::store::{
    codec::{
        memory_permanence_name, memory_source_name, memory_status_name, memory_type_name,
        proposal_status_name, row_to_proposal,
    },
    events::insert_event_tx,
    transaction::connect,
};
use sqlx::Connection;

pub async fn insert_proposal_and_event(
    db_path: &Path,
    proposal: &MemoryProposal,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    let scope = match &proposal.scope {
        MemoryScope::GlobalMain { instance_id } => MemoryScope::GlobalMain {
            instance_id: instance_id.clone(),
        },
        MemoryScope::Workspace {
            instance_id,
            workspace_root,
        } => MemoryScope::Workspace {
            instance_id: instance_id.clone(),
            workspace_root: workspace_root.clone(),
        },
    };

    sqlx::query(
        "INSERT INTO memory_proposals (
            proposal_id, memory_type, scope_kind, instance_id, workspace_root,
            source, status, content, content_language, normalized_content, normalized_language,
            localized_summary, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&proposal.proposal_id)
    .bind(memory_type_name(&proposal.memory_type))
    .bind(scope_kind(&scope))
    .bind(instance_id(&scope))
    .bind(workspace_root(&scope))
    .bind(memory_source_name(&proposal.source))
    .bind(proposal_status_name(&proposal.status))
    .bind(&proposal.content)
    .bind(&proposal.content_language)
    .bind(&proposal.normalized_content)
    .bind(&proposal.normalized_language)
    .bind(&proposal.localized_summary)
    .bind(&proposal.created_at)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn accept_proposal(
    db_path: &Path,
    proposal_id: &str,
    record: &MemoryRecord,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_proposals
         SET status = ?
         WHERE proposal_id = ? AND status = ?",
    )
    .bind(proposal_status_name(&MemoryProposalStatus::Accepted))
    .bind(proposal_id)
    .bind(proposal_status_name(&MemoryProposalStatus::Open))
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
    }

    sqlx::query(
        "INSERT INTO memory_records (
            memory_id, record_version, memory_type, status, permanence, scope_kind, instance_id, workspace_root,
            content, content_language, normalized_content, normalized_language, localized_summary,
            source, evidence_json, state_key, state_version, current_state, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&record.memory_id)
    .bind(record.record_version)
    .bind(memory_type_name(&record.memory_type))
    .bind(memory_status_name(&record.status))
    .bind(memory_permanence_name(&record.permanence))
    .bind(scope_kind(&record.scope))
    .bind(instance_id(&record.scope))
    .bind(workspace_root(&record.scope))
    .bind(&record.content)
    .bind(&record.content_language)
    .bind(&record.normalized_content)
    .bind(&record.normalized_language)
    .bind(&record.localized_summary)
    .bind(memory_source_name(&record.source))
    .bind(&record.evidence_json)
    .bind(&record.state_key)
    .bind(record.state_version)
    .bind(&record.current_state)
    .bind(&record.created_at)
    .bind(&record.updated_at)
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn reject_proposal(
    db_path: &Path,
    proposal_id: &str,
    event: &MemoryEvent,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    let mut tx = conn
        .begin()
        .await
        .map_err(|error| MemoryError(error.to_string()))?;

    let update = sqlx::query(
        "UPDATE memory_proposals
         SET status = ?
         WHERE proposal_id = ? AND status = ?",
    )
    .bind(proposal_status_name(&MemoryProposalStatus::Rejected))
    .bind(proposal_id)
    .bind(proposal_status_name(&MemoryProposalStatus::Open))
    .execute(&mut *tx)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    if update.rows_affected() == 0 {
        return Err(MemoryError(format!("non-open proposal: {proposal_id}")));
    }

    insert_event_tx(&mut tx, event).await?;
    tx.commit()
        .await
        .map_err(|error| MemoryError(error.to_string()))
}

pub async fn load_proposals(db_path: &Path) -> Result<Vec<MemoryProposal>, MemoryError> {
    let mut conn = connect(db_path).await?;
    let rows =
        sqlx::query("SELECT * FROM memory_proposals ORDER BY created_at ASC, proposal_id ASC")
            .fetch_all(&mut conn)
            .await
            .map_err(|error| MemoryError(error.to_string()))?;

    rows.into_iter().map(row_to_proposal).collect()
}

#[cfg(test)]
pub async fn delete_proposal_for_tests(
    db_path: &Path,
    proposal_id: &str,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query("DELETE FROM memory_proposals WHERE proposal_id = ?")
        .bind(proposal_id)
        .execute(&mut conn)
        .await
        .map_err(|error| MemoryError(error.to_string()))?;
    Ok(())
}
