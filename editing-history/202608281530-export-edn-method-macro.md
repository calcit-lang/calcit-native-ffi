# 同步 EDN 导出宏 / Synchronous EDN export macro

## 中文

- 新增 `export_edn_buffer_method_v1!`，统一生成同步 EDN 方法的 C ABI wrapper。
- 业务函数仍由模块维护；共享 crate 负责请求解码、panic 隔离、响应编码和 buffer ownership。
- 测试覆盖成功返回与业务错误返回，文档补充多方法模块的迁移示例。

## English

- Added `export_edn_buffer_method_v1!` to generate the C ABI wrapper for synchronous EDN methods.
- Domain handlers remain module-owned while the shared crate handles request decoding, panic isolation, response encoding, and buffer ownership.
- Covered both success and domain-error responses and documented the multi-method migration pattern.
