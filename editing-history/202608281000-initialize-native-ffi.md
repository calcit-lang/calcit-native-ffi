# 初始化共享 native FFI crate / Initialize the shared native FFI crate

## 中文

- 冻结 Calcit C-safe buffer、async 与 blocking ABI v1 的公共 Rust 表达。
- 抽取 descriptor 校验、buffer ownership、EDN adapter、backpressure 与 blocking callback helpers。
- 通过宏在最终 `cdylib` 生成稳定协议 symbols，确保 allocator ownership 正确。
- 增加 ABI、迁移、贡献文档与跨 feature 测试门禁。

## English

- Freeze the shared Rust representation of Calcit C-safe buffer, async, and blocking ABI v1.
- Extract descriptor validation, buffer ownership, EDN adapters, backpressure, and blocking-callback helpers.
- Generate stable protocol symbols in the final `cdylib` through macros so allocator ownership remains correct.
- Add ABI, migration, and contribution documentation plus cross-feature quality gates.
