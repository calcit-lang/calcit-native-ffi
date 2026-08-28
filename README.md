# calcit-native-ffi

Calcit native 模块共用的稳定 C ABI 类型与适配器。

Stable C ABI types and adapters shared by Calcit native modules.

## 中文

该 crate 维护 Calcit runtime 与 Rust `cdylib` 之间的传输协议，避免每个
native 模块复制 descriptor、buffer ownership、Cirru EDN 编解码、异步任务和
blocking callback 模板。

Calcit host 也可以关闭默认 feature，仅复用 symbol 常量、function pointer
签名、resource token 常量和 C-layout descriptor。动态加载、task/resource
registry 与生命周期仍由 host 负责。

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

`BackpressurePolicy::default()` 每 1ms 重试，并在 5 秒后返回
`status::QUEUE_FULL`，避免 host 队列持续饱和时永久占住 native worker。按次数
限制可用 `bounded`，按时间限制可用 `deadline`，两者同时限制可用
`bounded_with_deadline`；只有明确需要旧行为时才使用 `unbounded`。

普通 raw payload `emit` 应使用 `enqueue_with_backpressure_until`，EDN callback
应使用 `publish_emit_until`，接入模块自己的取消状态；
它会在首次 enqueue 前及等待期间检查 predicate，取消时返回
`status::HANDLE_CLOSING`，最长 10ms 重新检查一次。`complete` / `fail` 是任务
terminal 事件，不应被业务取消 predicate 跳过；应使用普通背压 helper，让 host
为 terminal 事件预留的容量完成收尾。

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

The Calcit host may disable default features and reuse only symbol constants,
function-pointer signatures, resource-token constants, and C-layout
descriptors. Dynamic loading, task/resource registries, and lifecycle remain
host-owned.

The boundary is intentionally narrow. Business arguments, threads,
registries, cancellation state, and server lifecycles remain module-owned.

Explicitly invoke `export_buffer_abi_v1!` and, where needed,
`export_async_abi_v1!` in the final dynamic library. This keeps allocation and
release within the same linked artifact.

Use `export_edn_buffer_method_v1!` for synchronous EDN methods to generate the
three-argument C export while keeping each public symbol explicit.

`BackpressurePolicy::default()` retries every 1ms and returns
`status::QUEUE_FULL` after five seconds, so a permanently saturated host queue
cannot retain a native worker forever. Use `bounded` for a retry limit,
`deadline` for a time limit, `bounded_with_deadline` for both, and `unbounded`
only when the legacy behavior is explicitly required.

Ordinary raw-payload `emit` paths should use
`enqueue_with_backpressure_until`, while EDN callbacks should use
`publish_emit_until`, to connect module-owned cancellation state. The
transport checks the predicate before the first enqueue and while waiting,
returns `status::HANDLE_CLOSING` on cancellation, and polls at most every 10ms.
`complete` and `fail` are terminal events and must not be skipped by the
business cancellation predicate; use the regular backpressure helper so the
host's reserved terminal capacity can close them.

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
