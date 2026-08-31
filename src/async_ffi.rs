use std::thread::sleep;
use std::time::{Duration, Instant};

use crate::{
    AsyncResponseResolve, AsyncTaskCancel, CalcitFfiAsyncHostV1, CalcitFfiAsyncTaskV1, event_kind,
    response_outcome, status,
};

#[derive(Debug, Clone, Copy)]
pub struct BackpressurePolicy {
    pub retry_delay: Duration,
    pub max_retries: Option<u32>,
    pub max_wait: Option<Duration>,
}

pub const DEFAULT_BACKPRESSURE_RETRY_DELAY: Duration = Duration::from_millis(1);
pub const DEFAULT_BACKPRESSURE_MAX_WAIT: Duration = Duration::from_secs(5);
const CANCELLATION_POLL_INTERVAL: Duration = Duration::from_millis(10);

impl BackpressurePolicy {
    pub const fn unbounded(retry_delay: Duration) -> Self {
        Self {
            retry_delay,
            max_retries: None,
            max_wait: None,
        }
    }

    pub const fn bounded(retry_delay: Duration, max_retries: u32) -> Self {
        Self {
            retry_delay,
            max_retries: Some(max_retries),
            max_wait: None,
        }
    }

    pub const fn deadline(retry_delay: Duration, max_wait: Duration) -> Self {
        Self {
            retry_delay,
            max_retries: None,
            max_wait: Some(max_wait),
        }
    }

    pub const fn bounded_with_deadline(
        retry_delay: Duration,
        max_retries: u32,
        max_wait: Duration,
    ) -> Self {
        Self {
            retry_delay,
            max_retries: Some(max_retries),
            max_wait: Some(max_wait),
        }
    }
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        Self::deadline(
            DEFAULT_BACKPRESSURE_RETRY_DELAY,
            DEFAULT_BACKPRESSURE_MAX_WAIT,
        )
    }
}

pub fn enqueue(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    kind: u32,
    response_handle: u64,
    payload: &[u8],
) -> i32 {
    let Some(enqueue) = host.enqueue else {
        return status::INVALID_PAYLOAD;
    };
    // SAFETY: copied host function pointers remain valid while the host owns the
    // task, and the payload is readable for this call.
    unsafe {
        enqueue(
            host.context,
            task.handle,
            kind,
            response_handle,
            payload.as_ptr(),
            payload.len(),
        )
    }
}

pub fn enqueue_with_backpressure(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    kind: u32,
    response_handle: u64,
    payload: &[u8],
    policy: BackpressurePolicy,
) -> i32 {
    enqueue_with_backpressure_until(host, task, kind, response_handle, payload, policy, || true)
}

/// Retry a queue-full enqueue while the deadline/retry policy and caller-owned
/// cancellation predicate both allow it.
///
/// The predicate is checked before the first attempt and while waiting between
/// retries. Cancellation returns `HANDLE_CLOSING`; exhausting retries or the
/// deadline preserves the host's `QUEUE_FULL` result. A successful enqueue or
/// any other host status returns immediately.
pub fn enqueue_with_backpressure_until<F>(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    kind: u32,
    response_handle: u64,
    payload: &[u8],
    policy: BackpressurePolicy,
    mut should_continue: F,
) -> i32
where
    F: FnMut() -> bool,
{
    let started_at = Instant::now();
    let mut retries = 0;
    loop {
        if !should_continue() {
            return status::HANDLE_CLOSING;
        }
        if retries > 0
            && policy
                .max_wait
                .is_some_and(|max_wait| started_at.elapsed() >= max_wait)
        {
            return status::QUEUE_FULL;
        }
        let result = enqueue(host, task, kind, response_handle, payload);
        if result != status::QUEUE_FULL {
            return result;
        }
        if policy.max_retries.is_some_and(|limit| retries >= limit) {
            return result;
        }
        let retry_delay = match policy.max_wait {
            Some(max_wait) => {
                let Some(remaining) = max_wait.checked_sub(started_at.elapsed()) else {
                    return result;
                };
                if remaining.is_zero() {
                    return result;
                }
                policy.retry_delay.min(remaining)
            }
            None => policy.retry_delay,
        };
        retries += 1;
        if !wait_for_retry(retry_delay, &mut should_continue) {
            return status::HANDLE_CLOSING;
        }
    }
}

fn wait_for_retry<F>(duration: Duration, should_continue: &mut F) -> bool
where
    F: FnMut() -> bool,
{
    if duration.is_zero() {
        return should_continue();
    }
    let started_at = Instant::now();
    loop {
        if !should_continue() {
            return false;
        }
        let Some(remaining) = duration.checked_sub(started_at.elapsed()) else {
            return true;
        };
        sleep(remaining.min(CANCELLATION_POLL_INTERVAL));
    }
}

pub fn configure_task(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    kind: u32,
    flags: u32,
    task_context: u64,
    cancel: AsyncTaskCancel,
) -> i32 {
    let Some(configure) = host.configure_task else {
        return status::INVALID_PAYLOAD;
    };
    // SAFETY: copied host function pointers remain valid while the task runs.
    unsafe {
        configure(
            host.context,
            task.handle,
            kind,
            flags,
            task_context,
            Some(cancel),
        )
    }
}

pub fn open_response(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    response_context: u64,
    timeout_ms: u64,
    resolve: AsyncResponseResolve,
) -> Result<u64, i32> {
    let Some(open) = host.open_response else {
        return Err(status::INVALID_PAYLOAD);
    };
    let mut response_handle = 0;
    // SAFETY: copied host function pointers remain valid while the task runs and
    // the output pointer is writable for this call.
    let result = unsafe {
        open(
            host.context,
            task.handle,
            response_context,
            timeout_ms,
            Some(resolve),
            &mut response_handle,
        )
    };
    if result == status::OK {
        Ok(response_handle)
    } else {
        Err(result)
    }
}

pub fn resolve_response(
    callback: AsyncResponseResolve,
    response_context: u64,
    response_handle: u64,
    outcome: u32,
    payload: &[u8],
) -> i32 {
    // SAFETY: the response callback is copied from the host and payload remains
    // readable for this call.
    unsafe {
        callback(
            response_context,
            response_handle,
            outcome,
            payload.as_ptr(),
            payload.len(),
        )
    }
}

pub fn resolve_response_ok(
    callback: AsyncResponseResolve,
    response_context: u64,
    response_handle: u64,
    payload: &[u8],
) -> i32 {
    resolve_response(
        callback,
        response_context,
        response_handle,
        response_outcome::RESOLVE,
        payload,
    )
}

pub fn resolve_response_error(
    callback: AsyncResponseResolve,
    response_context: u64,
    response_handle: u64,
    payload: &[u8],
) -> i32 {
    resolve_response(
        callback,
        response_context,
        response_handle,
        response_outcome::REJECT,
        payload,
    )
}

pub fn publish_complete(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    policy: BackpressurePolicy,
) -> i32 {
    enqueue_with_backpressure(host, task, event_kind::COMPLETE, 0, b"&unit", policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ASYNC_PROTOCOL_VERSION, task_kind};
    use std::mem::size_of;
    use std::sync::atomic::{AtomicU32, Ordering};

    unsafe extern "C" fn queue_then_accept(
        context: u64,
        _: u64,
        _: u32,
        _: u64,
        _: *const u8,
        _: usize,
    ) -> i32 {
        // SAFETY: each test keeps its local counter alive for the synchronous call.
        let calls = unsafe { &*(context as *const AtomicU32) };
        if calls.fetch_add(1, Ordering::SeqCst) == 0 {
            status::QUEUE_FULL
        } else {
            status::OK
        }
    }

    unsafe extern "C" fn queue_forever(
        context: u64,
        _: u64,
        _: u32,
        _: u64,
        _: *const u8,
        _: usize,
    ) -> i32 {
        // SAFETY: each test keeps its local counter alive for the synchronous call.
        let calls = unsafe { &*(context as *const AtomicU32) };
        calls.fetch_add(1, Ordering::SeqCst);
        status::QUEUE_FULL
    }

    fn task() -> CalcitFfiAsyncTaskV1 {
        CalcitFfiAsyncTaskV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: size_of::<CalcitFfiAsyncTaskV1>() as u32,
            handle: 7,
            kind: task_kind::ONE_SHOT,
            flags: 0,
        }
    }

    fn host(enqueue: crate::AsyncHostEnqueue, calls: &AtomicU32) -> CalcitFfiAsyncHostV1 {
        CalcitFfiAsyncHostV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: size_of::<CalcitFfiAsyncHostV1>() as u32,
            context: (calls as *const AtomicU32) as u64,
            enqueue: Some(enqueue),
            configure_task: None,
            open_response: None,
        }
    }

    #[test]
    fn backpressure_policy_retries_queue_full() {
        let calls = AtomicU32::new(0);
        let policy = BackpressurePolicy::bounded(Duration::ZERO, 1);
        assert_eq!(
            enqueue_with_backpressure(
                host(queue_then_accept, &calls),
                task(),
                event_kind::EMIT,
                0,
                b"[]",
                policy
            ),
            status::OK
        );
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn default_backpressure_policy_has_a_deadline() {
        let policy = BackpressurePolicy::default();
        assert_eq!(policy.retry_delay, DEFAULT_BACKPRESSURE_RETRY_DELAY);
        assert_eq!(policy.max_wait, Some(DEFAULT_BACKPRESSURE_MAX_WAIT));
        assert_eq!(policy.max_retries, None);
    }

    #[test]
    fn zero_deadline_attempts_once_and_returns_queue_full() {
        let calls = AtomicU32::new(0);
        assert_eq!(
            enqueue_with_backpressure(
                host(queue_forever, &calls),
                task(),
                event_kind::EMIT,
                0,
                b"[]",
                BackpressurePolicy::deadline(Duration::from_secs(1), Duration::ZERO),
            ),
            status::QUEUE_FULL
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn deadline_does_not_enqueue_again_after_wait_expires() {
        let calls = AtomicU32::new(0);
        let started_at = Instant::now();
        let policy =
            BackpressurePolicy::deadline(Duration::from_secs(1), Duration::from_millis(10));
        assert_eq!(
            enqueue_with_backpressure(
                host(queue_forever, &calls),
                task(),
                event_kind::EMIT,
                0,
                b"[]",
                policy,
            ),
            status::QUEUE_FULL
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(started_at.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn cancellation_before_first_attempt_does_not_enqueue() {
        let calls = AtomicU32::new(0);
        assert_eq!(
            enqueue_with_backpressure_until(
                host(queue_forever, &calls),
                task(),
                event_kind::EMIT,
                0,
                b"[]",
                BackpressurePolicy::default(),
                || false,
            ),
            status::HANDLE_CLOSING
        );
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn cancellation_interrupts_retry_wait() {
        let calls = AtomicU32::new(0);
        let mut checks = 0;
        let started_at = Instant::now();
        assert_eq!(
            enqueue_with_backpressure_until(
                host(queue_forever, &calls),
                task(),
                event_kind::EMIT,
                0,
                b"[]",
                BackpressurePolicy::deadline(Duration::from_secs(1), Duration::from_secs(5)),
                || {
                    checks += 1;
                    checks < 3
                },
            ),
            status::HANDLE_CLOSING
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(started_at.elapsed() < Duration::from_millis(500));
    }
}
