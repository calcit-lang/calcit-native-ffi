use std::panic::{AssertUnwindSafe, catch_unwind};

use cirru_edn::{Edn, EdnListView};

use crate::{
    CalcitFfiAsyncTaskV1, CalcitFfiBlockingHostV1, CalcitFfiBuffer, buffer_status,
    copy_blocking_host, copy_buffer, copy_task_descriptor, decode_edn, decode_request, encode_edn,
    status, write_output,
};

pub fn invoke_blocking_callback(
    host: CalcitFfiBlockingHostV1,
    task: CalcitFfiAsyncTaskV1,
    args: Vec<Edn>,
) -> Result<Edn, String> {
    let invoke = host
        .invoke
        .ok_or_else(|| "blocking host is missing invoke".to_owned())?;
    let free_buffer = host
        .free_buffer
        .ok_or_else(|| "blocking host is missing free_buffer".to_owned())?;
    let payload = encode_edn(&Edn::List(EdnListView(args)))?;
    let mut output = CalcitFfiBuffer::empty();
    // SAFETY: copied host callbacks remain valid while the task is active.
    let callback_status = unsafe {
        invoke(
            host.context,
            task.handle,
            payload.as_ptr(),
            payload.len(),
            &mut output,
        )
    };
    let has_output = !output.ptr.is_null() || output.len != 0 || output.cap != 0;
    if !has_output {
        return Err(
            if matches!(callback_status, status::OK | status::CALLBACK_ERROR) {
                "Calcit callback returned no output buffer".to_owned()
            } else {
                format!("Calcit host rejected blocking callback with status {callback_status}")
            },
        );
    }
    // SAFETY: the host owns and keeps this validated output alive until free_buffer.
    let copied = unsafe { copy_buffer(output) };
    // SAFETY: the copied host callback owns this exact output allocation.
    let free_status = unsafe { free_buffer(host.context, task.handle, output) };
    if free_status != status::OK {
        return Err(format!(
            "Calcit host rejected callback buffer release with status {free_status}"
        ));
    }
    let bytes = copied.map_err(|error| error.to_string())?;
    if callback_status == status::OK {
        // SAFETY: bytes are now owned by this module for the duration of parsing.
        unsafe { decode_edn(bytes.as_ptr(), bytes.len()) }
    } else if callback_status == status::CALLBACK_ERROR {
        Err(String::from_utf8_lossy(&bytes).into_owned())
    } else {
        Err(format!(
            "Calcit host rejected blocking callback with status {callback_status}"
        ))
    }
}

pub fn finish_blocking_task(host: CalcitFfiBlockingHostV1, task: CalcitFfiAsyncTaskV1) -> i32 {
    let Some(finish) = host.finish else {
        return status::INVALID_PAYLOAD;
    };
    // SAFETY: copied host callbacks remain valid while the task is active.
    unsafe { finish(host.context, task.handle) }
}

/// Run a method behind the versioned blocking callback ABI.
///
/// # Safety
///
/// Request bytes must be readable for `request_len`; `task` and `host` must
/// satisfy their versioned descriptor contracts; and `output` must be writable
/// for one [`CalcitFfiBuffer`]. Host callbacks must remain valid until `method`
/// returns.
pub unsafe fn run_blocking_adapter<F>(
    request_ptr: *const u8,
    request_len: usize,
    task: *const CalcitFfiAsyncTaskV1,
    host: *const CalcitFfiBlockingHostV1,
    output: *mut CalcitFfiBuffer,
    method: F,
) -> i32
where
    F: FnOnce(Vec<Edn>, CalcitFfiAsyncTaskV1, CalcitFfiBlockingHostV1) -> Result<Edn, String>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: descriptors follow the exported blocking ABI contract.
        let task = unsafe { copy_task_descriptor(task) }
            .map_err(|error| format!("invalid blocking task descriptor: {error}"))?;
        // SAFETY: descriptor validation copies only after checking its versioned header.
        let host = unsafe { copy_blocking_host(host) }
            .map_err(|error| format!("invalid blocking host descriptor: {error}"))?;
        if host.invoke.is_none() || host.free_buffer.is_none() {
            return Err("blocking host is missing required operations".to_owned());
        }
        // SAFETY: request bytes remain readable for this call and are copied by the decoder.
        let args = unsafe { decode_request(request_ptr, request_len) }?;
        method(args, task, host).and_then(|value| encode_edn(&value))
    }));
    match result {
        Ok(Ok(bytes)) => unsafe { write_output(output, bytes) },
        Ok(Err(error)) => {
            let _ = unsafe { write_output(output, error.into_bytes()) };
            buffer_status::ERROR
        }
        Err(_) => {
            let _ =
                unsafe { write_output(output, b"Calcit FFI blocking adapter panicked".to_vec()) };
            status::INTERNAL_ERROR
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ASYNC_PROTOCOL_VERSION, free_buffer, task_kind};
    use std::mem::size_of;
    use std::sync::atomic::{AtomicBool, Ordering};

    static FREED: AtomicBool = AtomicBool::new(false);

    unsafe extern "C" fn echo_invoke(
        _: u64,
        _: u64,
        payload_ptr: *const u8,
        payload_len: usize,
        output: *mut CalcitFfiBuffer,
    ) -> i32 {
        // SAFETY: the test caller provides an encoded readable EDN payload.
        let bytes = unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) }.to_vec();
        // SAFETY: the test caller provides a writable output slot.
        unsafe { write_output(output, bytes) }
    }

    unsafe extern "C" fn echo_free(_: u64, _: u64, buffer: CalcitFfiBuffer) -> i32 {
        FREED.store(true, Ordering::SeqCst);
        // SAFETY: echo_invoke produced this exact allocation in this test crate.
        unsafe { free_buffer(buffer) };
        status::OK
    }

    #[test]
    fn blocking_callback_copies_and_releases_host_buffer() {
        FREED.store(false, Ordering::SeqCst);
        let host = CalcitFfiBlockingHostV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: size_of::<CalcitFfiBlockingHostV1>() as u32,
            context: 0,
            invoke: Some(echo_invoke),
            finish: None,
            free_buffer: Some(echo_free),
        };
        let task = CalcitFfiAsyncTaskV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: size_of::<CalcitFfiAsyncTaskV1>() as u32,
            handle: 9,
            kind: task_kind::ONE_SHOT,
            flags: 0,
        };
        let expected = Edn::List(EdnListView(vec![Edn::str("ok")]));
        let result = invoke_blocking_callback(host, task, vec![Edn::str("ok")]).expect("callback");
        assert_eq!(result, expected);
        assert!(FREED.load(Ordering::SeqCst));
    }
}
