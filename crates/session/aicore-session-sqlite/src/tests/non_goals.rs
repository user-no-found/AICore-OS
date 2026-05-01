use aicore_session::traits::SessionLedger;

use crate::tests::open_store;

#[test]
fn unsupported_pending_input_returns_unavailable() {
    let path = super::temp_store_path("unsupported-pending-input");
    let store = open_store(path.db_path());

    let result = store.writer().create_pending_input();
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("pending_inputs not implemented yet")
            || msg.contains("create_pending_input not implemented yet")
    );
}

#[test]
fn unsupported_approval_returns_unavailable() {
    let path = super::temp_store_path("unsupported-approval");
    let store = open_store(path.db_path());

    let result = store.writer().submit_approval();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("approvals not implemented yet")
            || msg.contains("submit_approval not implemented yet")
    );
}

#[test]
fn unsupported_approval_response_returns_unavailable() {
    let path = super::temp_store_path("unsupported-approval-resp");
    let store = open_store(path.db_path());

    let result = store.writer().respond_approval();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("approval_responses not implemented yet")
            || msg.contains("respond_approval not implemented yet")
    );
}

#[test]
fn unsupported_read_pending_inputs_returns_unavailable() {
    let path = super::temp_store_path("unsupported-read-pending");
    let store = open_store(path.db_path());

    let result = store.reader().read_pending_inputs();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("pending_inputs not implemented yet")
            || msg.contains("read_pending_inputs not implemented yet")
    );
}

#[test]
fn unsupported_read_approvals_returns_unavailable() {
    let path = super::temp_store_path("unsupported-read-approval");
    let store = open_store(path.db_path());

    let result = store.reader().read_approvals();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("approvals not implemented yet")
            || msg.contains("read_approvals not implemented yet")
    );
}

#[test]
fn unsupported_read_approval_responses_returns_unavailable() {
    let path = super::temp_store_path("unsupported-read-approval-resp");
    let store = open_store(path.db_path());

    let result = store.reader().read_approval_responses();
    assert!(result.is_err());
    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("approval_responses not implemented yet")
            || msg.contains("read_approval_responses not implemented yet")
    );
}
