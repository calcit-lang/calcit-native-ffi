use std::panic::{AssertUnwindSafe, catch_unwind};
use std::slice;

use cirru_edn::{Edn, EdnListView};

use crate::{
    BackpressurePolicy, CalcitFfiAsyncHostV1, CalcitFfiAsyncTaskV1, CalcitFfiBuffer,
    MAX_BUFFER_BYTES, buffer_status, event_kind, status,
};

/// Decode a call-scoped Cirru EDN list into owned arguments.
///
/// # Safety
///
/// When `request_len` is non-zero, `request_ptr` must be readable for exactly
/// `request_len` bytes during this call.
pub unsafe fn decode_request(
    request_ptr: *const u8,
    request_len: usize,
) -> Result<Vec<Edn>, String> {
    let data = unsafe { decode_edn(request_ptr, request_len) }?;
    let Edn::List(EdnListView(args)) = data else {
        return Err("FFI request must be a Cirru EDN list".to_owned());
    };
    Ok(args)
}

/// Decode a call-scoped Cirru EDN value.
///
/// # Safety
///
/// When `request_len` is non-zero, `request_ptr` must be readable for exactly
/// `request_len` bytes during this call.
pub unsafe fn decode_edn(request_ptr: *const u8, request_len: usize) -> Result<Edn, String> {
    if request_ptr.is_null() && request_len != 0 {
        return Err("FFI request pointer is null".to_owned());
    }
    if request_len > MAX_BUFFER_BYTES {
        return Err(format!("FFI request exceeds {MAX_BUFFER_BYTES} bytes"));
    }
    let bytes = if request_len == 0 {
        &[]
    } else {
        // SAFETY: the host keeps request bytes readable for this exported call.
        unsafe { slice::from_raw_parts(request_ptr, request_len) }
    };
    let source =
        std::str::from_utf8(bytes).map_err(|error| format!("FFI request is not UTF-8: {error}"))?;
    cirru_edn::parse(source).map_err(|error| format!("FFI request is not valid Cirru EDN: {error}"))
}

pub fn encode_edn(value: &Edn) -> Result<Vec<u8>, String> {
    cirru_edn::format(value, true)
        .map(String::into_bytes)
        .map_err(|error| format!("failed to encode Cirru EDN: {error}"))
}

pub fn encode_callback_args(values: Vec<Edn>) -> Result<Vec<u8>, String> {
    encode_edn(&Edn::List(EdnListView(values)))
}

pub fn encode_failure(message: impl Into<String>) -> Vec<u8> {
    encode_edn(&Edn::str(message.into()))
        .unwrap_or_else(|_| b"|failed-to-encode-ffi-error".to_vec())
}

/// Run a synchronous EDN method behind the versioned buffer ABI.
///
/// # Safety
///
/// `request_ptr` follows [`decode_request`]'s readable-memory contract and
/// `output` follows [`crate::write_output`]'s writable-slot contract.
pub unsafe fn run_buffer_adapter<F>(
    request_ptr: *const u8,
    request_len: usize,
    output: *mut CalcitFfiBuffer,
    method: F,
) -> i32
where
    F: FnOnce(Vec<Edn>) -> Result<Edn, String>,
{
    let result = catch_unwind(AssertUnwindSafe(|| {
        // SAFETY: forwarded from the exported buffer ABI contract.
        let args = unsafe { decode_request(request_ptr, request_len) }?;
        method(args).and_then(|value| encode_edn(&value))
    }));
    match result {
        Ok(Ok(bytes)) => unsafe { crate::write_output(output, bytes) },
        Ok(Err(error)) => {
            let _ = unsafe { crate::write_output(output, error.into_bytes()) };
            buffer_status::ERROR
        }
        Err(_) => {
            let _ = unsafe {
                crate::write_output(output, b"Calcit FFI buffer adapter panicked".to_vec())
            };
            status::INTERNAL_ERROR
        }
    }
}

pub fn publish_emit(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    args: Vec<Edn>,
    policy: BackpressurePolicy,
) -> i32 {
    match encode_callback_args(args) {
        Ok(payload) => {
            crate::enqueue_with_backpressure(host, task, event_kind::EMIT, 0, &payload, policy)
        }
        Err(_) => status::INTERNAL_ERROR,
    }
}

pub fn publish_failure(
    host: CalcitFfiAsyncHostV1,
    task: CalcitFfiAsyncTaskV1,
    message: impl Into<String>,
    policy: BackpressurePolicy,
) -> i32 {
    let payload = encode_failure(message);
    crate::enqueue_with_backpressure(host, task, event_kind::FAIL, 0, &payload, policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{copy_buffer, free_buffer};

    #[test]
    fn buffer_adapter_round_trips_edn() {
        let expected = Edn::List(EdnListView(vec![Edn::Number(1.0), Edn::Number(2.0)]));
        let request = encode_edn(&expected).expect("encode request");
        let mut output = CalcitFfiBuffer::empty();
        let status = unsafe {
            run_buffer_adapter(request.as_ptr(), request.len(), &mut output, |args| {
                Ok(Edn::List(EdnListView(args)))
            })
        };
        assert_eq!(status, status::OK);
        let bytes = unsafe { copy_buffer(output) }.expect("copy output");
        assert_eq!(
            cirru_edn::parse(std::str::from_utf8(&bytes).expect("utf8")).expect("edn"),
            expected
        );
        unsafe { free_buffer(output) };
    }
}
