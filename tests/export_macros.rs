use calcit_native_ffi::{ASYNC_PROTOCOL_VERSION, BUFFER_PROTOCOL_VERSION};

calcit_native_ffi::export_buffer_abi_v1!();
calcit_native_ffi::export_async_abi_v1!();

#[test]
fn exported_protocol_symbols_report_v1() {
    assert_eq!(calcit_ffi_buffer_version(), BUFFER_PROTOCOL_VERSION);
    assert_eq!(calcit_ffi_async_version(), ASYNC_PROTOCOL_VERSION);
}
