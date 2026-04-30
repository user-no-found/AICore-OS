use std::path::{Path, PathBuf};
use std::sync::Mutex;

use aicore_event::{
    EventEnvelope, EventGetRequest, EventGetResponse, EventQueryRequest, EventQueryResponse,
    EventReader, EventWriter,
};
use aicore_foundation::{AicoreClock, AicoreResult, InstanceId, SystemClock};
use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{query_not_implemented, sqlite_schema_error, sqlite_write_error};
use crate::row::{EventRow, event_from_row, event_row_from_envelope};
use crate::schema;

pub struct SqliteEventStore {
    _path: PathBuf,
    _instance_id: InstanceId,
    connection: Mutex<Connection>,
}

impl SqliteEventStore {
    pub fn open(path: impl AsRef<Path>, instance_id: &InstanceId) -> AicoreResult<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                aicore_foundation::AicoreError::InvalidPath(format!(
                    "failed to create sqlite parent directory: {error}"
                ))
            })?;
        }

        let connection = schema::open_connection(&path)?;
        schema::initialize_or_validate(&connection, instance_id, SystemClock.now())?;

        Ok(Self {
            _path: path,
            _instance_id: instance_id.clone(),
            connection: Mutex::new(connection),
        })
    }
}

impl EventWriter for SqliteEventStore {
    fn write(&self, envelope: &EventEnvelope) -> AicoreResult<()> {
        envelope.validate()?;

        let mut connection = self.connection.lock().map_err(|_| {
            aicore_foundation::AicoreError::Unavailable("sqlite mutex poisoned".to_string())
        })?;

        let tx = connection.transaction().map_err(sqlite_write_error)?;

        let row = event_row_from_envelope(envelope);

        tx.execute(
            "INSERT INTO events (
                event_id, event_type, schema_version, occurred_at, recorded_at, source_component,
                source_instance, subject_type, subject_id, summary, retention_class,
                correlation_id, causation_id, invocation_id, redaction_level, visibility, status,
                replay_policy, evidence_ref, payload_ref
             ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6,
                ?7, ?8, ?9, ?10, ?11,
                ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20
             )",
            params![
                row.event_id,
                row.event_type,
                row.schema_version,
                row.occurred_at,
                row.recorded_at,
                row.source_component,
                row.source_instance,
                row.subject_type,
                row.subject_id,
                row.summary,
                row.retention_class,
                row.correlation_id,
                row.causation_id,
                row.invocation_id,
                row.redaction_level,
                row.visibility,
                row.status,
                row.replay_policy,
                envelope.evidence_ref,
                envelope.payload_ref,
            ],
        )
        .map_err(sqlite_write_error)?;

        for tag in &envelope.tag_set.tags {
            tx.execute(
                "INSERT INTO event_tags (event_id, tag) VALUES (?1, ?2)",
                params![envelope.event_id.as_str(), tag.as_str()],
            )
            .map_err(sqlite_write_error)?;
        }

        for tag in &envelope.tag_set.confirmed {
            tx.execute(
                "INSERT INTO event_confirmed_tags (event_id, tag) VALUES (?1, ?2)",
                params![envelope.event_id.as_str(), tag.as_str()],
            )
            .map_err(sqlite_write_error)?;
        }

        if let Some(evidence_ref) = &envelope.evidence_ref {
            tx.execute(
                "INSERT INTO event_refs (event_id, ref_kind, ref_value) VALUES (?1, 'evidence_ref', ?2)",
                params![envelope.event_id.as_str(), evidence_ref],
            )
            .map_err(sqlite_write_error)?;
        }

        if let Some(payload_ref) = &envelope.payload_ref {
            tx.execute(
                "INSERT INTO event_refs (event_id, ref_kind, ref_value) VALUES (?1, 'payload_ref', ?2)",
                params![envelope.event_id.as_str(), payload_ref],
            )
            .map_err(sqlite_write_error)?;
        }

        tx.commit().map_err(sqlite_write_error)
    }
}

impl EventReader for SqliteEventStore {
    fn query(&self, _request: &EventQueryRequest) -> AicoreResult<EventQueryResponse> {
        Err(query_not_implemented())
    }

    fn get(&self, request: &EventGetRequest) -> AicoreResult<EventGetResponse> {
        let connection = self.connection.lock().map_err(|_| {
            aicore_foundation::AicoreError::Unavailable("sqlite mutex poisoned".to_string())
        })?;

        let row = connection
            .query_row(
                "SELECT
                    event_id, event_type, schema_version, occurred_at, recorded_at, source_component,
                    source_instance, subject_type, subject_id, summary, retention_class,
                    correlation_id, causation_id, invocation_id, redaction_level, visibility,
                    status, replay_policy
                 FROM events
                 WHERE event_id = ?1",
                params![request.event_id],
                |row| {
                    Ok(EventRow {
                        event_id: row.get(0)?,
                        event_type: row.get(1)?,
                        schema_version: row.get(2)?,
                        occurred_at: row.get(3)?,
                        recorded_at: row.get(4)?,
                        source_component: row.get(5)?,
                        source_instance: row.get(6)?,
                        subject_type: row.get(7)?,
                        subject_id: row.get(8)?,
                        summary: row.get(9)?,
                        retention_class: row.get(10)?,
                        correlation_id: row.get(11)?,
                        causation_id: row.get(12)?,
                        invocation_id: row.get(13)?,
                        redaction_level: row.get(14)?,
                        visibility: row.get(15)?,
                        status: row.get(16)?,
                        replay_policy: row.get(17)?,
                    })
                },
            )
            .optional()
            .map_err(sqlite_schema_error)?;

        let Some(row) = row else {
            return Ok(EventGetResponse { event: None });
        };

        let tags = collect_strings(
            &connection,
            "SELECT tag FROM event_tags WHERE event_id = ?1 ORDER BY tag",
            &request.event_id,
        )?;
        let confirmed_tags = collect_strings(
            &connection,
            "SELECT tag FROM event_confirmed_tags WHERE event_id = ?1 ORDER BY tag",
            &request.event_id,
        )?;
        let evidence_ref = collect_optional_ref(&connection, &request.event_id, "evidence_ref")?;
        let payload_ref = collect_optional_ref(&connection, &request.event_id, "payload_ref")?;

        let event = event_from_row(row, evidence_ref, payload_ref, tags, confirmed_tags)?;
        Ok(EventGetResponse { event: Some(event) })
    }
}

fn collect_strings(
    connection: &Connection,
    sql: &str,
    event_id: &str,
) -> AicoreResult<Vec<String>> {
    let mut stmt = connection.prepare(sql).map_err(sqlite_schema_error)?;
    let rows = stmt
        .query_map(params![event_id], |row| row.get::<_, String>(0))
        .map_err(sqlite_schema_error)?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(sqlite_schema_error)
}

fn collect_optional_ref(
    connection: &Connection,
    event_id: &str,
    ref_kind: &str,
) -> AicoreResult<Option<String>> {
    connection
        .query_row(
            "SELECT ref_value FROM event_refs WHERE event_id = ?1 AND ref_kind = ?2",
            params![event_id, ref_kind],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(sqlite_schema_error)
}
