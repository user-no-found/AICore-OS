use aicore_foundation::{AicoreClock, SessionId, SystemClock};
use aicore_session::traits::{SessionLedger, SessionLedgerReader, SessionLedgerWriter};
use aicore_session::types::{
    AppendMessageRequest, BeginTurnRequest, CreateSessionRequest, FinishTurnRequest, MessageKind,
    RuntimeStatus, TurnStatus,
};

use crate::tests::{open_store, temp_db_path};

#[test]
fn snapshot_returns_idle_after_creation() {
    let path = temp_db_path("snapshot-idle");
    let store = open_store(&path);

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.instance_id, "global-main");
    assert_eq!(snapshot.active_session_id, None);
    assert_eq!(snapshot.active_turn_id, None);
    assert_eq!(snapshot.last_message_seq, None);
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
    assert!(!snapshot.dirty_shutdown);
    assert!(!snapshot.recovery_required);
}

#[test]
fn snapshot_tracks_active_session_and_turn() {
    let path = temp_db_path("snapshot-active");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.snap.001").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Active Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(
        snapshot.active_session_id,
        Some("sess.snap.001".to_string())
    );
    assert_eq!(snapshot.active_turn_id, Some("turn.001".to_string()));
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Running);
}

#[test]
fn snapshot_clears_active_turn_after_finish() {
    let path = temp_db_path("snapshot-finish");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.snap.002").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Finish Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .finish_turn(&FinishTurnRequest {
            turn_id: "turn.001".to_string(),
            finished_at: SystemClock.now(),
            terminal_status: TurnStatus::Completed,
        })
        .unwrap();

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(
        snapshot.active_session_id,
        Some("sess.snap.002".to_string())
    );
    assert_eq!(
        snapshot.active_turn_id, None,
        "active_turn_id should be cleared after finish"
    );
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
}

#[test]
fn snapshot_tracks_last_message_seq() {
    let path = temp_db_path("snapshot-msg-seq");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.snap.003").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Msg Seq Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .append_message(&AppendMessageRequest {
            session_id: session_id.clone(),
            turn_id: Some("turn.001".to_string()),
            message_id: "msg.001".to_string(),
            message_seq: 1,
            kind: MessageKind::User,
            content: "Hello".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.last_message_seq, Some(1));

    store
        .writer()
        .append_message(&AppendMessageRequest {
            session_id: session_id.clone(),
            turn_id: Some("turn.001".to_string()),
            message_id: "msg.002".to_string(),
            message_seq: 5,
            kind: MessageKind::AssistantFinal,
            content: "World".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.last_message_seq, Some(5));
}

#[test]
fn snapshot_preserves_unfinished_turn_pointer() {
    let path = temp_db_path("snapshot-unfinished");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.snap.004").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Unfinished Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    // Drop store (simulates dirty shutdown)
    drop(store);

    // Reopen - pointer should still show active turn
    let store = open_store(&path);
    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(
        snapshot.active_session_id,
        Some("sess.snap.004".to_string())
    );
    assert_eq!(snapshot.active_turn_id, Some("turn.001".to_string()));
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Running);
}

#[test]
fn list_sessions_returns_created_sessions() {
    let path = temp_db_path("snapshot-list");
    let store = open_store(&path);

    let session_id1 = SessionId::new("sess.list.001").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id1.clone(),
            title: "First Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let session_id2 = SessionId::new("sess.list.002").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id2.clone(),
            title: "Second Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let sessions = store.reader().list_sessions().unwrap();
    assert_eq!(sessions.len(), 2);
    let titles: Vec<String> = sessions.iter().map(|s| s.title.clone()).collect();
    assert!(titles.contains(&"First Session".to_string()));
    assert!(titles.contains(&"Second Session".to_string()));

    // Turn count should be correct
    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id1.clone(),
            turn_id: "turn.001".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();
    store
        .writer()
        .finish_turn(&FinishTurnRequest {
            turn_id: "turn.001".to_string(),
            finished_at: SystemClock.now(),
            terminal_status: TurnStatus::Completed,
        })
        .unwrap();

    let sessions = store.reader().list_sessions().unwrap();
    let s1 = sessions
        .iter()
        .find(|s| s.session_id == "sess.list.001")
        .unwrap();
    let s2 = sessions
        .iter()
        .find(|s| s.session_id == "sess.list.002")
        .unwrap();
    assert_eq!(s1.turn_count, 1);
    assert_eq!(s2.turn_count, 0);
}

#[test]
fn get_session_returns_none_for_missing() {
    let path = temp_db_path("snapshot-missing");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.missing").expect("valid session id");
    let result = store.reader().get_session(&session_id).unwrap();
    assert_eq!(result, None);
}
