# calcit-native-ffi

Calcit native 模块共用的稳定 C ABI 类型与适配器。

Stable C ABI types and adapters shared by Calcit native modules.

## Repository status / 仓库状态

`calcit-native-ffi` 是生产使用的共享 ABI/helper crate，不是
`dylib-workflow` 模板，也不是具体 native 模块的运行时。`src/abi.rs` 是 buffer、
async、blocking 与 resource v1 的协议版本、symbol、function pointer 和
`#[repr(C)]` descriptor 的唯一 Rust source of truth。

`calcit-native-ffi` is the production shared ABI/helper crate. It is neither
the `dylib-workflow` template nor a runtime for a specific native module.
`src/abi.rs` is the single Rust source of truth for buffer, async, blocking,
and resource-v1 versions, symbols, function pointers, and `#[repr(C)]`
descriptors.

Calcit core 负责动态加载、task/resource registry、callback scheduling、lease、
取消和错误映射；业务模块负责线程、连接、业务状态与 terminal ordering。本 crate
只拥有跨边界 contract 和可复用 transport adapters。

Calcit core owns dynamic loading, task/resource registries, callback
scheduling, leases, cancellation, and error mapping. Business modules own
threads, connections, domain state, and terminal ordering. This crate owns
only the cross-boundary contract and reusable transport adapters.

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

Published v1 field order, numeric values, and exported symbol names are
immutable within compatible `0.1.x` releases. Additive helpers may ship in a
patch release; any ABI-incompatible contract requires a new protocol suffix
and an explicit migration path. Releases are cut when a shared contract or
adapter change is needed by consumers, rather than for template-only changes.

已发布的 v1 字段顺序、数值和导出 symbol 在兼容的 `0.1.x` 版本内保持不变。新增
helper 可以通过 patch release 发布；不兼容 ABI 必须使用新的协议 suffix，并提供
明确迁移路径。仅在消费方需要共享 contract 或 adapter 更新时发版，不跟随模板项目
做无意义版本迭代。

## Consumers and tracking / 消费方与追踪

| Consumer / 消费方 | Shared contract / 共享边界 | Tracking / 追踪 |
| --- | --- | --- |
| [Calcit core](https://github.com/calcit-lang/calcit) | raw ABI with `default-features = false`; host lifecycle remains in core / 关闭默认 feature 复用 raw ABI，host 生命周期留在 core | [calcit#544](https://github.com/calcit-lang/calcit/issues/544) |
| [calcit-bindgen](https://github.com/calcit-lang/calcit-bindgen) | generated adapters import public ABI/helper APIs / 生成 adapter 引用公开 ABI/helper API | [calcit-bindgen#3](https://github.com/calcit-lang/calcit-bindgen/issues/3) |
| extracted `caps` tool / 拆分后的 `caps` 工具 | verifier consumes ABI constants and descriptors without depending on core / verifier 不依赖 core，直接消费 ABI constants/descriptors | [calcit#546](https://github.com/calcit-lang/calcit/issues/546) |
| native modules / 原生模块 | macros, codecs, descriptors, bounded backpressure / 宏、编解码、descriptor 与有界背压 | [migration guide](docs/Migration.md) |

Repository-boundary and cross-consumer work is tracked in
[issue #7](https://github.com/calcit-lang/calcit-native-ffi/issues/7) and the
[Calcit modularization index](https://github.com/calcit-lang/calcit/issues/549).
仓库边界与跨消费方工作统一由上述两个 Issue 索引。

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo package
```

Every Issue and PR must contain both Chinese and English descriptions.
