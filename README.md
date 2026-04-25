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

说明：

- `cargo foundation` 执行底层 workflow，完成检查、测试、编译和安装。
- `cargo kernel` 执行内核层 workflow，完成检查、测试、编译和安装。
- `cargo core` 按顺序执行底层与内核层 workflow。
- workflow 会自动检查对应 layer 的 target 目录，超过 `30GiB` 时会先清理再重新编译安装。
- 当前阶段不包含应用层编译。
