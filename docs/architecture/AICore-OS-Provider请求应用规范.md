# AICore OS Provider 请求应用规范

## 职责

Provider 请求应用负责把内核路由来的模型调用转换为外部模型服务请求，并把外部响应归一化为 AICore 的 provider 事件和模型响应。

内核只处理能力、路由、权限、审计、取消、超时、事件和错误边界。SDK 初始化、请求字段组装、外部端点差异、登录态细节、响应归一化都属于 Provider 请求应用。

## 应用整体包

`aicore-provider` 是应用层的一个整体包。不同厂家、协议和 SDK engine 是应用内部模块，不拆成多个独立应用。

应用内部边界如下：

```text
ProviderHost
-> ProviderRuntimeResolver
-> ProviderAdapter
-> RequestEngine
-> PythonSdkWorker
-> ProviderEventNormalizer
```

## Rust ProviderHost

Rust ProviderHost 是控制面，负责接收调用、解析 provider runtime、管理 engine 调用、执行超时和取消策略、处理脱敏、归一化错误并返回稳定事件。

ProviderHost 不直接持有明文密钥，不把 raw SDK request 暴露给 CLI、TUI、Web、agent surface 或日志。

## Python SDK Engine

Python SDK Engine 是数据面 worker，直接调用官方 Python SDK。

Rust 与 Python 之间只通过 Provider Engine Interface 传输结构化 JSONL 消息。Rust 不嵌入 Python 解释器，Python worker 不读取 Rust 内部对象。

Python worker 的 stdout 只输出 ProviderEngineEvent JSONL。诊断日志只进入 stderr，public surface 不消费 stderr 原文。

## ProviderAdapter

ProviderAdapter 表示厂家或服务形态差异。它负责：

- provider_id 归一化
- adapter_id 选择
- base_url 默认值和覆盖值
- auth kind 约束
- model family 约束
- api_mode 选择
- extra header / extra body 的边界

ProviderAdapter 不执行 SDK 调用。

## RequestEngine

RequestEngine 表示请求执行方式。常见 engine 包括：

- `dummy`
- `python.openai`
- `python.anthropic`
- `python.codex_bridge`
- `rust.openai_compatible_http`
- `rust.anthropic_compatible_http`

官方 Python SDK 是优先 engine。Rust HTTP engine 用于兼容端点、诊断或缺少官方 SDK 的路径。

## ProviderRuntimeResolver

ProviderRuntimeResolver 根据实例运行配置、全局凭证池和 provider profile 解析 ProviderRuntime。

解析顺序：

```text
auth_ref
-> AuthEntry
-> provider_id
-> ProviderRegistry
-> ProviderProfile
-> ProviderAdapter
-> api_mode
-> RequestEngine
-> ProviderRuntime
```

`AuthCapability::Chat` 是 chat provider 调用的必要能力。缺少 chat capability 的 auth_ref 不会进入 SDK engine。

## Provider Engine Interface

Provider Engine Interface 是 Rust ProviderHost 和 Python SDK Engine 之间的稳定消息接口。

消息使用 JSONL：

- 一行一个 JSON 对象
- stdin 传入 ProviderEngineRequest
- stdout 输出 ProviderEngineEvent
- stderr 仅用于 worker 诊断

## ProviderRuntime

ProviderRuntime 包含：

- provider_id
- adapter_id
- engine_id
- api_mode
- auth_mode
- model
- base_url
- auth_ref

ProviderRuntime 不包含明文密钥。

## ProviderEngineRequest

ProviderEngineRequest 包含：

- protocol_version
- invocation_id
- provider_id
- adapter_id
- engine_id
- api_mode
- model
- base_url
- credential_lease_ref
- messages
- tools_json
- parameters_json
- stream
- timeout_ms

`credential_lease_ref` 是凭证租约引用，不是明文密钥。ProviderEngineRequest 不允许携带 raw API key。

## ProviderEngineEvent

ProviderEngineEvent 表示 engine 输出事件：

- Started
- MessageDelta
- ReasoningDelta
- ToolCallDelta
- Usage
- Finished
- Error

Error 事件必须提供面向用户的中文错误文本和面向机器处理的 machine_code。错误事件进入 public surface 前必须脱敏。

## Provider 选择规则

Provider 请求应用采用：

```text
provider first
api_mode second
engine third
```

强 provider 语义优先于 URL heuristic。自定义端点和显式 endpoint profile 可以使用 URL heuristic 辅助选择 api_mode。

## API Mode

常见 api_mode：

- `dummy`
- `openai_chat_completions`
- `openai_responses`
- `anthropic_messages`
- `gemini_generate_content`
- `codex_responses`

OpenAI-compatible 第三方服务默认复用 `openai_chat_completions`。Anthropic-compatible 第三方服务默认复用 `anthropic_messages`。

## Credential Lease 与 Secret Redaction

密钥存放在凭证系统中。Provider 请求应用只接收凭证引用和租约引用。

禁止输出：

- raw API key
- secret_ref
- credential_lease_ref
- raw SDK request
- raw provider payload

错误、事件、CLI 输出、agent surface 和测试断言都必须遵守脱敏边界。

## 错误与事件归一化

SDK error、transport error、engine unavailable、credential unavailable、timeout、cancel 都归一化为 ProviderError 或 ProviderEngineEvent。

用户可见错误使用中文。命令名、provider_id、api_mode、engine_id、machine_code 保持英文标识。

## 热插拔、Drain 与 Rollback

ProviderHost 管理 engine 生命周期。engine 更新时应支持：

- 新调用进入新 engine
- 运行中的调用继续 drain
- 失败时返回结构化错误
- 回滚时恢复到可用 engine profile

热插拔不改变内核协议，也不要求内核重启。

## 官方 SDK 优先策略

有官方 Python SDK 的 provider 优先使用官方 Python SDK。兼容端点通过 base_url、api_mode、headers 和 parameters 进入对应 SDK engine。

Rust HTTP engine 保留为兼容、诊断和特殊端点能力，不替代官方 SDK 的首选位置。

## Custom Endpoint 策略

Custom endpoint 必须显式配置 provider profile。base_url 不能从错误的 provider identity 静默推断。

OpenAI-compatible custom endpoint 使用 `custom-openai-compatible`。Anthropic-compatible custom endpoint 使用 `custom-anthropic-compatible`。

## 登录态 Provider 策略

登录态 provider 使用独立 provider_id、auth_mode 和 engine_id。登录态链路不混入普通 API key 链路。

Codex 登录态使用独立 `openai-codex-login` provider。Coding 类特殊 endpoint 使用独立 adapter，避免被普通 chat-compatible 路径吞掉。
