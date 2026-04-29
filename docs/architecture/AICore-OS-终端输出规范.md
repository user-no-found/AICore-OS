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
│ □ Root     : <repo-root>                                 │
╰──────────────────────────────────────────────────────────╯
```

plain 示例：

```text
AICore OS
Composable Rust AgentOS Platform
Workflow  core
Mode      plain
Root      <repo-root>
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
- StructuredJson
- TOML
- Text
- WarningSummary
- FinalSummary

渲染规则：

- Markdown 保留标题、列表、代码块和空行。
- JSON block 对合法 JSON 做 pretty print；非法 JSON 输出 diagnostic。
- StructuredJson block 在 JSON mode 中输出指定 event name 与结构化 payload，适合内核 invocation result 等需要机器读取的 public result；human mode 可以退化为 JSON block 展示。
- TOML block 保留源文本。
- Table 处理中文与英文混合宽度。
- Diagnostic 分字段展示 severity、code、path、line、column、message 与 help。
- Text 必须做 sanitization 与 redaction。
- 所有 block 都必须支持 plain fallback。
- json mode 输出结构化 event，不输出人类 panel 字符串。

JSON mode 中，如果命令已经具备稳定机器结果对象，应优先输出结构化 result event，而不是要求调用方解析 panel body 或人类 summary 文本。

## CLI 输出接入

`aicore-cli` 的 terminal-facing 命令可以通过 `aicore-terminal` 输出结构化 document。

应用层命令应先构造语义化 view data，再转换为 terminal document：

- `Document`
- `Block::Panel`
- `Block::KeyValue`
- `Block::Diagnostic`
- `Block::FinalSummary`
- `Block::Table`
- `Block::Markdown`
- `Block::Text`
- `Block::WarningSummary`

CLI 不应在业务分支中重复实现私有 panel、table、JSON Lines 或 ANSI 风格。需要 rich / plain / json 的命令输出应复用 `aicore-terminal`。

已接入 terminal document 的 CLI 入口包括：

- `aicore-cli status`
- `aicore-cli kernel route <operation>`
- `aicore-cli kernel invoke-smoke <operation>`
- `aicore-cli kernel invoke-readonly <operation>`
- `aicore-cli config smoke`
- `aicore-cli config path`
- `aicore-cli config init`
- `aicore-cli config validate`
- `aicore-cli auth list`
- `aicore-cli model show`
- `aicore-cli service list`
- `aicore-cli runtime smoke`
- `aicore-cli instance list`
- `aicore-cli memory status`
- `aicore-cli memory search <关键词>`
- `aicore-cli memory proposals`
- `aicore-cli memory audit`
- `aicore-cli memory wiki [page]`
- `aicore-cli memory remember <内容>`
- `aicore-cli memory accept <proposal_id>`
- `aicore-cli memory reject <proposal_id>`
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
- `auth list` 可以显示 `auth_ref` 和 secret 配置状态，但不显示完整 `secret_ref`。

## App Entry 输出

`aicore` 顶层入口是 application entry / system summary，不是 TUI 入口。

`aicore` 输出可以作为 terminal-facing surface 使用 `aicore-terminal`：

- rich mode 输出 system summary panel。
- plain mode 输出无 ANSI、无 Unicode 边框的文本。
- json mode 输出 JSON Lines event，不混入 logo、人类 panel 或 ANSI。
- `NO_COLOR=1` 禁用 ANSI。

`aicore` summary 至少展示：

- 主实例
- 主实例工作目录
- 主实例状态目录
- 组件数量
- 实例数量
- Runtime

`aicore-tui` 是 TUI entry。顶层 `aicore` 不承载 TUI 菜单，也不承担产品化 TUI 交互。

## Install Visibility 输出

foundation workflow 的 install step 负责 shell PATH bootstrap。canonical install dir 为：

```text
$HOME/.aicore/bin
```

foundation install 在 bash 环境中通过 managed marker block 写入 `$HOME/.bashrc`：

```text
# >>> AICore OS >>>
export PATH="$HOME/.aicore/bin:$PATH"
# <<< AICore OS <<<
```

该 block 必须具备以下语义：

- block 不存在时追加。
- block 已存在时更新。
- 重复执行不重复追加。
- 用户删除该 block 即可回滚。
- CI 环境下跳过真实 shell rc 写入。

foundation shell bootstrap 输出必须包含：

- status
- shell
- rc file
- bin path
- action
- reload
- rollback

应用 workflow 的 install step 使用 canonical install dir：

```text
~/.aicore/bin
```

安装完成后，如果 `~/.aicore/bin` 不在当前 `PATH`，workflow 必须输出中文 warning，说明：

- `~/.aicore/bin` 当前不在 `PATH`
- 当前安装的二进制路径
- 如果 managed block 已存在，提示当前 shell 可能尚未 reload
- 如果 managed block 不存在，提示先运行 `cargo foundation`
- 重新加载命令：`source ~/.bashrc && hash -r`

安装完成后，workflow 会检查这些命令当前会被 shell 解析到哪里：

- `aicore`
- `aicore-cli`
- `aicore-tui`

如果当前解析路径不是 canonical install dir 下的新二进制，workflow 必须输出 shadowing warning，说明：

- 被 shadow 的命令名
- 当前 shell 实际执行路径
- 新安装二进制路径
- 修正 PATH 顺序或清理旧文件的建议

应用 workflow 的 install visibility warning 只负责提示，不自动删除旧文件，不自动覆盖非 managed 文件，不自动修改用户 shell rc。

## Utility Surface 输出

轻量 utility CLI 命令可以作为 terminal-facing consumer 使用 `aicore-terminal`：

- `config smoke` 展示配置存储读写、默认配置文件与配置校验结果。
- `runtime smoke` 展示 CLI、external origin 与 followed external 的 runtime 投递检查结果。
- `instance list` 展示 instance id、instance kind 与 workspace root。

Utility surface 输出只负责呈现，不改变：

- ConfigStore 读写规则
- config root 选择规则
- runtime ingress / output routing 语义
- instance registry 内容
- control plane 行为

rich mode 可以使用 panel 展示 utility summary。plain mode 保留可读文本。json mode 使用 JSON Lines event 输出结构化 payload，不混入人类 panel、ANSI 或 logo。

## Kernel Invocation Surface 输出

Kernel route / invocation 类 CLI 命令可以作为 terminal-facing consumer 使用 `aicore-terminal`：

- `kernel route <operation>` 展示 installed manifest registry 产生的 route decision。
- `kernel invoke-smoke <operation>` 展示受控 in-process smoke handler 调用结果。
- `kernel invoke-readonly <operation>` 展示 first-party read-only handler 调用结果。

Kernel invocation surface 输出只负责呈现，不改变 route runtime、handler registry、ledger 或 component state。

`kernel invoke-readonly` 的 JSON mode 应输出结构化 invocation result event，使 automation 可以读取 invocation id、route metadata、handler metadata、ledger status、result kind、result summary 与 public result fields。调用方不应解析 human panel body 来获得机器数据。

Kernel invocation surface 不得输出 raw `KernelInvocationEnvelope.payload`、raw secret、`secret_ref`、`credential_lease_ref`、raw provider request、raw provider payload、raw tool input/output、API key、token 或 cookie。

## Memory Read Surface 输出

Memory 只读 CLI 命令可以作为 terminal-facing consumer 使用 `aicore-terminal`：

- `memory status` 展示 memory root、record / proposal / event 统计、projection stale / warning / last rebuild metadata。
- `memory search <关键词>` 展示 search result、`memory_id`、`memory_type`、`source`、`permanence`、`score` 与 `matched_fields`。
- `memory proposals` 展示 open proposal 列表或空列表提示。
- `memory audit` 展示 ledger consistency 检查结果与 issue 列表。
- `memory wiki [page]` 展示只读 wiki projection metadata 与 Markdown 内容。

Memory read surface 输出只负责呈现，不改变：

- MemoryRecord 生命周期
- memory db schema
- Memory Event Ledger
- proposal review 状态机
- search/filter/ranking 语义
- FTS fallback 语义
- wiki projection 生成逻辑
- wiki page 白名单
- path traversal 拒绝逻辑

`memory wiki [page]` 的 Markdown 内容来自 wiki projection。该 projection 是派生读面，不是事实来源；事实来源仍是 `memory.db`、`MemoryRecord` 与 Memory Event Ledger。CLI 输出不得将 wiki projection 解析、重写或反向同步回事实源。

rich mode 可以将 wiki metadata 渲染为 panel，并将 Markdown projection 渲染为 Markdown block。plain mode 保留可读文本。json mode 使用 JSON Lines event，Markdown 内容作为 payload 字段输出，不混入人类 panel、ANSI 或 logo。

Memory 内容是用户数据。终端输出层不得擅自翻译、摘要或隐藏用户原始记忆内容；敏感输出边界仍适用于 raw secret、`secret_ref`、`credential_lease_ref`、raw SDK request、raw provider payload、API key、token 与 cookie。

## Memory Write Surface 输出

Memory 写入类 CLI 命令可以作为 terminal-facing consumer 使用 `aicore-terminal`：

- `memory remember <内容>` 展示写入结果、`memory_id`、`type` 与 `status`。
- `memory accept <proposal_id>` 展示 proposal 接受结果、`proposal_id` 与生成的 `memory_id`。
- `memory reject <proposal_id>` 展示 proposal 拒绝结果与 `proposal_id`。

Memory write surface 输出只负责呈现，不改变：

- MemoryRecord 写入语义
- proposal review 状态机
- Memory Event Ledger
- projection rebuild 行为
- accept / reject 错误语义

写入类命令应在业务操作完成后渲染 user-facing result。rich mode 可以使用 panel 展示结果；plain mode 保留可读文本；json mode 使用 JSON Lines event 输出结构化 payload，不混入人类 panel、ANSI 或 logo。

写入类命令不得为了输出迁移改变用户原始记忆内容。`proposal_id`、`memory_id`、`type`、`status` 等字段可以按技术字段原样展示。

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

workflow rich 输出中的 warning 状态使用 `! WARN`，不使用可能呈现为 emoji 风格的 warning symbol。plain / ASCII 输出继续使用 `[WARN]`。

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

rich mode 的 warning panel 必须结构化展示 warning，不把 multiline message 作为 raw blob 直接塞入 panel。安装可见性类 warning 使用字段化输出：

- `Message`
- `Paths`
- `Current`
- `Expected`
- `Fix`
- `Persist`

rich warning panel 必须与 workflow 主 panel 使用一致的宽度边界。长 path、command、shell rc 建议需要 wrap 或拆成缩进行，右边框不得被长行撑出终端可视区域。warning 状态可以用黄色强调，但不能整段染色。

plain mode 的 warning 输出不使用边框和 ANSI，仍应使用 `message / paths / current / expected / fix / persist` 等可读标签。

json mode 的 warning 输出仍是 JSON Lines event，不输出 rich panel、人类格式字符串或 ANSI。

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
