use super::support::*;

#[test]
fn memory_pack_respects_token_budget() {
    let mut kernel =
        MemoryKernel::open(temp_paths("memory-pack")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Permanent,
            scope: global_scope(),
            content: "非常重要的长期记忆".to_string(),
            localized_summary: "重要记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Working,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "次要工作记忆".to_string(),
            localized_summary: "次要工作记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let pack = build_memory_pack_for_tests(kernel.records(), 8);

    assert_eq!(pack.len(), 1);
    assert_eq!(pack[0].content, "非常重要的长期记忆");
}

#[test]
fn stage_status_is_not_current_instruction() {
    let mut kernel = MemoryKernel::open(temp_paths("status-not-instruction"))
        .expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "始终使用中文回复".to_string(),
            localized_summary: "中文回复".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Status,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "当前阶段".to_string(),
            localized_summary: "当前阶段".to_string(),
            state_key: Some("stage".to_string()),
            current_state: Some("P6.1".to_string()),
        })
        .expect("remember should succeed");

    let status = build_status_projection_for_tests(kernel.records());
    assert!(status.contains("P6.1"));
    assert!(!status.contains("始终使用中文回复"));
}

#[test]
fn safety_scan_blocks_secret_but_not_technical_discussion() {
    assert!(blocks_secret("api_key=sk-test-secret"));
    assert!(!blocks_secret(
        "这里讨论 api_key 命名规范和 secret storage 设计"
    ));
}

#[test]
fn audit_passes_for_remembered_memory() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-remember")).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "审计记忆".to_string(),
            localized_summary: "审计记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
    assert_eq!(report.checked_events, 1);
    assert!(report.issues.is_empty());
}

#[test]
fn audit_detects_accepted_event_without_record() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-missing-record")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "待删记录".to_string(),
            localized_summary: "待删记录".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .delete_record_for_tests(&memory_id)
        .expect("record deletion should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(!report.ok);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("accepted event") && issue.contains("missing memory"))
    );
}

#[test]
fn audit_passes_for_assistant_summary_proposal() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-proposal")).expect("memory kernel should open");

    kernel
        .submit_assistant_summary(global_scope(), "assistant summary proposal")
        .expect("assistant summary should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
    assert_eq!(report.checked_events, 1);
}

#[test]
fn audit_detects_proposed_event_without_proposal() {
    let mut kernel = MemoryKernel::open(temp_paths("audit-missing-proposal"))
        .expect("memory kernel should open");

    let proposal_id = kernel
        .submit_assistant_summary(global_scope(), "assistant summary proposal")
        .expect("assistant summary should succeed");
    kernel
        .delete_proposal_for_tests(&proposal_id)
        .expect("proposal deletion should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(!report.ok);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("proposed event") && issue.contains("missing proposal"))
    );
}

#[test]
fn audit_passes_for_user_correction_supersedes_edge() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-correction")).expect("memory kernel should open");

    let old_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "旧记忆".to_string(),
            localized_summary: "旧记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .correct_by_user_with_version(&old_id, 1, "新记忆")
        .expect("correct should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
}

#[test]
fn audit_detects_missing_supersedes_edge_for_correction() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-missing-edge")).expect("memory kernel should open");

    let old_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "旧记忆".to_string(),
            localized_summary: "旧记忆".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    let new_id = kernel
        .correct_by_user_with_version(&old_id, 1, "新记忆")
        .expect("correct should succeed");
    kernel
        .delete_edge_for_tests(&new_id, &old_id, "supersedes")
        .expect("edge deletion should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(!report.ok);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("missing supersedes edge"))
    );
}

#[test]
fn audit_passes_for_archived_memory() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-archived")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "待归档".to_string(),
            localized_summary: "待归档".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .archive_with_version(&memory_id, 1)
        .expect("archive should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
}

#[test]
fn audit_detects_archived_event_without_archived_record() {
    let mut kernel = MemoryKernel::open(temp_paths("audit-archived-mismatch"))
        .expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "待归档".to_string(),
            localized_summary: "待归档".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .archive_with_version(&memory_id, 1)
        .expect("archive should succeed");
    kernel
        .force_record_status_for_tests(&memory_id, MemoryStatus::Active)
        .expect("status override should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(!report.ok);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("archived event") && issue.contains("non-archived"))
    );
}

#[test]
fn audit_passes_for_forgotten_memory() {
    let mut kernel =
        MemoryKernel::open(temp_paths("audit-forgotten")).expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "待遗忘".to_string(),
            localized_summary: "待遗忘".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .forget_with_version(&memory_id, 1)
        .expect("forget should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
}

#[test]
fn audit_detects_forgotten_event_without_forgotten_record() {
    let mut kernel = MemoryKernel::open(temp_paths("audit-forgotten-mismatch"))
        .expect("memory kernel should open");

    let memory_id = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "待遗忘".to_string(),
            localized_summary: "待遗忘".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("remember should succeed");
    kernel
        .forget_with_version(&memory_id, 1)
        .expect("forget should succeed");
    kernel
        .force_record_status_for_tests(&memory_id, MemoryStatus::Active)
        .expect("status override should succeed");

    let report = kernel.verify_ledger_consistency();
    assert!(!report.ok);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.contains("forgotten event") && issue.contains("non-forgotten"))
    );
}

#[test]
fn manual_lock_blocks_second_writer() {
    let paths = temp_paths("manual-lock-blocks");
    write_lock_file(&paths, "9999999999", "remember_user_explicit");
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    let error = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "被锁阻塞".to_string(),
            localized_summary: "被锁阻塞".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect_err("active lock should block writer");

    assert!(error.0.contains("memory write locked"));
}

#[test]
fn stale_manual_lock_allows_recovery() {
    let paths = temp_paths("stale-lock-recovery");
    write_lock_file(&paths, "0", "remember_user_explicit");
    let lock_path = paths.lock_path.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "stale lock recovery".to_string(),
            localized_summary: "stale lock recovery".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("stale lock should be recoverable");

    assert!(!lock_path.exists());
    assert_eq!(kernel.records().len(), 1);
}

#[test]
fn write_lock_is_released_after_projection_failure() {
    let paths = temp_paths("lock-release-projection-failure");
    let lock_path = paths.lock_path.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
    kernel.set_projection_failure_for_tests(true);

    kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "projection failure".to_string(),
            localized_summary: "projection failure".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect("write should still succeed");

    assert!(!lock_path.exists());
}

#[test]
fn write_lock_is_released_after_storage_error() {
    let paths = temp_paths("lock-release-storage-failure");
    let lock_path = paths.lock_path.clone();
    let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
    kernel.set_write_failure_for_tests(true);

    let error = kernel
        .remember_user_explicit(RememberInput {
            memory_type: MemoryType::Core,
            permanence: MemoryPermanence::Standard,
            scope: global_scope(),
            content: "storage failure".to_string(),
            localized_summary: "storage failure".to_string(),
            state_key: None,
            current_state: None,
        })
        .expect_err("injected write failure should fail");

    assert!(error.0.contains("write failure injected for tests"));
    assert!(!lock_path.exists());
}

#[test]
fn concurrent_remember_calls_do_not_reuse_ids() {
    let paths = temp_paths("concurrent-remember");
    let mut handles = Vec::new();

    for index in 0..8 {
        let paths = paths.clone();
        handles.push(thread::spawn(move || {
            let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
            kernel
                .remember_user_explicit(RememberInput {
                    memory_type: MemoryType::Core,
                    permanence: MemoryPermanence::Standard,
                    scope: global_scope(),
                    content: format!("concurrent remember {index}"),
                    localized_summary: format!("concurrent remember {index}"),
                    state_key: None,
                    current_state: None,
                })
                .expect("remember should succeed")
        }));
    }

    let mut ids = Vec::new();
    for handle in handles {
        ids.push(handle.join().expect("thread should finish"));
    }

    let unique: HashSet<_> = ids.iter().cloned().collect();
    assert_eq!(ids.len(), unique.len());

    let kernel = MemoryKernel::open(paths).expect("memory kernel should reopen");
    assert_eq!(kernel.records().len(), ids.len());
}

#[test]
fn memory_audit_passes_after_concurrent_writes() {
    let paths = temp_paths("concurrent-audit");
    let mut handles = Vec::new();

    for index in 0..6 {
        let paths = paths.clone();
        handles.push(thread::spawn(move || {
            let mut kernel = MemoryKernel::open(paths).expect("memory kernel should open");
            kernel
                .remember_user_explicit(RememberInput {
                    memory_type: MemoryType::Core,
                    permanence: MemoryPermanence::Standard,
                    scope: global_scope(),
                    content: format!("audit concurrent {index}"),
                    localized_summary: format!("audit concurrent {index}"),
                    state_key: None,
                    current_state: None,
                })
                .expect("remember should succeed");
        }));
    }

    for handle in handles {
        handle.join().expect("thread should finish");
    }

    let kernel = MemoryKernel::open(paths).expect("memory kernel should reopen");
    let report = kernel.verify_ledger_consistency();
    assert!(report.ok);
    assert_eq!(report.checked_events, 6);
}
