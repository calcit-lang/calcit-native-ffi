# calcit-native-ffi

Calcit native 模块共用的稳定 C ABI 类型与适配器。

Stable C ABI types and adapters shared by Calcit native modules.

## 中文

该 crate 维护 Calcit runtime 与 Rust `cdylib` 之间的传输协议，避免每个
native 模块复制 descriptor、buffer ownership、Cirru EDN 编解码、异步任务和
blocking callback 模板。

边界刻意保持较窄：业务参数、线程、连接表、取消状态和 server 生命周期仍由
各模块维护。

### 使用

```toml
[dependencies]
calcit_native_ffi = "0.1"
```

在最终 `cdylib` 中显式生成协议符号：

```rust
calcit_native_ffi::export_buffer_abi_v1!();
calcit_native_ffi::export_async_abi_v1!();
```

同步函数保留一个很薄的导出 wrapper：

```rust
use calcit_native_ffi::{CalcitFfiBuffer, run_buffer_adapter};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn demo_calcit_ffi_v1(
  request_ptr: *const u8,
  request_len: usize,
  output: *mut CalcitFfiBuffer,
) -> i32 {
  unsafe { run_buffer_adapter(request_ptr, request_len, output, demo) }
}
```

`BackpressurePolicy::default()` 保留现有 native 模块的 1ms retry 行为。新代码若
不希望无限等待，应显式使用 `BackpressurePolicy::bounded`，或直接调用一次性
`enqueue` 并自行处理 `status::QUEUE_FULL`。

多个同步 EDN 方法可以直接使用导出宏，省去每个模块重复编写 wrapper：

```rust
calcit_native_ffi::export_edn_buffer_method_v1!(
  demo_calcit_ffi_v1,
  demo
);
```

详见：

- [ABI 协议 / ABI protocol](docs/ABI.md)
- [迁移指南 / Migration guide](docs/Migration.md)

## English

This crate maintains the transport contract between the Calcit runtime and
Rust `cdylib` modules. It removes repeated descriptor validation, buffer
ownership, Cirru EDN codecs, asynchronous-task, and blocking-callback adapters.

The boundary is intentionally narrow. Business arguments, threads,
registries, cancellation state, and server lifecycles remain module-owned.

Explicitly invoke `export_buffer_abi_v1!` and, where needed,
`export_async_abi_v1!` in the final dynamic library. This keeps allocation and
release within the same linked artifact.

Use `export_edn_buffer_method_v1!` for synchronous EDN methods to generate the
three-argument C export while keeping each public symbol explicit.

`BackpressurePolicy::default()` preserves the existing 1ms retry behavior.
New code that must not wait indefinitely should use a bounded policy or handle
`status::QUEUE_FULL` after a single `enqueue` call.

## Compatibility

- Rust edition: 2024
- Buffer protocol: v1
- Async and blocking protocol: v1
- Default feature `edn`: Cirru EDN request/result adapters
- `--no-default-features`: raw ABI, buffer, and async transport only

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo package
```

Every Issue and PR must contain both Chinese and English descriptions.
