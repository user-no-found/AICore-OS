use aicore_foundation::{AicoreClock, InstanceId, SessionId, SystemClock};
use aicore_session::traits::SessionLedger;
use aicore_session::types::{
    AppendControlEventRequest, AppendLedgerWriteRequest, AppendMessageRequest, BeginTurnRequest,
    ControlEventKind, CreateSessionRequest, FinishTurnRequest, LedgerWriteKind, MessageKind,
    RuntimeStatus, SessionStatus, SetRuntimeStateRequest, TurnStatus,
};

use crate::tests::{open_store, temp_store_path};

#[test]
fn create_session_atomic_writes_all_tables() {
    let path = temp_store_path("tx-create-session");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.001").expect("valid session id");
    let now = SystemClock.now();

    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Test Session".to_string(),
            created_at: now,
            metadata: None,
        })
        .expect("create_session should succeed");

    // Verify session exists
    let session = store
        .reader()
        .get_session(&session_id)
        .expect("get_session should succeed")
        .expect("session should exist");
    assert_eq!(session.session_id, "sess.001");
    assert_eq!(session.title, "Test Session");
    assert_eq!(session.status, SessionStatus::Active);

    // Verify control_event was written
    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let event_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM control_events WHERE instance_id = ?1",
            rusqlite::params![InstanceId::global_main().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(event_count, 1, "one control event should be written");

    // Verify ledger_writes was written
    let write_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ledger_writes WHERE instance_id = ?1",
            rusqlite::params![InstanceId::global_main().as_str()],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(write_count, 1, "one ledger write should be written");

    // Verify instance_runtime_state updated
    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.active_session_id, Some("sess.001".to_string()));
    assert_eq!(snapshot.active_turn_id, None);
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
}

#[test]
fn begin_turn_atomic_writes_all_tables() {
    let path = temp_store_path("tx-begin-turn");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.002").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Session 2".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .expect("begin_turn should succeed");

    // Verify control_events and ledger_writes have grown
    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM control_events", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        event_count, 2,
        "two control events should exist (create_session + begin_turn)"
    );

    let write_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_writes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(write_count, 2, "two ledger writes should exist");

    // Verify instance_runtime_state
    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.active_turn_id, Some("turn.001".to_string()));
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Running);
}

#[test]
fn append_message_atomic_writes_all_tables() {
    let path = temp_store_path("tx-append-msg");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.003").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Session 3".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .append_message(&AppendMessageRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: Some("turn.001".to_string()),
            message_id: "msg.001".to_string(),
            message_seq: 1,
            kind: MessageKind::User,
            content: "Hello".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .expect("append_message should succeed");

    // Verify messages table
    let messages = store
        .reader()
        .read_messages(&session_id)
        .expect("read_messages should succeed");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].message_id, "msg.001");
    assert_eq!(messages[0].message_seq, 1);
    assert_eq!(messages[0].kind, MessageKind::User);

    // Verify ledger_writes has grown (but not control_events for append_message)
    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM control_events", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        event_count, 2,
        "append_message should not write control_events by default"
    );

    let write_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_writes", [], |row| row.get(0))
        .unwrap();
    assert_eq!(write_count, 3, "three ledger writes should exist");

    // Verify last_message_seq in snapshot
    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.last_message_seq, Some(1));
}

#[test]
fn finish_turn_clears_active_turn() {
    let path = temp_store_path("tx-finish-turn");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.004").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Session 4".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .finish_turn(&FinishTurnRequest {
            instance_id: InstanceId::global_main(),
            turn_id: "turn.001".to_string(),
            finished_at: SystemClock.now(),
            terminal_status: TurnStatus::Completed,
        })
        .expect("finish_turn should succeed");

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(
        snapshot.active_turn_id, None,
        "active_turn_id should be cleared"
    );
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
}

#[test]
fn append_message_to_nonexistent_session_fails() {
    let path = temp_store_path("tx-orphan-msg");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.orphan").expect("valid session id");
    let result = store.writer().append_message(&AppendMessageRequest {
        instance_id: InstanceId::global_main(),
        session_id,
        turn_id: None,
        message_id: "msg.001".to_string(),
        message_seq: 1,
        kind: MessageKind::User,
        content: "Hello".to_string(),
        created_at: SystemClock.now(),
        metadata: None,
    });

    assert!(
        result.is_err(),
        "append_message to nonexistent session should fail"
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("session not found")
    );

    // Verify no messages written
    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let msg_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(msg_count, 0, "no messages should be written on failure");
}

#[test]
fn append_message_to_nonexistent_turn_fails() {
    let path = temp_store_path("tx-orphan-turn");
    let store = open_store(path.db_path());

    let session_id = SessionId::new("sess.005").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Session 5".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let result = store.writer().append_message(&AppendMessageRequest {
        instance_id: InstanceId::global_main(),
        session_id: session_id.clone(),
        turn_id: Some("turn.does.not.exist".to_string()),
        message_id: "msg.001".to_string(),
        message_seq: 1,
        kind: MessageKind::User,
        content: "Hello".to_string(),
        created_at: SystemClock.now(),
        metadata: None,
    });

    assert!(
        result.is_err(),
        "append_message to nonexistent turn should fail"
    );
    assert!(result.unwrap_err().to_string().contains("turn not found"));
}

#[test]
fn finish_turn_to_nonexistent_turn_fails() {
    let path = temp_store_path("tx-finish-orphan");
    let store = open_store(path.db_path());

    let result = store.writer().finish_turn(&FinishTurnRequest {
        instance_id: InstanceId::global_main(),
        turn_id: "turn.does.not.exist".to_string(),
        finished_at: SystemClock.now(),
        terminal_status: TurnStatus::Completed,
    });

    assert!(
        result.is_err(),
        "finish_turn for nonexistent turn should fail"
    );
    assert!(result.unwrap_err().to_string().contains("turn not found"));
}

#[test]
fn append_control_event_and_ledger_write_do_not_write_messages() {
    let path = temp_store_path("tx-control-write-separate");
    let store = open_store(path.db_path());

    store
        .writer()
        .append_control_event(&AppendControlEventRequest {
            instance_id: InstanceId::global_main(),
            turn_id: Some("turn.audit".to_string()),
            event_id: "event.audit.001".to_string(),
            event_kind: ControlEventKind::RuntimeStateUpdated,
            detail: "runtime_state_updated".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .append_ledger_write(&AppendLedgerWriteRequest {
            instance_id: InstanceId::global_main(),
            turn_id: Some("turn.audit".to_string()),
            write_id: "write.audit.001".to_string(),
            write_kind: LedgerWriteKind::Insert,
            target_table: "control_events".to_string(),
            target_id: "event.audit.001".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();

    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let message_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .unwrap();
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM control_events", [], |row| row.get(0))
        .unwrap();
    let write_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_writes", [], |row| row.get(0))
        .unwrap();

    assert_eq!(message_count, 0);
    assert_eq!(event_count, 1);
    assert_eq!(write_count, 1);
}

#[test]
fn set_runtime_state_updates_recovery_pointers_only() {
    let path = temp_store_path("tx-runtime-state");
    let store = open_store(path.db_path());

    store
        .writer()
        .set_runtime_state(&SetRuntimeStateRequest {
            instance_id: InstanceId::global_main(),
            active_session_id: Some("sess.runtime".to_string()),
            active_turn_id: Some("turn.runtime".to_string()),
            pending_input_id: Some("pending.runtime".to_string()),
            pending_approval_id: Some("approval.runtime".to_string()),
            runtime_status: RuntimeStatus::Stopping,
            dirty_shutdown: true,
            recovery_required: true,
            updated_at: SystemClock.now(),
        })
        .unwrap();

    let snapshot = store.reader().get_runtime_state().unwrap();
    assert_eq!(snapshot.active_session_id, Some("sess.runtime".to_string()));
    assert_eq!(snapshot.active_turn_id, Some("turn.runtime".to_string()));
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Stopping);
    assert!(snapshot.dirty_shutdown);
    assert!(snapshot.recovery_required);
}

#[test]
fn mismatched_instance_id_rejects_write_without_partial_rows() {
    let path = temp_store_path("tx-instance-mismatch");
    let store = open_store(path.db_path());
    let other = InstanceId::new("workspace.other").unwrap();
    let session_id = SessionId::new("sess.mismatch").unwrap();

    let result = store.writer().create_session(&CreateSessionRequest {
        instance_id: other,
        session_id,
        title: "Mismatch".to_string(),
        created_at: SystemClock.now(),
        metadata: None,
    });

    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("instance id mismatch")
    );

    let conn = rusqlite::Connection::open(path.db_path()).unwrap();
    let session_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
        .unwrap();
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM control_events", [], |row| row.get(0))
        .unwrap();
    let write_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_writes", [], |row| row.get(0))
        .unwrap();

    assert_eq!(session_count, 0);
    assert_eq!(event_count, 0);
    assert_eq!(write_count, 0);
}
