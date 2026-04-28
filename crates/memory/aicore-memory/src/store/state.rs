use super::*;
use crate::store::transaction::connect;

pub async fn load_projection_state(db_path: &Path) -> Result<ProjectionState, MemoryError> {
    let mut conn = connect(db_path).await?;
    let row = sqlx::query(
        "SELECT stale, warning, last_rebuild_at FROM memory_projection_state WHERE singleton = 1",
    )
    .fetch_one(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(projection_state(
        row.get::<i64, _>("stale") != 0,
        row.get::<Option<String>, _>("warning"),
        row.get::<Option<String>, _>("last_rebuild_at"),
    ))
}

pub async fn save_projection_state(
    db_path: &Path,
    state: &ProjectionState,
) -> Result<(), MemoryError> {
    let mut conn = connect(db_path).await?;
    sqlx::query(
        "UPDATE memory_projection_state SET stale = ?, warning = ?, last_rebuild_at = ? WHERE singleton = 1",
    )
    .bind(if state.stale { 1 } else { 0 })
    .bind(&state.warning)
    .bind(&state.last_rebuild_at)
    .execute(&mut conn)
    .await
    .map_err(|error| MemoryError(error.to_string()))?;

    Ok(())
}
