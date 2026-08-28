# 共享 host raw ABI / Share the host raw ABI

## 中文

- 补齐 buffer、async、blocking 与 resource v1 的固定 symbol、suffix 和 host call function pointer 类型。
- 为三类 C-layout descriptor 提供构造器，避免 Calcit runtime 再维护布局相同的本地定义。
- 保持动态加载、task/resource registry、lease 与 lifecycle 在 host 或业务模块侧。

## English

- Added canonical symbols, suffixes, and host-call function pointer types for buffer, async, blocking, and resource v1.
- Added constructors for the three C-layout descriptors so the Calcit runtime no longer needs layout-identical local definitions.
- Kept dynamic loading, task/resource registries, leases, and lifecycle in the host or business modules.
