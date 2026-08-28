//! Stable C ABI building blocks for Calcit native modules.
//!
//! This crate deliberately contains only transport and adapter code. Native
//! modules keep ownership of their domain logic, cancellation state, threads,
//! and registries.

#![forbid(unsafe_op_in_unsafe_fn)]

mod abi;
mod async_ffi;
mod buffer;

#[cfg(feature = "edn")]
mod blocking;
#[cfg(feature = "edn")]
mod edn;

pub use abi::*;
pub use async_ffi::*;
pub use buffer::*;

#[cfg(feature = "edn")]
pub use blocking::*;
#[cfg(feature = "edn")]
pub use edn::*;

/// Export the buffer protocol symbols from the final `cdylib`.
///
/// Invoke this macro exactly once in a native module crate. Defining the
/// symbols at the final link boundary ensures that the matching allocator is
/// used to release every returned buffer.
#[macro_export]
macro_rules! export_buffer_abi_v1 {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn calcit_ffi_buffer_version() -> u32 {
            $crate::BUFFER_PROTOCOL_VERSION
        }

        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn calcit_ffi_buffer_free(buffer: $crate::CalcitFfiBuffer) {
            // SAFETY: Calcit returns the exact metadata previously emitted by this
            // final dynamic library.
            unsafe { $crate::free_buffer(buffer) }
        }
    };
}

/// Export the async/blocking protocol version symbol from the final `cdylib`.
#[macro_export]
macro_rules! export_async_abi_v1 {
    () => {
        #[unsafe(no_mangle)]
        pub extern "C" fn calcit_ffi_async_version() -> u32 {
            $crate::ASYNC_PROTOCOL_VERSION
        }
    };
}

/// Export one synchronous Cirru EDN method through buffer protocol v1.
///
/// Available with the default `edn` feature. The handler accepts owned EDN
/// arguments and returns an EDN value or message.
/// The generated function keeps the public symbol explicit while delegating
/// decoding, panic isolation, response encoding, and output ownership.
#[macro_export]
macro_rules! export_edn_buffer_method_v1 {
    ($export:ident, $method:path) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $export(
            request_ptr: *const u8,
            request_len: usize,
            output: *mut $crate::CalcitFfiBuffer,
        ) -> i32 {
            // SAFETY: this forwards the documented buffer protocol v1 contract.
            unsafe { $crate::run_buffer_adapter(request_ptr, request_len, output, $method) }
        }
    };
}
