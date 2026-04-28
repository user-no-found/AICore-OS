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
mod tests;
