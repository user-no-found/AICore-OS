use aicore_foundation::{AicoreClock, SessionId, SystemClock};
use aicore_session::traits::{SessionLedger, SessionLedgerReader, SessionLedgerWriter};
use aicore_session::types::{
    AppendMessageRequest, BeginTurnRequest, CreateSessionRequest, FinishTurnRequest, MessageKind,
    TurnStatus,
};

use crate::tests::{open_store, temp_db_path};

#[test]
fn turn_seq_increments_within_session() {
    let path = temp_db_path("ordering-turn-seq");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.001").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Seq Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    for turn_seq in 1..=3 {
        store
            .writer()
            .begin_turn(&BeginTurnRequest {
                session_id: session_id.clone(),
                turn_id: format!("turn.{turn_seq}"),
                turn_seq,
                started_at: SystemClock.now(),
            })
            .unwrap();
        store
            .writer()
            .finish_turn(&FinishTurnRequest {
                turn_id: format!("turn.{turn_seq}"),
                finished_at: SystemClock.now(),
                terminal_status: TurnStatus::Completed,
            })
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&path).unwrap();
    let seqs: Vec<i64> = conn
        .prepare("SELECT turn_seq FROM turns WHERE session_id = ?1 ORDER BY turn_seq")
        .unwrap()
        .query_map(rusqlite::params!["sess.ord.001"], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(seqs, vec![1, 2, 3]);
}

#[test]
fn turn_seq_allows_gaps() {
    let path = temp_db_path("ordering-turn-gaps");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.002").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Gap Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.1".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.3".to_string(),
            turn_seq: 3,
            started_at: SystemClock.now(),
        })
        .unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let seqs: Vec<i64> = conn
        .prepare("SELECT turn_seq FROM turns WHERE session_id = ?1 ORDER BY turn_seq")
        .unwrap()
        .query_map(rusqlite::params!["sess.ord.002"], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(seqs, vec![1, 3], "gap should be allowed");
}

#[test]
fn turn_seq_duplicate_fails() {
    let path = temp_db_path("ordering-turn-duplicate");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.003").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Dup Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.1".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    let result = store.writer().begin_turn(&BeginTurnRequest {
        session_id: session_id.clone(),
        turn_id: "turn.2".to_string(),
        turn_seq: 1,
        started_at: SystemClock.now(),
    });

    assert!(result.is_err(), "duplicate turn_seq should fail");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("constraint violation")
    );
}

#[test]
fn message_seq_increments_within_turn() {
    let path = temp_db_path("ordering-msg-seq");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.004").expect("valid session id");
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
            turn_id: "turn.1".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    for msg_seq in 1..=3 {
        store
            .writer()
            .append_message(&AppendMessageRequest {
                session_id: session_id.clone(),
                turn_id: Some("turn.1".to_string()),
                message_id: format!("msg.{msg_seq}"),
                message_seq: msg_seq,
                kind: MessageKind::User,
                content: format!("Message {msg_seq}"),
                created_at: SystemClock.now(),
                metadata: None,
            })
            .unwrap();
    }

    let conn = rusqlite::Connection::open(&path).unwrap();
    let seqs: Vec<i64> = conn
        .prepare("SELECT message_seq FROM messages WHERE turn_id = ?1 ORDER BY message_seq")
        .unwrap()
        .query_map(rusqlite::params!["turn.1"], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(seqs, vec![1, 2, 3]);
}

#[test]
fn message_seq_allows_gaps() {
    let path = temp_db_path("ordering-msg-gaps");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.005").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "Msg Gap Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .begin_turn(&BeginTurnRequest {
            session_id: session_id.clone(),
            turn_id: "turn.1".to_string(),
            turn_seq: 1,
            started_at: SystemClock.now(),
        })
        .unwrap();

    store
        .writer()
        .append_message(&AppendMessageRequest {
            session_id: session_id.clone(),
            turn_id: Some("turn.1".to_string()),
            message_id: "msg.1".to_string(),
            message_seq: 1,
            kind: MessageKind::User,
            content: "First".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    store
        .writer()
        .append_message(&AppendMessageRequest {
            session_id: session_id.clone(),
            turn_id: Some("turn.1".to_string()),
            message_id: "msg.3".to_string(),
            message_seq: 3,
            kind: MessageKind::User,
            content: "Third".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();

    let conn = rusqlite::Connection::open(&path).unwrap();
    let seqs: Vec<i64> = conn
        .prepare("SELECT message_seq FROM messages WHERE turn_id = ?1 ORDER BY message_seq")
        .unwrap()
        .query_map(rusqlite::params!["turn.1"], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();

    assert_eq!(seqs, vec![1, 3], "message gap should be allowed");
}

#[test]
fn business_ids_look_like_uuidv7() {
    let path = temp_db_path("ordering-uuidv7");
    let store = open_store(&path);

    let session_id = SessionId::new("sess.ord.006").expect("valid session id");
    store
        .writer()
        .create_session(&CreateSessionRequest {
            session_id: session_id.clone(),
            title: "UUID Session".to_string(),
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

    // Verify writer/reader generates id-like identifiers
    let messages = store.reader().read_messages(&session_id).unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].message_id, "msg.001");

    // Verify control_events event_id looks like uuid
    let conn = rusqlite::Connection::open(&path).unwrap();
    let event_id: String = conn
        .query_row(
            "SELECT event_id FROM control_events WHERE event_type = 'session_created'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    // UUID v7 format: xxxxxxxx-xxxx-7xxx-xxxx-xxxxxxxxxxxx
    let parts: Vec<&str> = event_id.split('-').collect();
    assert_eq!(parts.len(), 5, "event_id should be hyphen-separated UUID");
    assert_eq!(parts[2].len(), 4, "UUID should have 4-char segments");
    assert!(
        parts[2].starts_with('7'),
        "third segment should indicate UUIDv7: got {event_id}"
    );
}
