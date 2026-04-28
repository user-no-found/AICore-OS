# AICore OS 运行时安装布局规范

## 运行时根目录

AICore OS 的全局运行时根目录是：

```text
$HOME/.aicore
```

该目录承载本机用户级 AICore OS runtime 状态。repo-local `target/layers/*` 仍只作为构建、测试和本地安装记录 artifact，不代表全局 runtime 已激活。

## 顶层布局

```text
$HOME/.aicore/
  bin/
  runtime/
    foundation/
    kernel/
  share/
    manifests/
    contracts/
    schemas/
  state/
    kernel/
      invocation-ledger.jsonl
  config/
  cache/
  logs/
```

`bin` 保存用户可执行入口。`runtime` 保存 foundation 与 kernel 的 installed runtime metadata。`share` 保存可被 kernel registry 读取的 manifests、contracts、schemas。`state/kernel` 保存 kernel runtime 状态、route state、invocation ledger 与 lease 文件。`config`、`cache`、`logs` 分别保存配置、缓存和日志。

## Foundation Runtime Metadata

foundation install 负责写入：

```text
$HOME/.aicore/runtime/foundation/install.toml
$HOME/.aicore/runtime/foundation/version.toml
$HOME/.aicore/runtime/foundation/primitives.toml
$HOME/.aicore/runtime/foundation/terminal.toml
$HOME/.aicore/runtime/foundation/paths.toml
```

foundation metadata 描述底层原语、终端输出能力、路径布局和 runtime 版本。foundation install 同时确保 `$HOME/.aicore/bin` 存在，并负责 shell PATH bootstrap。

foundation 不作为用户直接执行命令安装。

## Kernel Runtime Metadata

kernel install 负责写入：

```text
$HOME/.aicore/runtime/kernel/install.toml
$HOME/.aicore/runtime/kernel/version.toml
$HOME/.aicore/runtime/kernel/contracts.toml
$HOME/.aicore/runtime/kernel/capabilities.toml
$HOME/.aicore/runtime/kernel/registry.toml
$HOME/.aicore/runtime/kernel/routing.toml
$HOME/.aicore/runtime/kernel/scheduler.toml
```

kernel metadata 描述 kernel runtime 版本、contracts、capability registry 来源、routing 策略和 scheduler 能力。kernel install 只写 runtime metadata，不安装 app binary，不执行 provider 请求，不执行 tool 或 MCP。

kernel 不作为用户直接执行命令安装。

## App Binary 与 Manifest

应用入口安装到：

```text
$HOME/.aicore/bin
```

应用 manifest 安装到：

```text
$HOME/.aicore/share/manifests
```

每个安装后的应用必须写入一个 TOML manifest。文件名使用 component id：

```text
$HOME/.aicore/share/manifests/aicore.toml
$HOME/.aicore/share/manifests/aicore-cli.toml
$HOME/.aicore/share/manifests/aicore-tui.toml
```

manifest 描述 component id、app id、kind、entrypoint、contract version 和 capabilities。第一轮 manifest schema 至少包含：

```toml
component_id = "aicore-cli"
app_id = "aicore-cli"
kind = "app"
entrypoint = "/home/user/.aicore/bin/aicore-cli"
contract_version = "kernel.app.v1"

[[capabilities]]
id = "memory.status"
operation = "memory.status"
visibility = "user"
```

`component_id` 与 `app_id` 用于 registry identity。`entrypoint` 指向 installed app binary。`contract_version` 描述应用接入 kernel runtime 的合同版本。`capabilities` 描述该 component 对外声明的用户可见操作。

kernel registry 以 installed manifests 作为运行时 registry 的输入。manifest loader 会读取 `*.toml`，构建 component registry summary 与 capability registry。当前 loader 只读取本地 TOML 文件。

route decision runtime 使用 installed manifests 解析 operation。成功路由时，内核返回目标 component、app、capability、operation、contract version、entrypoint 和 visibility。缺失 operation 返回 missing capability；重复 operation 返回 ambiguous route；contract id 或 major version 不兼容返回 contract version mismatch。

route decision runtime 不执行 handler，不启动进程，不做跨进程调用，不写 ledger。invocation ledger 属于调用执行链路，不属于只读 route decision。

应用安装阶段只写自身 manifest，不修改 kernel runtime metadata。

## Kernel Invocation Ledger

内核调用审计账本路径为：

```text
$HOME/.aicore/state/kernel/invocation-ledger.jsonl
```

该文件使用 append-only JSON Lines 格式。每行记录一次 invocation 生命周期中的一个审计 stage。ledger 用于审计和诊断，不是业务事实源，不参与恢复 component state，不提供 replay、query、compaction 或 conversation persistence。

每次调用必须使用单次 invocation 唯一的 `invocation_id`。同一次调用写入的所有 ledger records 与对应 `KernelEventEnvelope` 必须共享同一个 `invocation_id`，连续两次相同 operation 的调用不得复用同一个 `invocation_id`。

调用执行链路负责显式传入 ledger 路径或 writer。kernel crate 不应自行假设 `$HOME`，也不应隐藏读取全局路径。用户入口或应用入口负责从运行时 layout 解析 ledger 路径，并将其传入调用运行时。

ledger 写入失败必须作为结构化调用失败暴露。动作已发生但审计闭合失败时，public surface 必须同时表达 handler/event 状态和 ledger failure 状态。

## Shell PATH Bootstrap

foundation install 负责维护 bash 环境中的 managed PATH block：

```text
# >>> AICore OS >>>
export PATH="$HOME/.aicore/bin:$PATH"
# <<< AICore OS <<<
```

该 block 可重复执行、可更新、可删除回滚。CI 环境不修改真实 shell rc。应用安装阶段只检测当前 shell 可见性和 command shadowing，不写 shell rc。

## 元数据写入规则

runtime metadata 写入应使用同目录临时文件与 rename 进行本地原子替换。metadata 文件是运行时描述和 registry 输入，不是业务事实源；业务事实源仍由对应应用或 kernel ledger 管理。

## 顶层入口状态读取

`aicore` 顶层入口读取全局 runtime layout 并展示：

- global root
- foundation installed
- kernel installed
- contract version
- manifest count
- capability count
- event ledger path
- bin path status

`manifest count` 与 `capability count` 来自 `$HOME/.aicore/share/manifests/*.toml`。该入口用于显示系统安装与 runtime 状态，不等同于 TUI 菜单，也不承担 provider、tool、MCP 或 memory 业务执行。
