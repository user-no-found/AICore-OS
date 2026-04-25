# AICore-OS

## 底层编译

```bash
cd /vol1/1000/sun/aicore/AICore-OS && TARGET_BYTES=$(du -s -B1 target 2>/dev/null | cut -f1); if [ -n "$TARGET_BYTES" ] && [ "$TARGET_BYTES" -gt 32212254720 ]; then rm -rf target; fi && cargo fmt --check && cargo test -p aicore-foundation -p aicore-contracts --offline && cargo build -p aicore-foundation -p aicore-contracts --offline
```

## 内核层编译

```bash
cd /vol1/1000/sun/aicore/AICore-OS && TARGET_BYTES=$(du -s -B1 target 2>/dev/null | cut -f1); if [ -n "$TARGET_BYTES" ] && [ "$TARGET_BYTES" -gt 32212254720 ]; then rm -rf target; fi && cargo fmt --check && cargo test -p aicore-auth -p aicore-config -p aicore-control -p aicore-runtime -p aicore-surface -p aicore-tools -p aicore-memory -p aicore-skills -p aicore-evolution --offline && cargo build -p aicore-auth -p aicore-config -p aicore-control -p aicore-runtime -p aicore-surface -p aicore-tools -p aicore-memory -p aicore-skills -p aicore-evolution --offline
```

## 底层与内核层一键编译

```bash
cd /vol1/1000/sun/aicore/AICore-OS && TARGET_BYTES=$(du -s -B1 target 2>/dev/null | cut -f1); if [ -n "$TARGET_BYTES" ] && [ "$TARGET_BYTES" -gt 32212254720 ]; then rm -rf target; fi && cargo fmt --check && cargo test -p aicore-foundation -p aicore-contracts -p aicore-auth -p aicore-config -p aicore-control -p aicore-runtime -p aicore-surface -p aicore-tools -p aicore-memory -p aicore-skills -p aicore-evolution --offline && cargo build -p aicore-foundation -p aicore-contracts -p aicore-auth -p aicore-config -p aicore-control -p aicore-runtime -p aicore-surface -p aicore-tools -p aicore-memory -p aicore-skills -p aicore-evolution --offline
```
