# AICore-OS

## 底层编译

```bash
cargo build-foundation
```

## 内核层编译

```bash
cargo build-kernel
```

## 底层与内核层一键编译

```bash
cargo build-core
```

说明：

- `build-foundation` 执行底层 workflow。
- `build-kernel` 执行内核层 workflow。
- `build-core` 按顺序执行底层与内核层 workflow。
- workflow 会自动检查 `target/`，超过 `30GiB` 时会先清理再重新编译。
- 当前阶段不包含应用层编译。
