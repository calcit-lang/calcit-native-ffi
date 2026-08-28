# Migration guide / 迁移指南

## 中文

1. 添加 `calcit_native_ffi` 依赖，并保持 `cirru_edn` 与公共 crate 使用兼容版本。
2. 删除模块内重复的 `CalcitFfi*` structs、function pointer aliases、status/event
   constants、descriptor copy、buffer codec 和 panic adapter。
3. 在最终 `cdylib` crate root 调用 `export_buffer_abi_v1!()`；异步或 blocking
   模块还要调用 `export_async_abi_v1!()`。
4. 保留每个导出业务函数；函数体改为调用 `run_buffer_adapter`、
   `run_blocking_adapter` 或 async helpers。
5. 保留模块自己的 cancel handler、thread state、registry 和 terminal ordering。
6. 通过真实 release dylib smoke 验证 symbol、请求、响应、取消和 buffer free。

迁移应保持外部 symbol 名称与 EDN payload 不变。不要在同一 PR 顺便调整业务协议。

## English

1. Add `calcit_native_ffi` and keep a compatible `cirru_edn` version.
2. Remove local copies of `CalcitFfi*` structures, function-pointer aliases,
   status/event constants, descriptor copying, buffer codecs, and panic
   adapters.
3. Invoke `export_buffer_abi_v1!()` in the final `cdylib` root. Async or
   blocking modules also invoke `export_async_abi_v1!()`.
4. Keep each business export and delegate its body to `run_buffer_adapter`,
   `run_blocking_adapter`, or the async helpers.
5. Keep module-specific cancellation, thread state, registries, and terminal
   ordering local.
6. Use a real release dylib smoke test for symbols, requests, responses,
   cancellation, and buffer release.

A migration keeps public symbol names and EDN payloads unchanged. Do not mix a
business-protocol redesign into the extraction PR.
