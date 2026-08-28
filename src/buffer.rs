use std::fmt;
use std::mem::ManuallyDrop;
use std::slice;

use crate::{CalcitFfiBuffer, MAX_BUFFER_BYTES, status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferError {
    NullPointer,
    LengthExceedsCapacity { len: usize, cap: usize },
    TooLarge { len: usize, max: usize },
}

impl fmt::Display for BufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullPointer => write!(f, "buffer pointer is null while length is non-zero"),
            Self::LengthExceedsCapacity { len, cap } => {
                write!(f, "buffer length {len} exceeds capacity {cap}")
            }
            Self::TooLarge { len, max } => write!(f, "buffer length {len} exceeds maximum {max}"),
        }
    }
}

impl std::error::Error for BufferError {}

pub fn validate_buffer(buffer: CalcitFfiBuffer) -> Result<(), BufferError> {
    if buffer.len > buffer.cap {
        return Err(BufferError::LengthExceedsCapacity {
            len: buffer.len,
            cap: buffer.cap,
        });
    }
    if buffer.len > MAX_BUFFER_BYTES {
        return Err(BufferError::TooLarge {
            len: buffer.len,
            max: MAX_BUFFER_BYTES,
        });
    }
    if buffer.ptr.is_null() && buffer.len != 0 {
        return Err(BufferError::NullPointer);
    }
    Ok(())
}

/// Copy bytes from a foreign-owned buffer after validating its metadata.
///
/// # Safety
///
/// For a non-empty valid buffer, `buffer.ptr` must remain readable for
/// `buffer.len` bytes during this call. This function does not release it.
pub unsafe fn copy_buffer(buffer: CalcitFfiBuffer) -> Result<Vec<u8>, BufferError> {
    validate_buffer(buffer)?;
    if buffer.len == 0 {
        return Ok(vec![]);
    }
    // SAFETY: metadata was validated and the owner keeps the allocation alive
    // for this copy operation.
    Ok(unsafe { slice::from_raw_parts(buffer.ptr, buffer.len) }.to_vec())
}

/// Transfer ownership of `bytes` to a writable C ABI output slot.
///
/// # Safety
///
/// `output` must be null or writable for one [`CalcitFfiBuffer`]. A successful
/// caller must return the exact metadata to [`free_buffer`] in this same final
/// dynamic library exactly once.
pub unsafe fn write_output(output: *mut CalcitFfiBuffer, bytes: Vec<u8>) -> i32 {
    if output.is_null() {
        return status::INVALID_PAYLOAD;
    }
    let mut bytes = ManuallyDrop::new(bytes);
    let buffer = CalcitFfiBuffer {
        ptr: bytes.as_mut_ptr(),
        len: bytes.len(),
        cap: bytes.capacity(),
    };
    // SAFETY: the caller supplied a writable output slot for this call.
    unsafe { output.write(buffer) };
    status::OK
}

/// Release a buffer previously created by [`write_output`].
///
/// # Safety
///
/// A non-null `buffer` must contain the exact pointer, length, and capacity
/// emitted by [`write_output`] from this allocator and must not have been freed
/// before.
pub unsafe fn free_buffer(buffer: CalcitFfiBuffer) {
    if buffer.ptr.is_null() {
        return;
    }
    // SAFETY: the caller must return exactly the metadata produced by
    // `write_output` from this same final dynamic library.
    drop(unsafe { Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.cap) });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn output_round_trip_uses_matching_allocator() {
        let mut output = CalcitFfiBuffer::empty();
        assert_eq!(
            unsafe { write_output(&mut output, b"calcit".to_vec()) },
            status::OK
        );
        assert_eq!(unsafe { copy_buffer(output) }.expect("copy"), b"calcit");
        unsafe { free_buffer(output) };
    }

    #[test]
    fn invalid_buffer_metadata_is_rejected() {
        let buffer = CalcitFfiBuffer {
            ptr: ptr::null_mut(),
            len: 1,
            cap: 1,
        };
        assert_eq!(validate_buffer(buffer), Err(BufferError::NullPointer));
    }
}
