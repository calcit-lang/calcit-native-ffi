# Development guide / 开发指南

## 中文

- 本仓库只维护 Calcit native C ABI 与通用 transport adapter，不加入具体模块业务逻辑。
- ABI struct 必须使用 `#[repr(C)]`；跨边界不得暴露 Rust enum、`Vec` 或 trait object。
- v1 已发布字段、数值常量和 symbol 名称不可在 minor release 中改变。
- 所有 unsafe operation 都要有局部 SAFETY 注释和正常/异常路径测试。
- Issue、PR 标题和正文必须中英双语。
- 每次提交前在 `editing-history/` 增加时间戳文件，记录中英双语改动摘要。

## English

- This repository contains only the Calcit native C ABI and common transport
  adapters, never module-specific business logic.
- ABI structures use `#[repr(C)]`; Rust enums, `Vec`, and trait objects never
  cross the boundary.
- Published v1 fields, numeric constants, and symbol names are immutable in a
  minor release.
- Every unsafe operation needs a local SAFETY comment and success/failure tests.
- Issue and PR titles and bodies must be bilingual in Chinese and English.
- Add a timestamped bilingual note under `editing-history/` before each commit.

## Verification / 验证

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo package
```
