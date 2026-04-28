# AICore OS 内核协议规范

## 职责

内核协议定义应用、能力、调用、事件、路由、错误、版本、权限、审计和 trace 的稳定合同。应用通过内核协议接入系统，不直接依赖内核内部源码边界。

## 编译整体

内核编译整体是 `crates/kernel/aicore-kernel`。应用层通过 `aicore-kernel` 使用内核公开类型和运行时合同。

## AppManifest

`AppManifest` 描述应用 ID、运行时类型、显示名称、合同版本、能力声明和权限边界。应用 ID 必须稳定，运行时类型用于区分 CLI、TUI、Web、Provider、Toolset、Gateway 和 Service 等形态。

## AppHandshake

`AppHandshake` 是应用启动或注册时交给内核的握手信息。它声明应用支持的合同版本和能力描述。

## CapabilityDescriptor

`CapabilityDescriptor` 描述能力 ID、操作列表、schema 引用、凭证要求和 sandbox 要求。能力操作使用英文机器名。

## KernelInvocationEnvelope

`KernelInvocationEnvelope` 是内核调用应用能力的标准 envelope。它必须携带 instance_id、capability、operation、payload、policy、trace_context 和 audit_context。

## Invocation Dispatcher

调用分发器接收 `KernelInvocationEnvelope`，先进入 route decision runtime，获得 `KernelRouteDecision` 后再查找 handler registry。handler registry 可以提供本进程内 handler，用于 smoke、测试和受控内核边界验证。

分发器必须保证 route failure 不执行 handler。route 成功但 handler 不存在时返回 handler lookup failure。handler 执行失败时返回 handler execute failure。成功执行时返回 `KernelEventEnvelope`，事件类型使用 invocation completed，事件 payload 只能包含安全摘要。

本进程 handler registry 不代表真实组件进程执行。它不得启动组件进程，不得打开 socket IPC，不得调用 provider adapter，不得执行 tool，也不得把 raw payload、raw provider request、raw secret、`secret_ref` 或 `credential_lease_ref` 暴露到 public surface。

event ledger 与 invocation ledger 是持久审计能力，不属于最小本进程 handler registry 的必要条件。未启用 ledger 时，public surface 必须明确 ledger 未追加。

## First-party Read-only Handler Boundary

一方只读 handler 是由 AICore OS 自身提供的受控 read-only adapter，用于读取全局运行时状态、配置路径状态、内核状态等安全摘要。它必须先通过 installed manifest registry 产生 route decision，再经由 `KernelInvocationRuntime` 执行，不得绕过 `KernelRouteRuntime` 直接调用。

一方只读 handler 可以作为本进程 adapter 存在，但它不是最终组件进程模型。它不得启动组件进程，不得进行跨进程调用，不得打开 socket IPC，不得调用 provider adapter，不得执行 tool，不得修改业务状态。

一方只读 handler 的 public surface 只能输出结构化摘要，例如 operation、route metadata、invocation id、handler status、ledger status、runtime installed status 和计数类信息。它不得输出 raw `KernelInvocationEnvelope.payload`、raw config、raw secret、`secret_ref`、`credential_lease_ref`、raw provider request、raw provider payload、raw tool input/output、API key、token 或 cookie。

启用 invocation ledger 时，一方只读 handler 的成功路径必须记录 accepted、route decision、handler execution、event generation 和 invocation completion。同一次 invocation 的所有 ledger records 与生成的 `KernelEventEnvelope` 必须共享同一个 `invocation_id`。

## Invocation Ledger

`invocation-ledger.jsonl` 是内核调用生命周期的 append-only audit ledger。它记录 invocation 被接受、路由、handler 查找、handler 执行、事件生成、调用完成和调用失败等审计事实。它不是业务事实源，不参与恢复 component state，不承担 event sourcing、conversation store、query、replay 或 compaction。

invocation ledger 使用 JSON Lines。每行是一条独立 record，schema version 固定为 `aicore.kernel.invocation_ledger.v1`。record 至少表达 record_id、timestamp、invocation_id、trace_id、instance_id、operation、stage、status、route metadata、failure metadata、handler metadata、handler_executed、event_generated、spawned_process 和 called_real_component。

`invocation_id` 标识一次具体 invocation。每次 `KernelInvocationEnvelope` 创建时必须分配唯一 `invocation_id`；同一次 invocation 生成的所有 ledger records 与对应 `KernelEventEnvelope` 必须使用同一个 `invocation_id`。`operation` 不能作为 invocation 唯一性来源。

允许的 stage 包括 `accepted`、`route_decision_made`、`route_failed`、`handler_lookup_failed`、`handler_failed`、`handler_executed`、`event_generated`、`invocation_completed` 和 `invocation_failed`。`handler_executed` 只在 handler 成功执行后记录；handler 返回错误时记录 `handler_failed`；`event_generated` 只在实际生成 `KernelEventEnvelope` 后记录；`invocation_completed` 只在审计闭合成功后记录。

ledger append failure 必须返回结构化失败，不能伪装成 invocation completed。`accepted` record 写入失败时不得继续 route 或执行 handler。handler 已执行且 event 已生成后，如果 `invocation_completed` 写入失败，public surface 必须表达动作已经发生但审计闭合失败。

invocation ledger 不得记录 raw `KernelInvocationEnvelope.payload`、raw provider request、raw provider payload、raw tool input/output、raw memory content、raw secret、`secret_ref`、`credential_lease_ref`、API key、token 或 cookie。failure_reason 写入 ledger 前必须转换成 redacted summary。

## KernelEventEnvelope

`KernelEventEnvelope` 是内核事件标准 envelope。它必须携带 event_id、event_type、instance_id、app_id、invocation_id、visibility、payload 和 trace_context。

## KernelRouteRequest

`KernelRouteRequest` 表达一次能力路由请求。它携带 instance、capability、operation、可选合同版本要求、trace context 和 audit context。

## KernelRouteDecision

`KernelRouteDecision` 表达路由结果。它携带目标 app、目标合同版本、路由策略、路由原因和 fallback chain。

## Runtime Route Decision

运行时路由决策以 installed component manifest registry 作为输入源。内核从 `$HOME/.aicore/share/manifests/*.toml` 读取 component manifest，构建 operation 到 capability 的映射，再生成 `KernelRouteRequest` 并产出 `KernelRouteDecision`。

route decision 只决定目标 component、app、capability、operation 与 contract version，不执行 handler，不启动进程，不写 invocation ledger，也不写 event ledger。

当 operation 不存在时，route runtime 返回 missing capability。多个 component 声明同一个 operation 时，route runtime 返回 ambiguous route，不静默选择其中一个。contract id 或 major version 不兼容时，route runtime 返回 contract version mismatch。

route decision 的 public surface 不得暴露 raw secret、`secret_ref`、`credential_lease_ref`、raw provider request、raw provider payload 或 internal handler request。

## KernelError

`KernelError` 包含机器错误码、错误阶段、中文用户消息、安全详情、重试提示和 secret-safe 标记。错误不得携带 raw secret 或内部 provider request。

## ContractVersion

`ContractVersion` 使用 contract_id、major 和 minor 表达合同版本。兼容范围通过合同 ID 与 major 范围判定。

## PermissionBoundary

`PermissionBoundary` 描述应用或组件的权限范围和能力边界。需要审批的能力必须显式标记。

## AuditContext

`AuditContext` 描述调用的发起者和原因。高影响调用必须带有可审计原因。

## TraceContext

`TraceContext` 描述 trace_id 和可选父 span。跨应用、跨组件调用应保持 trace 连续。
