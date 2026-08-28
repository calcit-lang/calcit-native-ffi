use std::thread::sleep;
use std::time::Duration;

use crate::{
    AsyncResponseResolve, AsyncTaskCancel, CalcitFfiAsyncHostV1, CalcitFfiAsyncTaskV1, event_kind,
    response_outcome, status,
};

#[derive(Debug, Clone, Copy)]
pub struct BackpressurePolicy {
    pub retry_delay: Duration,
    pub max_retries: Option<u32>,
}

impl BackpressurePolicy {
    pub const fn unbounded(retry_delay: Duration) -> Self {
        Self {
            retry_delay,
            max_retries: None,
        }
    }

    pub const fn bounded(retry_delay: Duration, max_retries: u32) -> Self {
        Self {
            retry_delay,
            max_retries: Some(max_retries),
        }
    }
}

impl Default for BackpressurePolicy {
    fn default() -> Self {
        Self::unbounded(Duration::from_millis(1))
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
    let mut retries = 0;
    loop {
        let result = enqueue(host, task, kind, response_handle, payload);
        if result != status::QUEUE_FULL {
            return result;
        }
        if policy.max_retries.is_some_and(|limit| retries >= limit) {
            return result;
        }
        retries += 1;
        sleep(policy.retry_delay);
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

    static CALLS: AtomicU32 = AtomicU32::new(0);

    unsafe extern "C" fn queue_then_accept(
        _: u64,
        _: u64,
        _: u32,
        _: u64,
        _: *const u8,
        _: usize,
    ) -> i32 {
        if CALLS.fetch_add(1, Ordering::SeqCst) == 0 {
            status::QUEUE_FULL
        } else {
            status::OK
        }
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

    #[test]
    fn backpressure_policy_retries_queue_full() {
        CALLS.store(0, Ordering::SeqCst);
        let host = CalcitFfiAsyncHostV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: size_of::<CalcitFfiAsyncHostV1>() as u32,
            context: 0,
            enqueue: Some(queue_then_accept),
            configure_task: None,
            open_response: None,
        };
        let policy = BackpressurePolicy::bounded(Duration::ZERO, 1);
        assert_eq!(
            enqueue_with_backpressure(host, task(), event_kind::EMIT, 0, b"[]", policy),
            status::OK
        );
        assert_eq!(CALLS.load(Ordering::SeqCst), 2);
    }
}
