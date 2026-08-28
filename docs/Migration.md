# Migration guide / 迁移指南

## 中文

1. 添加 `calcit_native_ffi` 依赖，并保持 `cirru_edn` 与公共 crate 使用兼容版本。
2. 删除模块内重复的 `CalcitFfi*` structs、function pointer aliases、status/event
   constants、descriptor copy、buffer codec 和 panic adapter。
3. 在最终 `cdylib` crate root 调用 `export_buffer_abi_v1!()`；异步或 blocking
   模块还要调用 `export_async_abi_v1!()`。
4. 保留每个导出业务函数；同步 EDN 方法优先使用
   `export_edn_buffer_method_v1!`，需要自定义边界时再直接调用
   `run_buffer_adapter`、`run_blocking_adapter` 或 async helpers。
5. 保留模块自己的 cancel handler、thread state、registry 和 terminal ordering。
6. 普通 raw payload `emit` 使用 `enqueue_with_backpressure_until`，EDN callback
   使用 `publish_emit_until`，连接模块自己的取消状态；`complete` / `fail` 不使用
   该 predicate，以免取消路径跳过 terminal 收尾。
7. 默认背压有 5 秒截止时间。只有能证明无限等待属于协议要求时才显式使用
   `BackpressurePolicy::unbounded`。
8. 通过真实 release dylib smoke 验证 symbol、请求、响应、队列饱和、取消和
   buffer free。

迁移应保持外部 symbol 名称与 EDN payload 不变。不要在同一 PR 顺便调整业务协议。

## English

1. Add `calcit_native_ffi` and keep a compatible `cirru_edn` version.
2. Remove local copies of `CalcitFfi*` structures, function-pointer aliases,
   status/event constants, descriptor copying, buffer codecs, and panic
   adapters.
3. Invoke `export_buffer_abi_v1!()` in the final `cdylib` root. Async or
   blocking modules also invoke `export_async_abi_v1!()`.
4. Keep each business export. Prefer `export_edn_buffer_method_v1!` for
   synchronous EDN methods; call `run_buffer_adapter`, `run_blocking_adapter`,
   or the async helpers directly when the boundary needs custom behavior.
5. Keep module-specific cancellation, thread state, registries, and terminal
   ordering local.
6. Connect ordinary raw-payload `emit` calls to module-owned cancellation
   through `enqueue_with_backpressure_until`, and use `publish_emit_until` for
   EDN callbacks. Do not apply that predicate to `complete` or `fail`, because
   cancellation must not skip terminal cleanup.
7. Default backpressure has a five-second deadline. Use
   `BackpressurePolicy::unbounded` only when indefinite waiting is an explicit
   protocol requirement.
8. Use a real release dylib smoke test for symbols, requests, responses, queue
   saturation, cancellation, and buffer release.

A migration keeps public symbol names and EDN payloads unchanged. Do not mix a
business-protocol redesign into the extraction PR.
