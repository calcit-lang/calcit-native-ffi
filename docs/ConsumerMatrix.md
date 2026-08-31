---
title: "Calcit native ABI consumer matrix"
summary: "Revision-pinned compatibility smoke across core, caps, bindgen, synchronous, asynchronous, blocking, and resource consumers"
scope: "module"
kind: "reference"
category: "ffi"
aliases:
  - "native ABI matrix"
  - "cross-consumer smoke"
  - "FFI compatibility pins"
  - "跨消费方 ABI 验证"
entry_for:
  - "consumer-pins.env"
  - "check-consumer-matrix.sh"
---

# Consumer matrix / 消费方矩阵

## 中文

`scripts/check-consumer-matrix.sh` 用 `consumer-pins.env` 中的完整 commit hash
检出真实消费方，并通过 Cargo patch 让它们统一链接当前 `calcit-native-ffi`
checkout。这样 PR 验证的是待提交代码，而不是 crates.io 上一个已发布版本。

矩阵依次验证：

- `symbol-layout`：Calcit core 以 `default-features = false` 编译共享 raw ABI，执行
  host layout、version、symbol、descriptor 和 callback registry 测试；
- `allocator-ownership-and-codec`：standalone `calcit-bindgen` 生成同步 adapter，真实
  release dylib 由 `caps` verifier 与 Calcit host 分别加载，并完成 EDN request、MD5
  response 和同库 buffer free；
- `blocking-lifecycle`：`calcit.std` 的 `read-file-by-line!` 通过 blocking v1 调用
  Calcit callback，并释放 host-owned callback buffers；
- `async-cancel-and-terminal-ordering`：`calcit-wss` 建立真实 WebSocket，交换消息，
  再取消 server task；listener、worker 和 registry 清理后 host 正常退出；
- `resource-lease-and-release`：`calcit-regex` 创建、借用并自动释放 opaque resource，
  trace 必须同时出现 create 与 release。

失败日志使用上述分类，避免把 symbol/layout mismatch、allocator ownership/codec、
blocking lifecycle、async terminal ordering 和 resource lease 混成同一种错误。

更新 pin 时必须使用上游已验证的 clean `main` commit，并在同一 PR 中运行完整矩阵、
更新本文兼容证据。不要改成 branch 或浮动 tag；pin 的目的正是让共享 ABI 变更可复现。

## English

`scripts/check-consumer-matrix.sh` checks out real consumers at the full commit
hashes recorded in `consumer-pins.env`. A Cargo patch makes every Rust consumer
link the current `calcit-native-ffi` checkout, so a pull request tests the code
under review instead of the previous crates.io release.

The matrix separates raw host symbol/layout checks, a generated synchronous
adapter loaded by both caps and Calcit, the calcit.std blocking callback path,
real calcit-wss connect/message/cancel ordering, and calcit-regex opaque-resource
creation and final release. GitHub annotations retain those categories so a
failure identifies the broken ownership or lifecycle layer.

Only update a pin to a validated clean `main` commit. Run the complete matrix
and update compatibility evidence in the same pull request. Do not replace
commit hashes with branches or floating tags; reproducibility is the purpose of
this test boundary.
