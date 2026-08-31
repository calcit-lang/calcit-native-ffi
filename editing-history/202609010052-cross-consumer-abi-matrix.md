# 跨消费方 ABI 验证 / Cross-consumer ABI matrix

## 中文

- 增加 revision-pinned 的 Calcit、caps、bindgen、WSS、std 与 regex 消费方矩阵。
- 所有 Rust 消费方通过 Cargo patch 链接当前 checkout，避免误测 crates.io 旧版本。
- 真实 release dylib 覆盖 generated sync adapter、buffer free、blocking callback、
  WebSocket cancel/terminal ordering 与 opaque-resource create/release。
- CI 按 symbol/layout、allocator/codec、blocking、async 与 resource lifecycle 分类失败。
- 将 async helper 测试的共享全局计数器改为 host context 独立计数器，消除并行测试竞态。

## English

- Add a revision-pinned consumer matrix across Calcit, caps, bindgen, WSS, std,
  and regex.
- Patch every Rust consumer to the current checkout rather than accidentally
  testing the previous crates.io release.
- Exercise real release dylibs for generated sync adapters and buffer release,
  blocking callbacks, WebSocket cancellation/terminal ordering, and opaque
  resource creation/final release.
- Classify CI failures by symbol/layout, allocator/codec, blocking, async, and
  resource lifecycle boundaries.
- Replace the async-helper tests' shared global counter with per-host-context
  counters, eliminating a parallel-test race exposed by the new gate.
