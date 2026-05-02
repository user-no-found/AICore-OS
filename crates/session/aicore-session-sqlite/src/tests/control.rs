use aicore_foundation::{AicoreClock, InstanceId, SessionId, SystemClock};
use aicore_session::traits::SessionLedger;
use aicore_session::types::{
    ActiveTurnAcquireRequest, ActiveTurnAcquireStatus, ActiveTurnReleaseRequest, ApprovalDecision,
    ApprovalResponseRequest, ApprovalResponseStatus, ApprovalScope, ApprovalStatus,
    CreateApprovalRequest, CreateSessionRequest, PendingInputCancelRequest, PendingInputStatus,
    PendingInputSubmitRequest, RuntimeStatus, StopTurnRequest, StopTurnStatus, TurnStatus,
};
use rusqlite::{Connection, params};

use crate::tests::{open_store, temp_store_path};

fn create_session(store: &crate::SqliteSessionStore, session_id: &SessionId) {
    store
        .writer()
        .create_session(&CreateSessionRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            title: "Control Session".to_string(),
            created_at: SystemClock.now(),
            metadata: None,
        })
        .unwrap();
}

fn acquire_turn(
    store: &crate::SqliteSessionStore,
    session_id: &SessionId,
    turn_id: &str,
) -> aicore_session::ActiveTurnAcquireOutcome {
    store
        .writer()
        .acquire_active_turn(&ActiveTurnAcquireRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: turn_id.to_string(),
            requested_at: SystemClock.now(),
        })
        .unwrap()
}

#[test]
fn active_turn_lock_acquire_release_and_versions() {
    let path = temp_store_path("control-active-turn");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.control").unwrap();
    create_session(&store, &session_id);

    let first = acquire_turn(&store, &session_id, "turn.one");
    assert_eq!(first.status, ActiveTurnAcquireStatus::Acquired);
    assert_eq!(first.active_turn_id.as_deref(), Some("turn.one"));
    assert_eq!(first.lock_version, 1);

    let second = store
        .writer()
        .acquire_active_turn(&ActiveTurnAcquireRequest {
            instance_id: InstanceId::global_main(),
            session_id: session_id.clone(),
            turn_id: "turn.two".to_string(),
            requested_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(second.status, ActiveTurnAcquireStatus::AlreadyActive);
    assert_eq!(second.active_turn_id.as_deref(), Some("turn.one"));

    let wrong_release = store
        .writer()
        .release_active_turn(&ActiveTurnReleaseRequest {
            instance_id: InstanceId::global_main(),
            turn_id: "turn.two".to_string(),
            terminal_status: TurnStatus::Completed,
            released_at: SystemClock.now(),
        });
    assert!(
        wrong_release
            .unwrap_err()
            .to_string()
            .contains("active turn")
    );

    let release = store
        .writer()
        .release_active_turn(&ActiveTurnReleaseRequest {
            instance_id: InstanceId::global_main(),
            turn_id: "turn.one".to_string(),
            terminal_status: TurnStatus::Completed,
            released_at: SystemClock.now(),
        })
        .unwrap();
    assert!(release.released);
    assert!(release.lock_version > first.lock_version);

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.active_turn_id, None);
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
    assert_eq!(snapshot.lock_version, release.lock_version);
}

#[test]
fn cross_instance_active_turn_write_is_rejected_without_rows() {
    let path = temp_store_path("control-cross-instance");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.cross").unwrap();
    create_session(&store, &session_id);
    let other = InstanceId::new("workspace.other").unwrap();

    let result = store
        .writer()
        .acquire_active_turn(&ActiveTurnAcquireRequest {
            instance_id: other,
            session_id,
            turn_id: "turn.cross".to_string(),
            requested_at: SystemClock.now(),
        });
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("instance id mismatch")
    );

    let conn = Connection::open(path.db_path()).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM turns WHERE turn_id = 'turn.cross'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn pending_input_is_single_pointer_and_not_a_message() {
    let path = temp_store_path("control-pending");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.pending").unwrap();
    create_session(&store, &session_id);
    acquire_turn(&store, &session_id, "turn.pending");

    let first = store
        .writer()
        .submit_or_replace_pending_input(&PendingInputSubmitRequest {
            instance_id: InstanceId::global_main(),
            pending_input_id: "pending.one".to_string(),
            session_id: Some(session_id.as_str().to_string()),
            turn_id: Some("turn.pending".to_string()),
            content: "first safe input".to_string(),
            submitted_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(first.status, PendingInputStatus::Pending);
    assert_eq!(first.replaced_pending_input_id, None);

    let second = store
        .writer()
        .submit_or_replace_pending_input(&PendingInputSubmitRequest {
            instance_id: InstanceId::global_main(),
            pending_input_id: "pending.two".to_string(),
            session_id: Some(session_id.as_str().to_string()),
            turn_id: Some("turn.pending".to_string()),
            content: "second safe input".to_string(),
            submitted_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(
        second.replaced_pending_input_id.as_deref(),
        Some("pending.one")
    );

    let pending = store.reader().get_pending_input().unwrap().unwrap();
    assert_eq!(pending.pending_input_id, "pending.two");
    assert_eq!(pending.status, PendingInputStatus::Pending);

    let conn = Connection::open(path.db_path()).unwrap();
    let old_status: String = conn
        .query_row(
            "SELECT status FROM pending_inputs WHERE pending_input_id = 'pending.one'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(old_status, "replaced");
    let messages: i64 = conn
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .unwrap();
    assert_eq!(messages, 0);

    let cancel = store
        .writer()
        .cancel_pending_input(&PendingInputCancelRequest {
            instance_id: InstanceId::global_main(),
            cancelled_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(
        cancel.cancelled_pending_input_id.as_deref(),
        Some("pending.two")
    );
    assert!(store.reader().get_pending_input().unwrap().is_none());
}

#[test]
fn stop_releases_active_turn_preserves_pending_and_does_not_write_final() {
    let path = temp_store_path("control-stop-running");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.stop").unwrap();
    create_session(&store, &session_id);
    acquire_turn(&store, &session_id, "turn.stop");
    store
        .writer()
        .submit_or_replace_pending_input(&PendingInputSubmitRequest {
            instance_id: InstanceId::global_main(),
            pending_input_id: "pending.stop".to_string(),
            session_id: Some(session_id.as_str().to_string()),
            turn_id: Some("turn.stop".to_string()),
            content: "queued safe input".to_string(),
            submitted_at: SystemClock.now(),
        })
        .unwrap();

    let outcome = store
        .writer()
        .request_stop_active_turn(&StopTurnRequest {
            instance_id: InstanceId::global_main(),
            requested_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(outcome.status, StopTurnStatus::StopRequested);
    assert_eq!(outcome.turn_id.as_deref(), Some("turn.stop"));

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.active_turn_id, None);
    assert_eq!(snapshot.pending_input_id.as_deref(), Some("pending.stop"));
    assert_eq!(snapshot.runtime_status, RuntimeStatus::Idle);
    assert_eq!(
        store
            .reader()
            .get_turn("turn.stop")
            .unwrap()
            .unwrap()
            .status,
        TurnStatus::Stopped
    );
    assert!(store.reader().get_pending_input().unwrap().is_some());

    let conn = Connection::open(path.db_path()).unwrap();
    let final_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM messages WHERE kind = 'assistant_final'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(final_count, 0);
    let stop_events: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM control_events WHERE event_type = 'stop_requested'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stop_events, 1);

    let repeated = store
        .writer()
        .request_stop_active_turn(&StopTurnRequest {
            instance_id: InstanceId::global_main(),
            requested_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(repeated.status, StopTurnStatus::NoActiveTurn);
}

#[test]
fn approval_first_response_wins_and_stop_invalidates_open_approval() {
    let path = temp_store_path("control-approval");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.approval").unwrap();
    create_session(&store, &session_id);
    acquire_turn(&store, &session_id, "turn.approval");

    let approval = store
        .writer()
        .create_approval(&CreateApprovalRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.one".to_string(),
            turn_id: "turn.approval".to_string(),
            scope: ApprovalScope::SingleToolCall,
            summary: "safe single action summary".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(approval.status, ApprovalStatus::Pending);

    let first = store
        .writer()
        .respond_approval_first_writer_wins(&ApprovalResponseRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.one".to_string(),
            response_id: "response.one".to_string(),
            decision: ApprovalDecision::Approve,
            responder_client_id: Some("client.one".to_string()),
            responder_client_kind: Some("tui".to_string()),
            responded_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(first.status, ApprovalResponseStatus::Accepted);
    assert_eq!(first.approval_status, ApprovalStatus::Approved);
    assert_eq!(first.resolved_response_id.as_deref(), Some("response.one"));

    let second = store
        .writer()
        .respond_approval_first_writer_wins(&ApprovalResponseRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.one".to_string(),
            response_id: "response.two".to_string(),
            decision: ApprovalDecision::Reject,
            responder_client_id: Some("client.two".to_string()),
            responder_client_kind: Some("web".to_string()),
            responded_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(
        second.status,
        ApprovalResponseStatus::RejectedAlreadyResolved
    );

    let conn = Connection::open(path.db_path()).unwrap();
    let attempts: i64 = conn
        .query_row("SELECT COUNT(*) FROM approval_responses", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(attempts, 2);
    let winner: Option<String> = conn
        .query_row(
            "SELECT resolved_response_id FROM approvals WHERE approval_id = 'approval.one'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(winner.as_deref(), Some("response.one"));
    let approval_status: String = conn
        .query_row(
            "SELECT status FROM approvals WHERE approval_id = 'approval.one'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(approval_status, "approved");
    let second_response_status: String = conn
        .query_row(
            "SELECT status FROM approval_responses WHERE response_id = 'response.two'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(second_response_status, "rejected_already_resolved");
    let turn_status_after_second: String = conn
        .query_row(
            "SELECT status FROM turns WHERE turn_id = 'turn.approval'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(turn_status_after_second, "running");

    store
        .writer()
        .create_approval(&CreateApprovalRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.two".to_string(),
            turn_id: "turn.approval".to_string(),
            scope: ApprovalScope::SingleToolCall,
            summary: "safe second action summary".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();
    store
        .writer()
        .request_stop_active_turn(&StopTurnRequest {
            instance_id: InstanceId::global_main(),
            requested_at: SystemClock.now(),
        })
        .unwrap();

    let stale = store
        .writer()
        .respond_approval_first_writer_wins(&ApprovalResponseRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.two".to_string(),
            response_id: "response.stale".to_string(),
            decision: ApprovalDecision::Approve,
            responder_client_id: None,
            responder_client_kind: None,
            responded_at: SystemClock.now(),
        })
        .unwrap();
    assert_eq!(stale.status, ApprovalResponseStatus::RejectedStale);
    assert_eq!(stale.approval_status, ApprovalStatus::InvalidatedByStop);
    let stale_status: String = conn
        .query_row(
            "SELECT status FROM approval_responses WHERE response_id = 'response.stale'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stale_status, "rejected_stale");
    let stopped_turn_status: String = conn
        .query_row(
            "SELECT status FROM turns WHERE turn_id = 'turn.approval'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stopped_turn_status, "stopped");
    let runtime_status: String = conn
        .query_row(
            "SELECT runtime_status FROM instance_runtime_state WHERE instance_id = 'global-main'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(runtime_status, "idle");
    let invalidated_status: String = conn
        .query_row(
            "SELECT status FROM approvals WHERE approval_id = 'approval.two'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(invalidated_status, "invalidated_by_stop");
}

#[test]
fn approval_response_cas_loser_is_not_accepted_and_does_not_reopen_turn() {
    let path = temp_store_path("control-approval-cas-loser");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.approval.cas").unwrap();
    create_session(&store, &session_id);
    acquire_turn(&store, &session_id, "turn.approval.cas");

    store
        .writer()
        .create_approval(&CreateApprovalRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.cas".to_string(),
            turn_id: "turn.approval.cas".to_string(),
            scope: ApprovalScope::SingleToolCall,
            summary: "safe cas action summary".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();

    let conn = Connection::open(path.db_path()).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER simulate_approval_cas_loss
         BEFORE UPDATE OF status ON approvals
         WHEN OLD.approval_id = 'approval.cas' AND OLD.status = 'pending'
         BEGIN
           UPDATE approvals
              SET status = 'approved',
                  resolved_at = 12345,
                  resolved_response_id = 'response.external'
            WHERE approval_id = OLD.approval_id;
           SELECT raise(ignore);
         END;",
    )
    .unwrap();
    drop(conn);

    let outcome = store
        .writer()
        .respond_approval_first_writer_wins(&ApprovalResponseRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.cas".to_string(),
            response_id: "response.loser".to_string(),
            decision: ApprovalDecision::Reject,
            responder_client_id: Some("client.loser".to_string()),
            responder_client_kind: Some("web".to_string()),
            responded_at: SystemClock.now(),
        })
        .unwrap();

    assert_eq!(
        outcome.status,
        ApprovalResponseStatus::RejectedAlreadyResolved
    );
    assert_eq!(outcome.approval_status, ApprovalStatus::Approved);
    assert_eq!(
        outcome.resolved_response_id.as_deref(),
        Some("response.external")
    );

    let conn = Connection::open(path.db_path()).unwrap();
    let winner: Option<String> = conn
        .query_row(
            "SELECT resolved_response_id FROM approvals WHERE approval_id = 'approval.cas'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(winner.as_deref(), Some("response.external"));
    let loser_status: String = conn
        .query_row(
            "SELECT status FROM approval_responses WHERE response_id = 'response.loser'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(loser_status, "rejected_already_resolved");
    let attempts: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM approval_responses WHERE approval_id = 'approval.cas'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(attempts, 1);
    let turn_status: String = conn
        .query_row(
            "SELECT status FROM turns WHERE turn_id = 'turn.approval.cas'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(turn_status, "waiting_approval");
    let runtime_status: String = conn
        .query_row(
            "SELECT runtime_status FROM instance_runtime_state WHERE instance_id = 'global-main'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(runtime_status, "waiting_approval");
}

#[test]
fn approval_creation_requires_active_turn() {
    let path = temp_store_path("control-approval-active-only");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.approval.active").unwrap();
    create_session(&store, &session_id);

    let result = store.writer().create_approval(&CreateApprovalRequest {
        instance_id: InstanceId::global_main(),
        approval_id: "approval.inactive".to_string(),
        turn_id: "turn.missing".to_string(),
        scope: ApprovalScope::SingleToolCall,
        summary: "safe summary".to_string(),
        created_at: SystemClock.now(),
    });
    assert!(result.unwrap_err().to_string().contains("active turn"));
}

#[test]
fn idle_pending_submit_does_not_start_turn() {
    let path = temp_store_path("control-idle-pending");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.idle.pending").unwrap();
    create_session(&store, &session_id);

    store
        .writer()
        .submit_or_replace_pending_input(&PendingInputSubmitRequest {
            instance_id: InstanceId::global_main(),
            pending_input_id: "pending.idle".to_string(),
            session_id: Some(session_id.as_str().to_string()),
            turn_id: None,
            content: "safe idle input".to_string(),
            submitted_at: SystemClock.now(),
        })
        .unwrap();

    let snapshot = store.reader().get_current_snapshot().unwrap();
    assert_eq!(snapshot.active_turn_id, None);
    assert_eq!(snapshot.pending_input_id.as_deref(), Some("pending.idle"));
    let conn = Connection::open(path.db_path()).unwrap();
    let turns: i64 = conn
        .query_row("SELECT COUNT(*) FROM turns", [], |row| row.get(0))
        .unwrap();
    assert_eq!(turns, 0);
}

#[test]
fn invalidating_open_approvals_updates_only_pending_rows() {
    let path = temp_store_path("control-invalidate-open");
    let store = open_store(path.db_path());
    let session_id = SessionId::new("sess.invalidate").unwrap();
    create_session(&store, &session_id);
    acquire_turn(&store, &session_id, "turn.invalidate");
    store
        .writer()
        .create_approval(&CreateApprovalRequest {
            instance_id: InstanceId::global_main(),
            approval_id: "approval.invalidate".to_string(),
            turn_id: "turn.invalidate".to_string(),
            scope: ApprovalScope::SingleToolCall,
            summary: "safe invalidation summary".to_string(),
            created_at: SystemClock.now(),
        })
        .unwrap();

    let updated = store
        .writer()
        .invalidate_open_approvals_for_turn(
            &InstanceId::global_main(),
            "turn.invalidate",
            ApprovalStatus::InvalidatedByTurnClose,
        )
        .unwrap();
    assert_eq!(updated, 1);

    let conn = Connection::open(path.db_path()).unwrap();
    let status: String = conn
        .query_row(
            "SELECT status FROM approvals WHERE approval_id = ?1",
            params!["approval.invalidate"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(status, "invalidated_by_turn_close");
}
