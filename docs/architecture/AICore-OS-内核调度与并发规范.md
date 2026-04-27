# AICore OS 内核调度与并发规范

## 多实例并行原则

内核调度以 instance 为主要隔离单位。多个有任务的 instance 可以并行执行，不使用单个全局轮片队列作为主执行模型。

## InstanceWorker

InstanceWorker 表达 instance 级工作所有权。同一 instance 的状态 mutation 通过 instance work ownership 串行化。

## ExecutionLane

ExecutionLane 表达工作运行 lane。State mutation 使用 instance lane，provider/tool/read 类工作可按策略进入并行 lane，memory write 使用 memory scope lane。

## WorkerLease

WorkerLease 表达一次工作对 lane 的占用。工作完成后必须释放租约。

## RunQueue

RunQueue 使用有界 FIFO 队列表达待执行工作。达到容量后，调度层返回 backpressure 错误。

## Backpressure

Backpressure 用于保护内核资源。队列满、资源预算不足或并发边界冲突时，调度层应返回结构化错误。

## Cancellation

Cancellation 支持 instance、conversation、turn 和 invocation 范围。取消信号通过可共享 token 传播。

## ResourceBudget

ResourceBudget 表达队列容量、并行读数量等资源限制。调度器根据预算决定接受、排队或拒绝工作。

## WorkItem 分类

WorkItem 至少区分 StateMutation、ProviderCall、ToolCall、MemoryRead 和 MemoryWrite。MemoryWrite 必须携带 memory scope。

## 同 instance / 同 conversation / 同 workspace / 同 memory scope 并发边界

同 instance 状态 mutation 串行。 同 conversation 默认一个 active turn。 同 workspace 的读类任务可并行。 同 memory scope 写入保持单写者。 MemoryRead 可并行。

## 事件顺序

调度事件应保留 instance_id 和 invocation_id。WorkStarted、WorkCompleted、InvocationFailed 等事件通过内核事件 envelope 进入事件总线。
