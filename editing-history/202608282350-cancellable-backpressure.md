# 可取消且有截止时间的异步背压 / Cancellable, deadline-aware async backpressure

## 中文

- 默认背压从无限重试改为每 1ms 重试、最长等待 5 秒，避免 host 队列持续饱和时永久占住 native worker。
- 新增 `deadline`、`bounded_with_deadline` 和 `enqueue_with_backpressure_until`，让模块在等待队列容量时继续响应自身取消状态。
- 新增 `publish_emit_until`，避免 EDN native 模块重新复制编码与可取消 enqueue 模板。
- 取消检查最长间隔 10ms；取消返回 `HANDLE_CLOSING`，重试次数或截止时间耗尽仍返回 `QUEUE_FULL`。
- terminal `complete` / `fail` 不应使用业务取消 predicate，应继续利用 host 预留容量可靠收尾。
- `.DS_Store` 加入忽略规则，避免本地元数据进入 crates.io 包。

## English

- Changed default backpressure from unlimited retries to 1ms retries with a five-second maximum wait, preventing a permanently saturated host queue from retaining native workers forever.
- Added `deadline`, `bounded_with_deadline`, and `enqueue_with_backpressure_until` so modules can observe their cancellation state while waiting for queue capacity.
- Added `publish_emit_until` so EDN native modules do not duplicate encoding and cancellable-enqueue templates.
- Cancellation is polled at most every 10ms; it returns `HANDLE_CLOSING`, while exhausted retry/deadline bounds preserve `QUEUE_FULL`.
- Terminal `complete` / `fail` events should not use a business cancellation predicate and should rely on host-reserved terminal capacity for reliable cleanup.
- Ignored `.DS_Store` so local filesystem metadata cannot enter the crates.io package.
