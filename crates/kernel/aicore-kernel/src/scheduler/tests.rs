use std::sync::{Arc, Barrier, mpsc};
use std::time::Duration;

use super::{
    BackpressureManager, CancellationRegistry, CancellationScope, ExecutionLane, KernelScheduler,
    WorkItem, WorkItemKind, WorkerLease,
};

#[test]
fn scheduler_assigns_distinct_lanes_to_distinct_instances() {
    let scheduler = KernelScheduler::new();
    let left = scheduler
        .assign_lane(&WorkItem::new(
            "invoke.1",
            "inst-a",
            WorkItemKind::StateMutation,
        ))
        .expect("lane should assign");
    let right = scheduler
        .assign_lane(&WorkItem::new(
            "invoke.2",
            "inst-b",
            WorkItemKind::StateMutation,
        ))
        .expect("lane should assign");

    assert_ne!(left, right);
}

#[test]
fn multiple_instances_start_work_without_global_serial_loop() {
    let scheduler = KernelScheduler::new();
    let barrier = Arc::new(Barrier::new(3));
    let (tx, rx) = mpsc::channel();

    for (invocation, instance) in [("invoke.1", "inst-a"), ("invoke.2", "inst-b")] {
        let barrier = Arc::clone(&barrier);
        let tx = tx.clone();
        scheduler
            .run_async(
                WorkItem::new(invocation, instance, WorkItemKind::StateMutation),
                move |_lease| {
                    tx.send(instance.to_string())
                        .expect("started send should pass");
                    barrier.wait();
                },
            )
            .expect("work should start");
    }

    let first = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("first instance should start");
    let second = rx
        .recv_timeout(Duration::from_millis(500))
        .expect("second instance should start");
    assert_ne!(first, second);
    barrier.wait();
}

#[test]
fn same_conversation_rejects_second_active_turn() {
    let scheduler = KernelScheduler::new();
    let mut first = WorkItem::new("invoke.1", "inst-a", WorkItemKind::StateMutation);
    first.conversation_id = Some("conv-a".to_string());
    let mut second = WorkItem::new("invoke.2", "inst-a", WorkItemKind::StateMutation);
    second.conversation_id = Some("conv-a".to_string());

    scheduler
        .assign_lane(&first)
        .expect("first turn should start");
    let error = scheduler
        .assign_lane(&second)
        .expect_err("second active turn should fail");

    assert!(error.to_string().contains("active conversation"));
}

#[test]
fn backpressure_rejects_when_run_queue_is_full() {
    let backpressure = BackpressureManager::new(1);
    backpressure
        .push(WorkItem::new(
            "invoke.1",
            "inst-a",
            WorkItemKind::ProviderCall,
        ))
        .expect("first item should fit");

    let error = backpressure
        .push(WorkItem::new(
            "invoke.2",
            "inst-a",
            WorkItemKind::ProviderCall,
        ))
        .expect_err("second item should fail");

    assert!(error.to_string().contains("queue full"));
}

#[test]
fn cancellation_registry_cancels_invocation_scope() {
    let registry = CancellationRegistry::new();
    let token = registry.token_for_invocation("invoke.1");

    registry.cancel(CancellationScope::Invocation("invoke.1".to_string()));

    assert!(token.is_cancelled());
}

#[test]
fn memory_write_work_requires_single_writer_scope() {
    let scheduler = KernelScheduler::new();
    let first = WorkItem::new(
        "invoke.1",
        "inst-a",
        WorkItemKind::MemoryWrite {
            scope: "global-main".to_string(),
        },
    );
    let second = WorkItem::new(
        "invoke.2",
        "inst-b",
        WorkItemKind::MemoryWrite {
            scope: "global-main".to_string(),
        },
    );

    scheduler
        .assign_lane(&first)
        .expect("first writer should acquire");
    let error = scheduler
        .assign_lane(&second)
        .expect_err("second writer should fail");

    assert!(error.to_string().contains("memory writer scope"));
}

#[test]
fn memory_read_work_allows_parallel_scope() {
    let scheduler = KernelScheduler::new();
    let left = scheduler
        .assign_lane(&WorkItem::new(
            "invoke.1",
            "inst-a",
            WorkItemKind::MemoryRead {
                scope: "global-main".to_string(),
            },
        ))
        .expect("first reader should acquire");
    let right = scheduler
        .assign_lane(&WorkItem::new(
            "invoke.2",
            "inst-b",
            WorkItemKind::MemoryRead {
                scope: "global-main".to_string(),
            },
        ))
        .expect("second reader should acquire");

    assert_eq!(left, ExecutionLane::MemoryRead("global-main".to_string()));
    assert_eq!(right, ExecutionLane::MemoryRead("global-main".to_string()));
}

#[test]
fn worker_lease_releases_when_work_completes() {
    let mut lease = WorkerLease::new(
        "invoke.1",
        ExecutionLane::InstanceState("inst-a".to_string()),
    );
    assert!(!lease.is_released());

    lease.release();

    assert!(lease.is_released());
}

#[test]
fn event_bus_preserves_instance_and_invocation_ids() {
    let scheduler = KernelScheduler::new();
    let handle = scheduler
        .run_async(
            WorkItem::new("invoke.1", "inst-a", WorkItemKind::ProviderCall),
            |_| {},
        )
        .expect("work should run");
    handle.join().expect("worker should finish");

    let events = scheduler.events();
    assert_eq!(events[0].instance_id, "inst-a");
    assert_eq!(events[0].invocation_id, "invoke.1");
}
