use std::fmt;
use std::mem::size_of;
use std::ptr;

pub const BUFFER_PROTOCOL_VERSION: u32 = 1;
pub const ASYNC_PROTOCOL_VERSION: u32 = 1;
pub const MAX_BUFFER_BYTES: usize = 256 * 1024 * 1024;

pub mod buffer_status {
    pub const OK: i32 = 0;
    pub const ERROR: i32 = 1;
}

pub mod status {
    pub const OK: i32 = 0;
    pub const INVALID_HANDLE: i32 = 1;
    pub const STALE_HANDLE: i32 = 2;
    pub const HANDLE_CLOSING: i32 = 3;
    pub const HANDLE_FINISHED: i32 = 4;
    pub const HANDLE_STILL_ACTIVE: i32 = 5;
    pub const HOST_CLOSING: i32 = 6;
    pub const QUEUE_FULL: i32 = 7;
    pub const INVALID_PAYLOAD: i32 = 8;
    pub const INTERNAL_ERROR: i32 = 9;
    pub const CALLBACK_ERROR: i32 = 10;
    pub const WRONG_THREAD: i32 = 11;
}

pub mod task_kind {
    pub const ONE_SHOT: u32 = 1;
    pub const STREAM: u32 = 2;
    pub const SERVER: u32 = 3;
    pub const RESPONSE: u32 = 4;
}

pub mod task_flags {
    pub const SERIAL_EVENTS: u32 = 1 << 0;
    pub const COALESCE_ALLOWED: u32 = 1 << 1;
    pub const REQUIRES_RESPONSE: u32 = 1 << 2;
    pub const KNOWN: u32 = SERIAL_EVENTS | COALESCE_ALLOWED | REQUIRES_RESPONSE;
}

pub mod event_kind {
    pub const EMIT: u32 = 1;
    pub const COMPLETE: u32 = 2;
    pub const FAIL: u32 = 3;
}

pub mod response_outcome {
    pub const RESOLVE: u32 = 1;
    pub const REJECT: u32 = 2;
}

pub type AsyncHostEnqueue = unsafe extern "C" fn(u64, u64, u32, u64, *const u8, usize) -> i32;
pub type AsyncTaskCancel = unsafe extern "C" fn(u64, u64, *const u8, usize) -> i32;
pub type AsyncResponseResolve = unsafe extern "C" fn(u64, u64, u32, *const u8, usize) -> i32;
pub type AsyncHostConfigure =
    unsafe extern "C" fn(u64, u64, u32, u32, u64, Option<AsyncTaskCancel>) -> i32;
pub type AsyncHostOpenResponse =
    unsafe extern "C" fn(u64, u64, u64, u64, Option<AsyncResponseResolve>, *mut u64) -> i32;
pub type BlockingHostInvoke =
    unsafe extern "C" fn(u64, u64, *const u8, usize, *mut CalcitFfiBuffer) -> i32;
pub type BlockingHostFinish = unsafe extern "C" fn(u64, u64) -> i32;
pub type BlockingHostFreeBuffer = unsafe extern "C" fn(u64, u64, CalcitFfiBuffer) -> i32;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalcitFfiBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub cap: usize,
}

impl CalcitFfiBuffer {
    pub const fn empty() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
            cap: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalcitFfiAsyncTaskV1 {
    pub protocol_version: u32,
    pub struct_size: u32,
    pub handle: u64,
    pub kind: u32,
    pub flags: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalcitFfiAsyncHostV1 {
    pub protocol_version: u32,
    pub struct_size: u32,
    pub context: u64,
    pub enqueue: Option<AsyncHostEnqueue>,
    pub configure_task: Option<AsyncHostConfigure>,
    pub open_response: Option<AsyncHostOpenResponse>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CalcitFfiBlockingHostV1 {
    pub protocol_version: u32,
    pub struct_size: u32,
    pub context: u64,
    pub invoke: Option<BlockingHostInvoke>,
    pub finish: Option<BlockingHostFinish>,
    pub free_buffer: Option<BlockingHostFreeBuffer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriptorError {
    Null,
    Version { expected: u32, actual: u32 },
    TooSmall { expected: u32, actual: u32 },
}

impl fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "descriptor pointer is null"),
            Self::Version { expected, actual } => write!(
                f,
                "descriptor protocol version {actual} does not match {expected}"
            ),
            Self::TooSmall { expected, actual } => {
                write!(f, "descriptor size {actual} is smaller than {expected}")
            }
        }
    }
}

impl std::error::Error for DescriptorError {}

unsafe fn read_abi_header<T>(value: *const T) -> Result<(u32, u32), DescriptorError> {
    if value.is_null() {
        return Err(DescriptorError::Null);
    }
    let bytes = value.cast::<u8>();
    // SAFETY: every versioned descriptor starts with two readable u32 fields.
    let version = unsafe { ptr::read_unaligned(bytes.cast::<u32>()) };
    // SAFETY: the second field begins directly after the first u32.
    let struct_size = unsafe { ptr::read_unaligned(bytes.add(size_of::<u32>()).cast::<u32>()) };
    Ok((version, struct_size))
}

unsafe fn copy_versioned<T: Copy>(value: *const T, version: u32) -> Result<T, DescriptorError> {
    // SAFETY: the caller provides a pointer to a descriptor beginning with the v1 header.
    let (actual_version, actual_size) = unsafe { read_abi_header(value) }?;
    if actual_version != version {
        return Err(DescriptorError::Version {
            expected: version,
            actual: actual_version,
        });
    }
    let expected_size = size_of::<T>() as u32;
    if actual_size < expected_size {
        return Err(DescriptorError::TooSmall {
            expected: expected_size,
            actual: actual_size,
        });
    }
    // SAFETY: the validated descriptor contains all v1 fields; unaligned reads
    // support descriptors produced by foreign C callers.
    Ok(unsafe { ptr::read_unaligned(value) })
}

/// Copy and validate an asynchronous task descriptor supplied over C ABI.
///
/// # Safety
///
/// `value` must be null or point to memory readable for at least the two-field
/// ABI header. If its declared size covers v1, the full v1 structure must be
/// readable for the duration of this call.
pub unsafe fn copy_task_descriptor(
    value: *const CalcitFfiAsyncTaskV1,
) -> Result<CalcitFfiAsyncTaskV1, DescriptorError> {
    // SAFETY: forwarded from the versioned descriptor contract.
    unsafe { copy_versioned(value, ASYNC_PROTOCOL_VERSION) }
}

/// Copy and validate an asynchronous host descriptor supplied over C ABI.
///
/// # Safety
///
/// `value` must follow the same readable versioned-descriptor contract as
/// [`copy_task_descriptor`]. Function pointers must remain valid while tasks
/// created from the copied descriptor are active.
pub unsafe fn copy_async_host(
    value: *const CalcitFfiAsyncHostV1,
) -> Result<CalcitFfiAsyncHostV1, DescriptorError> {
    // SAFETY: forwarded from the versioned descriptor contract.
    unsafe { copy_versioned(value, ASYNC_PROTOCOL_VERSION) }
}

/// Copy and validate a blocking host descriptor supplied over C ABI.
///
/// # Safety
///
/// `value` must follow the same readable versioned-descriptor contract as
/// [`copy_task_descriptor`]. Function pointers must remain valid for the
/// blocking call that consumes the copied descriptor.
pub unsafe fn copy_blocking_host(
    value: *const CalcitFfiBlockingHostV1,
) -> Result<CalcitFfiBlockingHostV1, DescriptorError> {
    // SAFETY: forwarded from the versioned descriptor contract.
    unsafe { copy_versioned(value, ASYNC_PROTOCOL_VERSION) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_layouts_remain_stable_on_64_bit_targets() {
        if size_of::<usize>() == 8 {
            assert_eq!(size_of::<CalcitFfiBuffer>(), 24);
            assert_eq!(size_of::<CalcitFfiAsyncTaskV1>(), 24);
            assert_eq!(size_of::<CalcitFfiAsyncHostV1>(), 40);
            assert_eq!(size_of::<CalcitFfiBlockingHostV1>(), 40);
        }
    }

    #[test]
    fn descriptor_copy_rejects_version_and_size_mismatches() {
        let wrong_version = CalcitFfiAsyncTaskV1 {
            protocol_version: 2,
            struct_size: size_of::<CalcitFfiAsyncTaskV1>() as u32,
            handle: 1,
            kind: task_kind::ONE_SHOT,
            flags: 0,
        };
        let error = unsafe { copy_task_descriptor(&wrong_version) }.expect_err("version mismatch");
        assert!(matches!(error, DescriptorError::Version { .. }));

        let too_small = CalcitFfiAsyncTaskV1 {
            protocol_version: ASYNC_PROTOCOL_VERSION,
            struct_size: 8,
            ..wrong_version
        };
        let error = unsafe { copy_task_descriptor(&too_small) }.expect_err("size mismatch");
        assert!(matches!(error, DescriptorError::TooSmall { .. }));
    }
}
