# AICore OS 终端输出规范

## 目标

AICore OS 使用统一的 terminal output kit 渲染面向用户的命令行输出。该能力用于 workflow、CLI、doctor、install、config、provider smoke、agent smoke、memory smoke 等 terminal-facing 场景，使输出在 rich、plain 与 JSON Lines 场景下保持一致、可读、可自动化处理。

终端输出层只负责展示、脱敏、清理控制字符、格式化诊断与汇总。业务状态、内核事件、provider 请求、memory record、tool result 等事实数据由对应层或应用产生。

## 非目标

终端输出层不负责：

- 内核路由
- provider 请求发送
- secret 解析
- credential lease
- tool calling
- MCP 桥接
- memory 事实写入
- session 持久化
- TUI / Web 产品化交互

## 架构归位

`aicore-terminal` 位于 Foundation Layer：

```text
crates/foundation/aicore-foundation
crates/foundation/aicore-terminal
```

职责划分：

- `aicore-foundation` 提供 ID、错误、路径、时间、取消、队列、租约、redaction 等基础 primitive。
- `aicore-terminal` 提供 terminal rendering、输出模式、状态符号、panel、table、diagnostic、warning summary、final summary、redaction 与 sanitization。
- `aicore-workflow`、`aicore-cli` 或其他 terminal-facing consumer 通过 `aicore-terminal` 渲染输出。

允许依赖方向：

```text
aicore-foundation
        ↑
aicore-terminal
        ↑
aicore-workflow / terminal-facing apps
```

禁止依赖方向：

- `aicore-foundation -> aicore-terminal`
- `aicore-kernel -> aicore-terminal`
- `aicore-terminal -> aicore-kernel`
- `aicore-terminal -> apps/*`
- `aicore-terminal -> aicore-provider`
- `aicore-terminal -> aicore-memory`
- `aicore-terminal -> aicore-agent`

内核层只产出结构化事件、错误、诊断和合同数据。终端渲染由使用方在应用边界或 workflow 边界完成。

## 输出模式

终端模式由 `AICORE_TERMINAL` 控制：

```text
AICORE_TERMINAL=auto|rich|plain|json
```

语义：

- `auto`：根据 TTY 与 CI 环境选择 rich 或 plain。
- `rich`：使用 header panel、颜色、边框、状态符号、workflow step table 与 summary panel。
- `plain`：无 ANSI、无 Unicode 边框、无动态刷新，适合日志。
- `json`：输出 JSON Lines event stream，适合 automation。

默认选择：

- TTY 且非 CI：rich
- CI 或 non-TTY：plain
- `AICORE_TERMINAL=json`：json

JSON mode 要求：

- stdout 只输出 JSON Lines。
- 不输出 logo、ANSI、人类 panel 或 prose。
- 每行包含 `schema = "aicore.terminal.v1"`。
- 不输出 raw secret。
- 不输出 raw provider request。
- 不输出 unsafe terminal control sequence。

## 环境变量

支持：

```text
AICORE_TERMINAL=auto|rich|plain|json
AICORE_COLOR=auto|always|never
AICORE_LOGO=compact|full|off
AICORE_SYMBOLS=unicode|ascii
AICORE_VERBOSE=0|1
AICORE_WORKFLOW_DENY_WARNINGS=0|1
AICORE_PROGRESS=auto|always|never
NO_COLOR=1
CI=1
```

## 颜色策略

颜色由 `AICORE_COLOR` 与 `NO_COLOR` 控制：

- `NO_COLOR=1` 必须禁用 ANSI。
- JSON mode 永远不输出 ANSI。
- CI / non-TTY 默认无颜色。
- 颜色不能作为唯一信息来源，必须同时有文本和符号。
- 不使用大面积背景色。
- workflow step table 中只对状态符号与状态词着色，不对整行输出着色。
- rich mode 使用 cyan 作为 AICore accent；绿色只表示成功，黄色只表示 warning，红色只表示 failure。
- rich mode 边框和弱分隔线使用 dim gray，避免高亮白色边框主导视觉。
- rich mode label 使用 soft violet，与 dim gray 边框 / 弱分隔线保持清晰区分，并避免回落成普通白色正文层级。

颜色语义：

```text
OK / success          green
WARN                  yellow
FAILED / error        red
RUNNING               cyan
SKIPPED               dim gray
section title         bold cyan
brand / accent        cyan
label                 soft violet
border / separator    dim gray
command               dim / neutral
path                  blue
secret redaction      yellow or dim
```

## Header Panel

Header / logo 由 `AICORE_LOGO` 控制：

```text
AICORE_LOGO=compact|full|off
```

rich 默认使用 compact header panel。plain 输出使用无边框文本。json mode 强制关闭 logo。

rich header 包含品牌行与 metadata 区：

- 品牌行包含轻量线性 symbol、`AICore OS` 与 `Composable Rust AgentOS Platform`。
- metadata 使用两列布局展示 `Workflow / Target / Root / Mode / Warnings`。
- label 与冒号对齐。
- Root 独占一行，长路径不得破坏 panel 右边界。
- `Warnings` policy metadata 使用中性 symbol；只有实际 warning 状态才使用 yellow。
- rich + unicode mode 可以使用线性 symbol；`AICORE_SYMBOLS=ascii` 时 symbol 使用 ASCII fallback。

compact rich 示例：

```text
╭──────────────────────────────────────────────────────────╮
│ ⎇ AICore OS — Composable Rust AgentOS Platform           │
│                                                          │
│ ⎇ Workflow : core                    ◈ Mode     : rich   │
│ ◎ Target   : foundation + kernel     ◇ Warnings : report │
│ □ Root     : /vol1/1000/sun/aicore/AICore-OS             │
╰──────────────────────────────────────────────────────────╯
```

plain 示例：

```text
AICore OS
Composable Rust AgentOS Platform
Workflow  core
Mode      plain
Root      /vol1/1000/sun/aicore/AICore-OS
Target    foundation + kernel
Warnings  report
```

## 状态符号

状态符号由 `AICORE_SYMBOLS` 控制：

```text
AICORE_SYMBOLS=unicode|ascii
```

映射：

```text
OK       ✓        [OK]
WARN     ⚠        [WARN]
FAILED   ✗        [FAILED]
RUNNING  [RUNNING] [RUNNING]
INFO     •        [INFO]
SKIPPED  –        [SKIPPED]
```

plain / CI 默认使用 ASCII。

rich mode 的 section symbol 使用轻量线性字符，不使用彩色 emoji。成功输出中不得残留 running symbol。

## Block 类型

`aicore-terminal` 支持以下 block：

- Logo
- Panel
- KeyValue
- Table
- Diagnostic
- Markdown
- JSON
- TOML
- Text
- WarningSummary
- FinalSummary

渲染规则：

- Markdown 保留标题、列表、代码块和空行。
- JSON block 对合法 JSON 做 pretty print；非法 JSON 输出 diagnostic。
- TOML block 保留源文本。
- Table 处理中文与英文混合宽度。
- Diagnostic 分字段展示 severity、code、path、line、column、message 与 help。
- Text 必须做 sanitization 与 redaction。
- 所有 block 都必须支持 plain fallback。
- json mode 输出结构化 event，不输出人类 panel 字符串。

## CLI 输出接入

`aicore-cli` 的 terminal-facing 命令可以通过 `aicore-terminal` 输出结构化 document。

应用层命令应先构造语义化 view data，再转换为 terminal document：

- `Document`
- `Block::Panel`
- `Block::KeyValue`
- `Block::Diagnostic`
- `Block::FinalSummary`
- `Block::Table`

CLI 不应在业务分支中重复实现私有 panel、table、JSON Lines 或 ANSI 风格。需要 rich / plain / json 的命令输出应复用 `aicore-terminal`。

已接入 terminal document 的 CLI 入口包括：

- `aicore-cli status`
- `aicore-cli provider smoke`
- `aicore-cli agent smoke <内容>`
- `aicore-cli agent session-smoke <第一轮内容> <第二轮内容>`

这些命令的输出规则：

- rich mode 输出 panel 或 summary 形态。
- plain mode 输出无 ANSI、无 Unicode 边框的文本。
- json mode 输出 JSON Lines，不混入 logo、人类文本或 ANSI。
- `NO_COLOR=1` 禁用 ANSI，但不改变输出语义。
- 用户说明使用中文；命令名、字段名、provider_id、api_mode、engine_id、machine code 保持英文。
- public surface 不暴露 raw secret、`secret_ref`、`credential_lease_ref`、raw SDK request 或 raw provider payload。

## Workflow 输出

`aicore-workflow` 使用 `aicore-terminal` 输出 workflow 状态。

Workflow event 至少包括：

- `run.started`
- `step.started`
- `step.finished`
- `warning`
- `run.finished`

Cargo workflow alias 使用 quiet run 入口，不显示 Cargo wrapper 的 `Finished ...` 与 `Running ...` 噪音。

rich mode 的 workflow 输出包含：

- header panel
- workflow step table
- warning panel
- summary panel

成功结束后的 workflow step table 汇总所有 step，不保留散乱 running 行。

step table 使用英文技术字段：

```text
╭─ > Workflow Steps ───────────────────────────────────────╮
│ #  Layer       Step     Status  Warn  Time               │
│ ──────────────────────────────────────────────────────── │
│ 1  foundation  fmt      ✓ OK    0     0.08s              │
│ ──────────────────────────────────────────────────────── │
│ 2  foundation  test     ✓ OK    0     0.31s              │
│ ──────────────────────────────────────────────────────── │
│ 3  foundation  build    ✓ OK    0     0.12s              │
╰──────────────────────────────────────────────────────────╯
```

rich step table 使用 soft violet header、cyan row number、dim separator、绿色成功状态、黄色 warning 状态、红色失败状态。状态颜色只作用于状态单元格，不对整行染色。

最终 summary 输出 workflow、状态、step 统计、warning 统计、duration 和 result：

```text
╭─ = Summary ──────────────────────────────────────────────╮
│ Workflow : core                                          │
│ Status   : ✓ OK                                          │
│ Steps    : 8 total / 8 ok / 0 failed                     │
│ Warnings : 0 scanned this run                            │
│ Duration : 1.42s                                         │
│ Result   : workflow completed successfully               │
╰──────────────────────────────────────────────────────────╯
```

summary result 按最终状态着色：success 使用 green，warning 使用 yellow，failure 使用 red。

summary 文案使用：

```text
Warnings 0 scanned this run
```

该文案只表示当前运行扫描到的 warning 数量，不表示缓存中的历史构建产物没有 warning。

## Warning 捕获与汇总

workflow 执行 cargo 命令时捕获 stdout / stderr，并解析 warning。

warning 来源包括：

- cargo structured diagnostics
- rustc rendered diagnostic text
- rustdoc warning text
- build script warning
- fallback text scanner

warning 去重使用：

```text
step + path + line + column + normalized message
```

warning summary 必须展示 warning 总数，并在数量较多时截断明细。

## Strict Warning Mode

`AICORE_WORKFLOW_DENY_WARNINGS=1` 启用 strict warning mode。

语义：

- 普通模式下，`warning_count > 0` 允许 workflow 完成，但最终状态为 warn。
- strict mode 下，`warning_count > 0` 使 workflow 失败。
- strict mode 失败时必须显示中文说明：

```text
已启用 AICORE_WORKFLOW_DENY_WARNINGS=1。
检测到 warning，因此 workflow 失败。
```

## Redaction 与 Sanitization

所有 terminal-facing 输出必须清理 unsafe terminal control sequence。

输出不得暴露：

- raw secret
- `secret_ref`
- `credential_lease_ref`
- raw SDK request
- raw provider payload

疑似 secret 的 token 应替换为：

```text
[REDACTED]
```

## JSON Event Schema

JSON Lines event 使用以下最小结构：

```json
{
  "schema": "aicore.terminal.v1",
  "event": "run.started",
  "payload": {}
}
```

要求：

- 每行都是合法 JSON。
- `schema` 固定为 `aicore.terminal.v1`。
- `event` 表示事件类型。
- `payload` 承载事件内容。
- payload 不包含 raw secret、raw provider request 或 unsafe terminal control sequence。

## 测试原则

测试应覆盖：

- auto mode 在 CI 下选择 plain。
- `NO_COLOR=1` 禁用 ANSI。
- Unicode / ASCII 状态符号。
- rich / plain panel。
- KeyValue 与 Table 对齐。
- Markdown / JSON / TOML block。
- Diagnostic rendering。
- WarningSummary 与 FinalSummary。
- unsafe control sequence sanitization。
- secret-like token redaction。
- workflow warning parser。
- workflow strict warning mode。
- workflow JSON Lines 输出。
