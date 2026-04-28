# AICore OS 组件进程与本地 IPC 规范

## 职责

组件进程边界定义 kernel invocation runtime 如何调用独立 application binary。该边界用于把 installed manifest 中声明的 capability 从本进程 handler 扩展到本地组件进程，同时保持 route decision、result envelope、ledger audit 和 public surface 的统一合同。

组件进程调用不等同于 daemon、socket server、component supervisor、package manager 或 plugin marketplace。组件进程可以被单次 invocation 启动，也可以在后续运行时模型中演进为受控的长期进程，但 public contract 必须继续由 kernel invocation envelope、route decision、result envelope 和 ledger 约束。

## Manifest Metadata

installed component manifest 可以声明 invocation mode 与 local IPC transport：

```toml
component_id = "aicore-component"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/user/.aicore/bin/aicore-cli"
invocation_mode = "local_process"
transport = "stdio_jsonl"
args = ["__component-smoke-stdio"]
working_dir = "/home/user"
env_policy = "minimal"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "component.process.smoke"
operation = "component.process.smoke"
visibility = "diagnostic"
```

`invocation_mode` 支持 `in_process` 与 `local_process`。缺省值为 `in_process`，用于兼容已有 manifest。

`transport` 支持 `stdio_jsonl`、`unix_socket` 与 `unsupported`。缺省值为 `unsupported`。当前可执行的本地 IPC skeleton 是 `stdio_jsonl`；`unix_socket` 是 manifest 可表达的 transport，不代表 socket server 已启用。

`args` 是传给 entrypoint 的静态参数列表。`working_dir` 是可选工作目录。`env_policy` 是环境变量策略标记，当前只表达策略名称，不承载完整 env injection 规则。

manifest 中的通用字段不得包含 provider、memory、agent、tool 等具体业务私有配置。业务协议和 SDK 请求组装属于对应应用内部。

业务只读 capability 可以声明为 local process capability。`config.validate` 使用独立 `aicore-config-validate` component manifest，operation 为 `config.validate`，entrypoint 指向已安装的 `aicore-cli` application binary，args 指向内部 stdio handler。`auth.list`、`model.show`、`service.list`、`runtime.smoke`、`instance.list`、`cli.status`、`provider.smoke`、`agent.smoke`、`agent.session_smoke` 与 memory read operations 使用同类独立 component manifests，并通过内部 stdio handler 输出结构化只读或 smoke 结果。此类 manifest 不改变 direct `aicore-cli config validate`、`aicore-cli auth list`、`aicore-cli model show`、`aicore-cli service list`、`aicore-cli runtime smoke`、`aicore-cli instance list`、`aicore-cli status`、`aicore-cli provider smoke`、`aicore-cli agent smoke`、`aicore-cli agent session-smoke` 或 direct memory read 本地命令语义。

## Route Metadata

route decision runtime 读取 installed manifests 后，必须把 component process metadata 传递到 route output：

- component id
- app id
- capability id
- operation
- entrypoint
- invocation mode
- transport
- args
- working directory
- env policy
- contract version
- visibility

route decision 仍只负责选择目标 component 和 capability，不启动进程，不写 ledger，不执行 handler。

## Stdio JSON Lines Transport

`stdio_jsonl` 是本地组件进程调用的最小 IPC skeleton。kernel invocation runtime 启动目标 entrypoint 后，通过 stdin 写入一行 JSON invocation request，并从 stdout 读取一行 JSON result。local process invocation 可以使用运行时默认 timeout 或调用 envelope 中的 timeout policy；timeout 发生后必须终止 child process，并返回结构化 failure。

面向应用的 public process invocation path 必须通过 installed Kernel runtime binary 发起。应用向 `$HOME/.aicore/bin/aicore-kernel --invoke-stdio-jsonl` 写入 runtime binary request，由 Kernel runtime binary 读取 installed manifest、做 route decision、进入 `LocalProcess` branch、spawn component process、处理 stdio JSONL、生成 result envelope，并写入 invocation ledger。应用不得绕过 Kernel runtime binary 直接执行 LocalProcess branch，也不得在 Kernel runtime binary 缺失时静默回落为 in-process success。

stdin request 至少包含：

- schema version
- protocol
- protocol version
- invocation id
- trace id
- instance id
- operation
- route metadata

stdin request 不应包含 raw `KernelInvocationEnvelope.payload`。如后续需要 payload 传输，必须定义 typed payload schema 和 redaction policy。

stdout 只承载 JSON Lines protocol result。当前 one-shot `stdio_jsonl` component process invocation 要求 stdout 恰好包含一条非空 JSON result line。空 stdout、非 JSON stdout、多条非空 stdout line、human log、progress、warning 或 debug 文本混入 stdout 都必须作为 protocol failure 处理。

stderr 可以作为 diagnostic source，但必须经过 redaction、truncation 和 terminal control sequence sanitization 后才能进入 public failure summary。ledger 不记录 raw stderr。

## Component Process Result

组件进程成功输出必须转换为 `KernelInvocationResultEnvelope`。result 至少表达：

- result schema version
- protocol
- protocol version
- invocation id
- trace id
- operation
- status
- route metadata
- handler kind
- result kind
- summary
- public fields
- handler executed
- event generated
- ledger appended

`result.kind` 可以使用 `component.process.smoke`、`provider.request`、`memory.read` 等稳定机器名。public fields 是机器可消费字段。human summary 只能从 result envelope 派生，不能反向作为机器数据源。

`config.validate` 的 public fields 应表达校验结果摘要，例如 operation、valid、config root、checked files、auth pool presence、runtime config presence、service profile presence、provider profile presence、error count、warning count 和 diagnostics summary。该 result 不得包含 raw config content、raw secret、`secret_ref` 或 credential material。

`auth.list` 的 public fields 应表达 auth count 与条目摘要。条目可以包含 auth_ref、provider、kind、enabled、capabilities 与 secret 状态。secret 状态只能是 configured、missing、redacted 等安全状态，不得包含 `secret_ref`、raw secret、API key、token 或 cookie。

`model.show` 的 public fields 应表达 primary model、primary auth_ref、provider、provider kind、fallback 状态和 runtime config presence。该 result 不得读取真实 key，不得解析 provider SDK payload，也不得输出 secret material。

`service.list` 的 public fields 应表达 service count 与服务角色摘要。服务条目可以包含 role、mode、auth_ref、model、enabled/configured 状态；不得输出 raw config file content 或 credential material。

`runtime.smoke` 的 public fields 应表达 runtime smoke 的只读检查摘要，例如 operation、runtime root、foundation runtime binary、kernel runtime binary、manifest presence、ledger presence、status、warning count 与 diagnostics。该 result 不得创建 instance，不得修改 runtime lifecycle，也不得打开 daemon 或 socket。

`instance.list` 的 public fields 应表达 instance count 与实例摘要。实例条目可以包含 instance_id、kind、workspace_root 或 scope、status、active/configured 状态；不得创建、删除或切换 instance。

`cli.status` 的 public fields 应表达 CLI status 的只读摘要，例如 app、contract version、runtime root、foundation installed、kernel installed、manifest count、capability count、kernel invocation path 和 bin path status。该 result 只描述状态，不执行 bootstrap 或安装修复。

Memory read operation 的 public fields 应表达现有 memory read surface 的结构化摘要。`memory.status` 至少表达 scope、record count、proposal count、wiki pages、db path 或 safe path summary、projection status 和 kernel invocation path。`memory.search` 至少表达 query、filters、result count 与 results，且 filters 必须保留 type、source、permanence 和 limit 等既有语义。`memory.proposals` 至少表达 proposal count 与 proposal 摘要。`memory.audit` 至少表达 ok、checked records、checked events、errors 和 warnings。`memory.wiki` 与 `memory.wiki_page` 必须保留 generated projection 的 not truth source 声明，`memory.wiki_page` 必须继续执行 page 白名单与 path traversal 拒绝规则。

Memory read result 可以保留 direct public read surface 已允许展示的记忆内容、localized summary、proposal 摘要和 wiki markdown。invocation ledger 不得记录 raw memory content、wiki markdown、search result raw content、raw stdout、raw stderr 或 raw invocation payload。Memory read local process capability 不代表 memory write、proposal accept/reject、Memory Agent、Vector、Graph 或 LLM Wiki 已接入。

组件进程失败也必须结构化表达：

- failure stage
- redacted failure reason
- handler kind
- spawned process
- process exit code
- transport
- route metadata

## Failure Semantics

组件进程调用的失败阶段至少包括：

- `transport_unsupported`
- `missing_entrypoint`
- `entrypoint_not_executable`
- `process_spawn_failed`
- `process_timeout`
- `process_stdin_failed`
- `process_stdout_failed`
- `process_non_zero_exit`
- `process_invalid_json`
- `process_protocol_mismatch`
- `process_result_mismatch`
- `process_result_schema_mismatch`
- `handler_lookup`

不支持的 transport 必须返回结构化 failure，不得回落为 in-process handler。entrypoint 缺失或不可执行时不得尝试执行。进程非零退出时 public surface 可以表达 exit code，但不得输出 raw stderr。stdout JSON 无效时返回 `process_invalid_json` failure。result schema、protocol version、invocation id 或 status 不匹配时必须返回结构化 protocol/result failure。

## Invocation Ledger

`invocation-ledger.jsonl` 继续是 append-only audit ledger。local process invocation 使用同一套 invocation id 关联规则。

成功路径记录：

- accepted
- route decision made
- handler executed
- event generated
- invocation completed

process boundary metadata 应进入 ledger metadata：

- handler kind：`local_process`
- spawned process：`true`
- called real component：按实际语义设置
- transport：例如 `stdio_jsonl`
- process exit code：如果可用

ledger 不记录完整 process result payload，不记录 raw stdout，不记录 raw stderr，不记录 raw runtime protocol payload，不记录 raw invocation request。

## Safety

public surface、result envelope 和 ledger 不得暴露：

- raw secret
- `secret_ref`
- `credential_lease_ref`
- raw provider request
- raw provider payload
- raw tool input/output
- raw memory content
- API key
- token
- cookie
- raw `KernelInvocationEnvelope.payload`

组件进程返回的所有 failure reason、stderr 摘要和 diagnostic 文本都必须经过脱敏。terminal control sequence 不得进入 public surface。

## Boundaries

`in_process` handler path 与 `local_process` path 必须在 route metadata 和 invocation runtime 中显式区分。local process capability 不得被误当成本进程 handler；unsupported transport 不得静默成功。

本地 IPC skeleton 只定义单次 invocation 的 stdio JSON Lines 边界。它不提供 daemon、socket server、process supervision、retry、rate-limit、endpoint fallback、model fallback、provider SDK 调用、tool execution、MCP、TUI/Web 产品化或 memory 功能扩展。
