# AICore-OS

## 底层编译安装

```bash
cargo foundation
```

## 内核层编译安装

```bash
cargo kernel
```

## 底层与内核层一键编译安装

```bash
cargo core
```

## 应用层编译安装

```bash
cargo app-aicore
cargo app-cli
cargo app-tui
```

## 安装后使用命令

```bash
aicore
aicore-cli status
aicore-cli instance list
aicore-cli runtime smoke
aicore-tui
```

说明：

- `cargo foundation` 执行底层 workflow，完成检查、测试、编译和安装。
- `cargo kernel` 执行内核层 workflow，完成检查、测试、编译和安装。
- `cargo core` 按顺序执行底层与内核层 workflow。
- `cargo app-aicore` 执行 `apps/aicore` 的 workflow，完成检查、测试、编译和安装。
- `cargo app-cli` 执行 `apps/aicore-cli` 的 workflow，完成检查、测试、编译和安装。
- `cargo app-tui` 执行 `apps/aicore-tui` 的 workflow，完成检查、测试、编译和安装。
- workflow 会自动检查对应 layer 的 target 目录，超过 `30GiB` 时会先清理再重新编译安装。
- `aicore` 当前显示主实例与 runtime 的最小系统状态摘要。
- `aicore-cli status` 显示当前系统摘要。
- `aicore-cli instance list` 显示当前实例列表。
- `aicore-cli runtime smoke` 验证 CLI / External Origin / Follow 三种运行时输出场景。
- `aicore-tui` 打开当前最小终端 AI 交互骨架。
