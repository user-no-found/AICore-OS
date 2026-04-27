# AICore-OS

AICore OS 是一个 composable Rust AgentOS platform。系统按底层、内核层、应用层拆分：底层提供稳定 primitive，内核层提供标准协议、路由、生命周期、事件、错误和调度合同，应用层按应用整体包独立编译并通过内核合同接入。

## 架构入口

- 底层编译整体：`crates/foundation/aicore-foundation`
- 内核编译整体：`crates/kernel/aicore-kernel`
- 应用层通过 `aicore-kernel / aicore-foundation` 接入，不把具体应用实现放入 kernel。
- Provider 请求实现属于 `aicore-provider` 应用层整体包，不属于 kernel。
- `aicore-provider` 当前采用 Rust ProviderHost + Python SDK worker 边界：Rust 侧负责 provider 选择、engine 管理和结果归一化，Python worker 侧承载官方 SDK 数据面。

## 正式文档

- [底层规范](docs/architecture/AICore-OS-底层规范.md)
- [内核协议规范](docs/architecture/AICore-OS-内核协议规范.md)
- [内核调度与并发规范](docs/architecture/AICore-OS-内核调度与并发规范.md)
- [应用接入规范](docs/architecture/AICore-OS-应用接入规范.md)
- [Provider 请求应用规范](docs/architecture/AICore-OS-Provider请求应用规范.md)
- [终端输出规范](docs/architecture/AICore-OS-终端输出规范.md)

## Workflow 命令

底层编译安装：

```bash
cargo foundation
```

内核层编译安装：

```bash
cargo kernel
```

底层与内核层一键编译安装：

```bash
cargo core
```

应用层编译安装：

```bash
cargo app-aicore
cargo app-cli
cargo app-tui
```

命令说明：

- `cargo foundation` 执行底层 workflow，完成检查、测试、编译和安装。
- `cargo kernel` 执行内核层 workflow，完成检查、测试、编译和安装。
- `cargo core` 按顺序执行底层与内核层 workflow。
- `cargo app-aicore` 执行 `apps/aicore` 的 workflow，完成检查、测试、编译和安装。
- `cargo app-cli` 执行 `apps/aicore-cli` 的 workflow，完成检查、测试、编译和安装。
- `cargo app-tui` 执行 `apps/aicore-tui` 的 workflow，完成检查、测试、编译和安装。
- workflow 会自动检查对应 layer 的 target 目录，超过 `30GiB` 时会先清理再重新编译安装。

## 当前 CLI 入口

系统状态与实例：

```bash
aicore
aicore-cli status
aicore-cli instance list
aicore-cli runtime smoke
```

Provider 与 agent smoke：

```bash
aicore-cli provider smoke
aicore-cli agent smoke <内容>
aicore-cli agent session-smoke <第一轮内容> <第二轮内容>
```

Memory smoke 与只读入口：

```bash
aicore-cli memory status
aicore-cli memory remember <内容>
aicore-cli memory search <关键词>
aicore-cli memory proposals
aicore-cli memory accept <proposal_id>
aicore-cli memory reject <proposal_id>
aicore-cli memory audit
aicore-cli memory wiki
aicore-cli memory wiki <page>
```

这些 CLI 入口用于检查当前合同、surface 和应用接入路径。`agent smoke`、`session-smoke` 和 `memory` 命令不代表产品化 chat UI、产品化 TUI 或历史会话数据库已经完成。

## Provider 请求应用边界

`aicore-provider` 是应用层整体包，provider 选择采用 provider-first 链路：

```text
provider_id -> ProviderAdapter -> api_mode -> RequestEngine
```

当前 OpenAI-compatible provider 走 OpenAI Python SDK worker，Anthropic-compatible provider 走 Anthropic Python SDK worker。Codex 登录态、Kimi Coding、Xiaomi 等路径保持独立 provider / profile 边界；Xiaomi 未显式配置 `base_url` 或 profile 时不会静默启用。

Provider public surface 不应暴露 raw secret、`secret_ref`、`credential_lease_ref`、raw SDK request 或 raw provider payload。

## Terminal Output

workflow 命令默认使用统一 terminal output kit。终端输出支持 rich、plain、json 与 auto mode。

常用环境变量：

```bash
AICORE_TERMINAL=plain cargo core
AICORE_TERMINAL=json cargo core
AICORE_VERBOSE=1 cargo core
AICORE_WORKFLOW_DENY_WARNINGS=1 cargo core
NO_COLOR=1 cargo core
```

模式说明：

- `AICORE_TERMINAL=auto` 根据 TTY / CI 自动选择 rich 或 plain。
- `AICORE_TERMINAL=rich` 使用 logo、状态符号、panel 和 summary。
- `AICORE_TERMINAL=plain` 使用无 ANSI、无 Unicode 边框的日志友好输出。
- `AICORE_TERMINAL=json` 输出 JSON Lines event stream，供 automation 使用。
- `AICORE_VERBOSE=1` 展开 cargo raw output。
- `AICORE_WORKFLOW_DENY_WARNINGS=1` 在检测到 warning 时让 workflow 失败。

## 当前非目标边界

以下能力当前不应被理解为已完成的产品化能力：

- provider live HTTP 默认调用
- secret resolver / real key loading
- credential lease 正式接入
- provider fallback、endpoint fallback、model fallback
- streaming
- tool calling
- MCP
- Web / TUI 产品化 agent loop
- Vector
- session 持久化 store
- 产品化 chat UI
