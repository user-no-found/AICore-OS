mod apply;
mod policy;
mod types;

use aicore_foundation::{AicoreResult, Timestamp};

use crate::error::sqlite_write_error;
use crate::store::SqliteEventStore;

pub use types::{RetentionApplyResult, RetentionPlan, RetentionSkip, RetentionSkipReason};

impl SqliteEventStore {
    pub fn plan_retention(&self, now: Timestamp) -> AicoreResult<RetentionPlan> {
        let connection = self.lock_connection()?;
        let records = policy::load_retention_records(&connection)?;

        policy::build_plan(&records, now, self.instance_id())
    }

    pub fn apply_retention(&self, now: Timestamp) -> AicoreResult<RetentionApplyResult> {
        let run_id = format!("run.{}", apply::current_run_nonce());
        self.apply_retention_internal(now, &run_id)
    }

    #[cfg(test)]
    pub(crate) fn apply_retention_with_run_id(
        &self,
        now: Timestamp,
        run_id: &str,
    ) -> AicoreResult<RetentionApplyResult> {
        self.apply_retention_internal(now, run_id)
    }

    fn apply_retention_internal(
        &self,
        now: Timestamp,
        run_id: &str,
    ) -> AicoreResult<RetentionApplyResult> {
        let mut connection = self.lock_connection()?;
        let tx = connection.transaction().map_err(sqlite_write_error)?;
        let records = policy::load_retention_records(&tx)?;
        let plan = policy::build_plan(&records, now, self.instance_id())?;
        let result = apply::apply_plan(&tx, run_id, now, plan)?;

        tx.commit().map_err(sqlite_write_error)?;
        Ok(result)
    }
}
