# Development guide / 开发指南

## 中文

- 本仓库只维护 Calcit native C ABI 与通用 transport adapter，不加入具体模块业务逻辑。
- ABI struct 必须使用 `#[repr(C)]`；跨边界不得暴露 Rust enum、`Vec` 或 trait object。
- v1 已发布字段、数值常量和 symbol 名称不可在 minor release 中改变。
- 所有 unsafe operation 都要有局部 SAFETY 注释和正常/异常路径测试。
- Issue、PR 标题和正文必须中英双语。
- 每次提交前在 `editing-history/` 增加时间戳文件，记录中英双语改动摘要。
- `src/abi.rs` 是稳定 raw ABI 的唯一 Rust source of truth；不得在 core、bindgen、
  `caps` 或 native modules 中复制协议版本、symbol suffix、function pointer 或
  `#[repr(C)]` descriptor。
- 发布前同时验证默认 feature 与 `--no-default-features`；涉及 ABI 或 adapter 的
  改动还要在 Calcit core 和至少一个代表性 native module 中执行真实 release
  dylib smoke。
- v1 ABI 不兼容改动不能作为 `0.1.x` patch 发布；必须增加新协议 suffix，并在
  README、ABI reference、migration guide 与 consumer tracking Issue 中记录迁移。

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
- `src/abi.rs` is the single Rust source of truth for the stable raw ABI. Do
  not copy protocol versions, symbol suffixes, function pointers, or
  `#[repr(C)]` descriptors into core, bindgen, `caps`, or native modules.
- Before release, test both default features and `--no-default-features`. ABI
  or adapter changes also require a real release-dylib smoke in Calcit core
  and at least one representative native module.
- Do not publish a v1 ABI break as a `0.1.x` patch. Introduce a new protocol
  suffix and document migration in the README, ABI reference, migration guide,
  and consumer tracking Issue.

## Verification / 验证

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo test --no-default-features
cargo package
```
