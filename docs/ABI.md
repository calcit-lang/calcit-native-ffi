---
title: "Calcit native C ABI v1"
summary: "Stable C-layout contracts for buffers, asynchronous tasks, blocking callbacks, responses, and native resource tokens"
scope: "module"
kind: "reference"
category: "ffi"
aliases:
  - "native ffi"
  - "C ABI"
  - "CalcitFfiBuffer"
  - "async task ABI"
  - "blocking callback ABI"
  - "response capability"
  - "resource token"
  - "原生模块 ABI"
entry_for:
  - "CalcitFfiBuffer"
  - "CalcitFfiAsyncHostV1"
  - "CalcitFfiBlockingHostV1"
  - "CalcitFfiResourceV1"
---

# Calcit native C ABI v1 / Calcit 原生 C ABI v1

## 中文

### 设计目标

- C layout 是唯一跨动态库边界的数据布局，不暴露 Rust enum、`Vec`、trait object
  或 allocator-owned object。
- descriptor 前两个字段固定为 `protocol_version: u32` 和 `struct_size: u32`。
- v1 consumer 接受更大的 descriptor，以便未来通过尾部字段扩展。
- 未知版本、小于 v1 的结构、无效 buffer metadata 必须在解引用完整结构前拒绝。
- 所有 payload 在调用期间复制，异步 producer 不保留 host 临时指针。

### Buffer ownership

模块通过 `CalcitFfiBuffer { ptr, len, cap }` 返回由模块 allocator 创建的数据。
Host 读取后必须调用同一个动态库导出的 `calcit_ffi_buffer_free`。不得由 host
直接构造 `Vec::from_raw_parts`，也不得使用另一个动态库的 free symbol。

共享 crate 同时导出协议版本、固定 symbol/suffix 与 host 解析 method 时使用的
function pointer 类型。Host 与 module 必须消费同一份 raw 定义，不能在各自
仓库复制一份“布局相同”的 struct 或数值常量。

Host-owned blocking callback buffer 则遵循相反方向：模块复制内容后调用
`CalcitFfiBlockingHostV1.free_buffer`，不能调用模块自己的 buffer free。

### Async lifecycle

- Task kind：one-shot、stream、server、response。
- Event kind：emit、complete、fail；complete/fail 是 terminal。
- `enqueue` 返回 `QUEUE_FULL` 时 payload 尚未被接受，producer 可按策略重试。
- cancel callback 只接受 C-safe bytes 和 opaque integer context/handle。
- response capability 必须 exactly-once resolve/reject，过期或重复 handle 由 host
  拒绝。

### Blocking lifecycle

Blocking callback 只能在 host 保留的执行线程调用。模块必须释放每一个 host
返回的 callback buffer，并在协议需要时 exactly-once 调用 `finish`。

### Resource token

Resource v1 使用固定的 version/release symbol，以及名为
`CalcitFfiResourceV1`、只包含 16-byte `token` buffer 的 Cirru EDN struct。
共享 crate 只定义 wire contract；generation registry、lease intern、dylib pin
与 exactly-once release 仍由模块和 host 各自按职责实现。

## English

### Design goals

- C layout is the only cross-library data layout. Rust enums, `Vec`, trait
  objects, and allocator-owned objects never cross the boundary.
- Every versioned descriptor starts with `protocol_version: u32` and
  `struct_size: u32`.
- A v1 consumer accepts a larger descriptor so future versions may append
  fields.
- Unknown versions, undersized structures, and invalid buffer metadata are
  rejected before reading the full structure.
- Payloads are copied during calls; asynchronous producers never retain host
  temporary pointers.

### Buffer ownership

A module returns `CalcitFfiBuffer { ptr, len, cap }` allocated by that module.
After copying it, the host calls `calcit_ffi_buffer_free` from the same dynamic
library. The host must not reconstruct the `Vec` itself or use another module's
free symbol.

The shared crate also exports protocol versions, fixed symbols/suffixes, and
the function-pointer types used by host method resolution. The host and module
must consume this single raw definition instead of maintaining structs or
numeric constants that merely happen to share a layout.

Host-owned blocking callback buffers flow in the opposite direction. The
module copies the bytes and calls `CalcitFfiBlockingHostV1.free_buffer`, never
its own buffer-free export.

### Async lifecycle

- Task kinds are one-shot, stream, server, and response.
- Event kinds are emit, complete, and fail; complete/fail are terminal.
- `QUEUE_FULL` means the payload was not accepted and may be retried according
  to an explicit policy.
- Cancellation crosses only C-safe bytes and opaque integer context/handles.
- Response capabilities resolve or reject exactly once; the host rejects stale
  or duplicate handles.

### Blocking lifecycle

Blocking callbacks run only on the execution thread reserved by the host. The
module releases every callback buffer returned by the host and calls `finish`
exactly once when required by the method contract.

### Resource token

Resource v1 uses fixed version/release symbols and a `CalcitFfiResourceV1`
Cirru EDN struct containing one 16-byte `token` buffer. The shared crate owns
only this wire contract. Generation registries, lease interning, dylib pinning,
and exactly-once release remain module- and host-owned responsibilities.
