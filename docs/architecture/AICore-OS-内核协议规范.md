# AICore OS 内核协议规范

## 职责

内核协议定义应用、能力、调用、事件、路由、错误、版本、权限、审计和 trace 的稳定合同。应用通过内核协议接入系统，不直接依赖内核内部源码边界。

## 编译整体

内核编译整体是 `crates/kernel/aicore-kernel`。应用层通过 `aicore-kernel` 使用内核公开类型和运行时合同。

## Runtime Binary Boundary

内核运行时 binary 是 `$HOME/.aicore/bin/aicore-kernel`。应用 public path 调用 kernel-native capability 时，应优先通过已安装的 kernel runtime binary 通讯，而不是静默使用应用私有 in-process kernel。

第一层本地协议是 one-shot `stdio_jsonl`。调用方把 `KernelInvocationEnvelope` 派生的 JSON request 写入 kernel runtime binary 的 stdin；kernel runtime binary 从 installed manifest registry 路由 operation，执行受控 invocation runtime，写入 invocation ledger，并在 stdout 输出 `kernel.invocation.result` JSON Lines event。stdout 是协议通道，不承载 human panel；stderr 只作为 diagnostic source，进入 public surface 前必须脱敏和截断。

`aicore-foundation` runtime binary 必须存在，kernel runtime binary 才能作为 AICore OS runtime boundary 正常服务。缺少 foundation 或 kernel runtime binary 时，调用方必须返回结构化失败，不得 fallback 成 in-process success。

当 route metadata 指向 `local_process` capability 时，Kernel runtime binary 是 process boundary owner。它负责 route decision、进程启动、stdio JSONL IPC、result envelope 生成与 invocation ledger 写入。应用侧 client 只传入 invocation request 并消费结构化 result，不直接执行 `LocalProcess` branch。

当前 runtime binary boundary 不是 daemon，不是 socket IPC server，不是 component supervisor，也不启动长期 kernel server。后续 socket、supervision、长期进程管理必须通过独立协议边界扩展。

### Runtime Binary Protocol

`stdio_jsonl` runtime binary protocol 使用一行 JSON request 对应一行 JSON result event。request 必须声明 schema、protocol、protocol version、contract version、request id、invocation id、trace id、instance id、capability、operation、payload summary 和 ledger path。payload summary 只能是安全摘要，不得携带 raw invocation payload。

当前协议常量：

```text
protocol = "stdio_jsonl"
protocol_version = "aicore.kernel.runtime_binary.stdio_jsonl.v1"
request_schema_version = "aicore.kernel.runtime_binary.request.v1"
response_schema_version = "aicore.kernel.runtime_binary.response.v1"
contract_version = "kernel.runtime.v1"
```

stdout 只能输出 JSON Lines protocol event。`kernel.invocation.result` event 必须包含 response schema、protocol、protocol version、contract version 和 payload。payload 是 `KernelInvocationResultEnvelope` 的 public JSON 表达。JSON mode consumer 必须读取该结构化 payload，不得解析 human summary。

stderr 只用于 diagnostic。stderr 进入 public surface 或 failure reason 前必须去除不安全控制字符、脱敏并截断。runtime binary client 和 ledger 都不得保存 raw stdout、raw stderr 或 raw protocol request。

exit code 语义：

- `0` 表示 runtime binary 成功输出一个 protocol-compatible result event，且 invocation status 为 completed。
- 非零表示 runtime binary 或 invocation 返回失败。调用方必须优先解析结构化 result event；无法解析时返回 non-zero exit 或 invalid JSONL output 结构化 failure。
- malformed JSONL、request schema mismatch、protocol mismatch、protocol version mismatch 和 contract version mismatch 都是结构化 protocol failure。

`invocation_id` 标识一次调用。request、result envelope、event envelope 和 invocation ledger records 必须共享同一个 `invocation_id`。`request_id` 用于 binary protocol request 关联，不替代 invocation id。

### Runtime Binary Client

应用侧 runtime binary client 必须显式接收或解析 installed kernel binary path 与 foundation binary path。client 不得在 public installed path 中 silent fallback 到 in-process kernel。

client 必须结构化区分以下失败：

- foundation runtime binary missing
- foundation runtime binary not executable
- kernel runtime binary missing
- kernel runtime binary not executable
- process spawn failure
- stdin write failure
- stdout read failure
- non-zero exit
- invalid JSONL output
- protocol version mismatch
- contract version mismatch
- kernel returned invocation failure

这些失败必须进入 public result payload 的 failure stage / reason，并保持 `in_process_fallback = false`。failure reason 是 redacted summary，不得包含 raw payload、raw secret、raw child output 或 internal request。

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

## Component Process Boundary

component process boundary 是 `KernelInvocationRuntime` 调用独立 application binary 的本地边界。route metadata 必须显式表达 `invocation_mode` 与 `transport`。`in_process` capability 走本进程 handler registry；`local_process` capability 进入本地组件进程调用分支。

当前最小 local IPC transport 是 `stdio_jsonl`。调用运行时通过 stdin 写入一行 JSON invocation request，通过 stdout 读取一行 JSON result。stdout 只作为 protocol channel 使用，不承载 human log。stderr 可以作为 diagnostic source，但进入 public surface 或 ledger 前必须脱敏、截断并去除不安全终端控制序列。

local process 成功结果必须进入 `KernelInvocationResultEnvelope`，并生成 `KernelEventEnvelope`。失败结果必须结构化表达 failure stage、redacted reason、transport、spawned process 和 exit code 等 metadata。component handler 的业务失败可以用 protocol-compatible `status = "failed"` result 表达；此时 invocation status 必须是 failed，且可以保留 safe public fields 供机器消费。

unsupported transport、缺失 entrypoint、spawn failure、IPC write/read failure、nonzero exit 与 invalid JSON result 都必须返回结构化 failure，不得静默回落为 in-process handler。

invocation ledger 应记录 local process metadata，例如 handler kind、spawned process、transport 和 process exit code。ledger 仍只记录审计 metadata，不记录 raw stdout、raw stderr、raw result payload 或 raw request。

应用通过 runtime binary client 触发 local process diagnostic invocation 时，public surface 必须表达 `kernel_invocation_path = binary` 或等价字段，并保持 `in_process_fallback = false`。缺少 foundation 或 kernel runtime binary 时，failure stage 必须结构化表达对应 binary 缺失或不可执行状态。

业务只读 operation 可以采用同一条 local process invocation 边界。`config.validate` 是配置校验只读 capability：route decision 来自 installed manifest，Kernel runtime binary 负责启动 component process，component process 通过 stdout 返回 `KernelInvocationResultEnvelope` 可消费字段。`auth.list`、`model.show`、`service.list`、`runtime.smoke`、`instance.list`、`cli.status`、`provider.smoke`、`agent.smoke`、`agent.session_smoke` 与 memory read operations 也是只读或 smoke component process capability；它们返回结构化 public fields，并继续禁止 raw config、secret、`secret_ref`、credential material、raw provider request、raw provider payload、full prompt 或 raw memory content 进入 ledger。direct `aicore-cli config validate`、`aicore-cli auth list`、`aicore-cli model show`、`aicore-cli service list`、`aicore-cli runtime smoke`、`aicore-cli instance list`、`aicore-cli status`、`aicore-cli provider smoke`、`aicore-cli agent smoke`、`aicore-cli agent session-smoke` 与 direct memory read commands 可以作为兼容本地命令保留，但不能被标记为 kernel-native invocation。

Memory read operations 包括 `memory.status`、`memory.search`、`memory.proposals`、`memory.audit`、`memory.wiki` 与 `memory.wiki_page`。这些 operation 的 public result 可以按现有 read surface 语义展示记忆摘要、搜索结果、proposal 摘要或 wiki markdown；invocation ledger 仍只记录 lifecycle、route、handler 和 failure metadata，不记录 raw memory content、wiki markdown、search result raw content、raw stdout、raw stderr 或 raw invocation payload。Wiki projection result 必须保留 not truth source 声明，wiki page request 必须继续执行 page 白名单与 path traversal 拒绝。

Memory write operations 包括 `memory.remember`、`memory.accept` 与 `memory.reject`。这些 operation 可以通过 `kernel invoke-write` 类入口进入 installed Kernel runtime binary、installed manifest route、`local_process` component handler 和 `KernelInvocationResultEnvelope`。direct `aicore-cli memory remember`、`aicore-cli memory accept` 与 `aicore-cli memory reject` 可以作为兼容本地命令保留，但不能被标记为 kernel-native invocation。

Memory write 的 audit contract 必须区分 Kernel invocation ledger 与 MemoryKernel 业务事实源。MemoryKernel DB 和 memory event ledger 记录 memory write 的业务事实；Kernel invocation ledger 只记录 invocation lifecycle。write result 的 public fields 必须表达 `write_applied`、`audit_closed`、`write_outcome` 和 `idempotency`。如果没有完整幂等系统，`idempotency` 必须是 `not_guaranteed`，不得声称 exactly-once。result 可以返回 memory_id、proposal_id、event_id 等 safe id，但不得返回 raw memory content 或 proposal content。

Memory write 失败必须结构化。empty content、invalid proposal id 或 MemoryKernel write failure 应返回 failed invocation，并尽量保留安全 public fields，例如 `write_applied = false`、`write_outcome = failed` 和 safe proposal id。若业务写入已经发生但 invocation audit close 失败，public surface 必须表达动作已经发生但审计闭合失败；若无法可靠判断 child process 是否已经应用写入，failure 必须表达 write outcome unknown，而不得静默当作无副作用失败。

## First-party Read-only Handler Boundary

一方只读 handler 是由 AICore OS 自身提供的受控 read-only adapter，用于读取全局运行时状态、配置路径状态、内核状态等安全摘要。它必须先通过 installed manifest registry 产生 route decision，再经由 `KernelInvocationRuntime` 执行，不得绕过 `KernelRouteRuntime` 直接调用。

一方只读 handler 可以作为本进程 adapter 存在，但它不是最终组件进程模型。它不得启动组件进程，不得进行跨进程调用，不得打开 socket IPC，不得调用 provider adapter，不得执行 tool，不得修改业务状态。

一方只读 handler 的 public surface 只能输出结构化摘要，例如 operation、route metadata、invocation id、handler status、ledger status、runtime installed status 和计数类信息。它不得输出 raw `KernelInvocationEnvelope.payload`、raw config、raw secret、`secret_ref`、`credential_lease_ref`、raw provider request、raw provider payload、raw tool input/output、API key、token 或 cookie。

启用 invocation ledger 时，一方只读 handler 的成功路径必须记录 accepted、route decision、handler execution、event generation 和 invocation completion。同一次 invocation 的所有 ledger records 与生成的 `KernelEventEnvelope` 必须共享同一个 `invocation_id`。

## Invocation Result Envelope

`KernelInvocationResultEnvelope` 是内核调用结果的 public result contract。handler 的结构化输出必须先进入 result envelope，再由 CLI、TUI、Web 或其他 terminal-facing consumer 渲染。人类可读 summary 只能作为 result envelope 的一个派生字段，不能反向作为机器数据源。

result envelope 至少表达 invocation id、trace id、operation、status、route metadata、handler metadata、result kind、result summary、public fields、failure stage、failure reason、handler_executed、event_generated 和 ledger_appended。

一方只读 operation 应通过 result envelope 返回结构化 public fields。`runtime.status` 类型的只读结果可以表达 global root、foundation installed、kernel installed、manifest count、capability count 和 bin path status 等字段。JSON terminal mode 应输出稳定 result object，调用方不应解析人类 panel body 来获取机器数据。

write operation 也必须通过 result envelope 返回结构化 public fields。JSON terminal mode consumer 必须读取 `result.fields` 中的 write audit fields，不得解析 human summary 或 stderr diagnostic 作为机器数据源。

`KernelEventEnvelope` 可以携带安全 summary，用于事件流和人类摘要，但不得代替 result envelope 作为结构化结果合同。invocation ledger 继续只记录生命周期、路由、handler 和失败 metadata，不记录完整 result payload，也不记录 raw handler output。

## Runtime Adoption Boundary

Kernel invocation runtime 是组件间能力调用的目标边界。面向业务能力或跨组件能力的 command，应逐步通过 installed manifest registry、route decision runtime、`KernelInvocationRuntime`、invocation ledger 和 result envelope 形成可审计调用链。

route 与 invocation diagnostic command 可以保留为内核链路验证工具。此类 command 用于展示 route decision、dispatcher、ledger 或 result envelope 状态，不代表真实业务 handler 已经接入。

本地 bootstrap、安装、配置路径读取、workflow 执行和 usage/error 输出可以保留 direct local path。此类路径必须被显式分类，不应伪装为 kernel-native business invocation。

provider、agent、memory、tool、MCP、workspace、patch、approval 等应用能力迁移到 kernel invocation 时，必须通过显式 adoption 边界完成。迁移前，direct path 的 public surface 仍需遵守 secret redaction、raw payload 隔离和用户可见语言策略。

CLI、TUI、Web 或 Gateway 不得把 human summary 当成机器数据源。机器可消费结果应来自 result envelope、event envelope 或明确的协议 payload。

`runtime.status` 是 first-party read-only runtime status capability。它的 handler ownership 属于共享 runtime status handler boundary，不属于 CLI 私有实现。调用方必须显式提供 runtime layout 或等价上下文，kernel 不应自行从用户环境推断 `HOME`。

顶层 `aicore` status 可以作为 kernel-native `runtime.status` consumer。该入口通过 installed Kernel runtime binary、installed manifest route、`KernelInvocationRuntime`、invocation ledger 与 result envelope 读取 runtime status；human status 文本从 structured result fields 派生。

`runtime.status` structured fields 可以表达 foundation runtime binary、kernel runtime binary 和 kernel invocation path。`kernel_invocation_path = binary` 表示 public application path 已经通过 installed kernel runtime binary；`in_process_test_only` 仅用于内部测试或受控 helper，不得作为 public installed path 的 silent fallback。

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
