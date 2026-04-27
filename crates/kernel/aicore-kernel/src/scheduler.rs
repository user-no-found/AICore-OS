use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use aicore_foundation::{AicoreError, BoundedQueue, CancellationToken};

use crate::{KernelEventEnvelope, KernelEventType, Visibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkItemKind {
    StateMutation,
    ProviderCall,
    ToolCall,
    MemoryRead { scope: String },
    MemoryWrite { scope: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItem {
    pub invocation_id: String,
    pub instance_id: String,
    pub conversation_id: Option<String>,
    pub kind: WorkItemKind,
}

impl WorkItem {
    pub fn new(
        invocation_id: impl Into<String>,
        instance_id: impl Into<String>,
        kind: WorkItemKind,
    ) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            instance_id: instance_id.into(),
            conversation_id: None,
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionLane {
    InstanceState(String),
    ProviderRead(String),
    ToolRead(String),
    MemoryRead(String),
    MemoryWrite(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackpressurePolicy {
    RejectWhenFull,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceBudget {
    pub max_queue_len: usize,
    pub max_parallel_reads: usize,
}

impl ResourceBudget {
    pub fn new(max_queue_len: usize, max_parallel_reads: usize) -> Self {
        Self {
            max_queue_len,
            max_parallel_reads,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancellationScope {
    Instance(String),
    Conversation(String),
    Turn(String),
    Invocation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerLease {
    pub invocation_id: String,
    pub lane: ExecutionLane,
    released: bool,
}

impl WorkerLease {
    pub fn new(invocation_id: impl Into<String>, lane: ExecutionLane) -> Self {
        Self {
            invocation_id: invocation_id.into(),
            lane,
            released: false,
        }
    }

    pub fn release(&mut self) {
        self.released = true;
    }

    pub fn is_released(&self) -> bool {
        self.released
    }
}

pub type RunQueue = BoundedQueue<WorkItem>;

#[derive(Debug, Clone)]
pub struct InstanceRuntimeRegistry {
    instances: Arc<Mutex<HashSet<String>>>,
}

impl InstanceRuntimeRegistry {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn register(&self, instance_id: impl Into<String>) {
        self.instances
            .lock()
            .expect("registry poisoned")
            .insert(instance_id.into());
    }
}

impl Default for InstanceRuntimeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct InstanceWorkerPool;

#[derive(Debug, Clone)]
pub struct AppInvocationDispatcher;

#[derive(Debug, Clone)]
pub struct ExecutionLanePool {
    active_conversations: Arc<Mutex<HashSet<String>>>,
    memory_writers: Arc<Mutex<HashSet<String>>>,
}

impl ExecutionLanePool {
    pub fn new() -> Self {
        Self {
            active_conversations: Arc::new(Mutex::new(HashSet::new())),
            memory_writers: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn lane_for(&self, item: &WorkItem) -> Result<ExecutionLane, AicoreError> {
        if let Some(conversation_id) = &item.conversation_id {
            let mut active = self
                .active_conversations
                .lock()
                .expect("conversation set poisoned");
            if !active.insert(conversation_id.clone()) {
                return Err(AicoreError::Conflict(format!(
                    "active conversation: {conversation_id}"
                )));
            }
        }

        match &item.kind {
            WorkItemKind::StateMutation => {
                Ok(ExecutionLane::InstanceState(item.instance_id.clone()))
            }
            WorkItemKind::ProviderCall => Ok(ExecutionLane::ProviderRead(item.instance_id.clone())),
            WorkItemKind::ToolCall => Ok(ExecutionLane::ToolRead(item.instance_id.clone())),
            WorkItemKind::MemoryRead { scope } => Ok(ExecutionLane::MemoryRead(scope.clone())),
            WorkItemKind::MemoryWrite { scope } => {
                let mut writers = self.memory_writers.lock().expect("writer set poisoned");
                if !writers.insert(scope.clone()) {
                    return Err(AicoreError::Conflict(format!(
                        "memory writer scope: {scope}"
                    )));
                }
                Ok(ExecutionLane::MemoryWrite(scope.clone()))
            }
        }
    }

    pub fn release(&self, item: &WorkItem) {
        if let Some(conversation_id) = &item.conversation_id {
            self.active_conversations
                .lock()
                .expect("conversation set poisoned")
                .remove(conversation_id);
        }
        if let WorkItemKind::MemoryWrite { scope } = &item.kind {
            self.memory_writers
                .lock()
                .expect("writer set poisoned")
                .remove(scope);
        }
    }
}

impl Default for ExecutionLanePool {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BackpressureManager {
    queue: Arc<Mutex<RunQueue>>,
}

impl BackpressureManager {
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: Arc::new(Mutex::new(RunQueue::new(capacity))),
        }
    }

    pub fn push(&self, item: WorkItem) -> Result<(), AicoreError> {
        self.queue.lock().expect("queue poisoned").push(item)
    }
}

#[derive(Debug, Clone)]
pub struct CancellationRegistry {
    tokens: Arc<Mutex<HashMap<String, CancellationToken>>>,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn token_for_invocation(&self, invocation_id: impl Into<String>) -> CancellationToken {
        let invocation_id = invocation_id.into();
        let mut tokens = self.tokens.lock().expect("cancellation registry poisoned");
        tokens.entry(invocation_id).or_default().clone()
    }

    pub fn cancel(&self, scope: CancellationScope) {
        if let CancellationScope::Invocation(invocation_id) = scope {
            self.token_for_invocation(invocation_id).cancel();
        }
    }
}

impl Default for CancellationRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ResourceBudgetManager {
    budget: ResourceBudget,
}

impl ResourceBudgetManager {
    pub fn new(budget: ResourceBudget) -> Self {
        Self { budget }
    }

    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
}

#[derive(Debug, Clone)]
pub struct KernelScheduler {
    lanes: ExecutionLanePool,
    events: Arc<Mutex<Vec<KernelEventEnvelope>>>,
}

impl KernelScheduler {
    pub fn new() -> Self {
        Self {
            lanes: ExecutionLanePool::new(),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn assign_lane(&self, item: &WorkItem) -> Result<ExecutionLane, AicoreError> {
        self.lanes.lane_for(item)
    }

    pub fn release(&self, item: &WorkItem) {
        self.lanes.release(item);
    }

    pub fn run_async<F>(&self, item: WorkItem, work: F) -> Result<JoinHandle<()>, AicoreError>
    where
        F: FnOnce(WorkerLease) + Send + 'static,
    {
        let lane = self.assign_lane(&item)?;
        let events = Arc::clone(&self.events);
        let lanes = self.lanes.clone();
        let lease = WorkerLease::new(item.invocation_id.clone(), lane);
        let item_for_thread = item.clone();

        let handle = thread::spawn(move || {
            events
                .lock()
                .expect("event bus poisoned")
                .push(KernelEventEnvelope::new(
                    format!("event.{}", item_for_thread.invocation_id),
                    KernelEventType::WorkStarted,
                    item_for_thread.instance_id.clone(),
                    "kernel.scheduler",
                    item_for_thread.invocation_id.clone(),
                    Visibility::Internal,
                ));
            work(lease);
            lanes.release(&item_for_thread);
        });

        Ok(handle)
    }

    pub fn events(&self) -> Vec<KernelEventEnvelope> {
        self.events.lock().expect("event bus poisoned").clone()
    }
}

impl Default for KernelScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier, mpsc};
    use std::time::Duration;

    use super::{
        BackpressureManager, CancellationRegistry, CancellationScope, ExecutionLane,
        KernelScheduler, WorkItem, WorkItemKind, WorkerLease,
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
}
