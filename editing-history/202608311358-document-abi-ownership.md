# 明确共享 ABI 职责 / Document shared ABI ownership

## 中文

- 明确本仓库是生产使用的共享 ABI/helper crate，不是 dylib 模板或业务 runtime。
- 记录 `src/abi.rs` 的 source-of-truth 地位，以及 core 与业务模块保留的职责。
- 增加兼容/发布规则、consumer matrix 和跨仓库 Issue 索引。
- 在 AGENTS.md 中补充跨消费方 smoke 与不兼容协议升级要求。

## English

- Mark this repository as the production shared ABI/helper crate rather than a
  dylib template or business runtime.
- Record `src/abi.rs` as the source of truth and preserve core/module-owned
  responsibilities.
- Add compatibility/release rules, a consumer matrix, and cross-repository
  tracking links.
- Add cross-consumer smoke and incompatible-protocol release requirements to
  AGENTS.md.
