use calcit_native_ffi::{ASYNC_PROTOCOL_VERSION, BUFFER_PROTOCOL_VERSION};
#[cfg(feature = "edn")]
use calcit_native_ffi::{CalcitFfiBuffer, buffer_status, copy_buffer, encode_edn, free_buffer};
#[cfg(feature = "edn")]
use cirru_edn::{Edn, EdnListView};

calcit_native_ffi::export_buffer_abi_v1!();
calcit_native_ffi::export_async_abi_v1!();

#[cfg(feature = "edn")]
fn echo(args: Vec<Edn>) -> Result<Edn, String> {
    if args.is_empty() {
        Err("echo expects at least one argument".to_owned())
    } else {
        Ok(Edn::List(EdnListView(args)))
    }
}

#[cfg(feature = "edn")]
calcit_native_ffi::export_edn_buffer_method_v1!(echo_calcit_ffi_v1, echo);

#[test]
fn exported_protocol_symbols_report_v1() {
    assert_eq!(calcit_ffi_buffer_version(), BUFFER_PROTOCOL_VERSION);
    assert_eq!(calcit_ffi_async_version(), ASYNC_PROTOCOL_VERSION);
}

#[test]
#[cfg(feature = "edn")]
fn exported_edn_method_uses_shared_buffer_adapter() {
    let request =
        encode_edn(&Edn::List(EdnListView(vec![Edn::Number(1.0)]))).expect("encode request");
    let mut output = CalcitFfiBuffer::empty();
    let status = unsafe { echo_calcit_ffi_v1(request.as_ptr(), request.len(), &mut output) };
    assert_eq!(status, buffer_status::OK);
    let response = unsafe { copy_buffer(output) }.expect("copy response");
    unsafe { free_buffer(output) };
    assert_eq!(
        cirru_edn::parse(std::str::from_utf8(&response).expect("UTF-8 response"))
            .expect("parse response"),
        Edn::List(EdnListView(vec![Edn::Number(1.0)]))
    );

    let empty = encode_edn(&Edn::List(EdnListView(vec![]))).expect("encode empty request");
    let mut error_output = CalcitFfiBuffer::empty();
    let error_status =
        unsafe { echo_calcit_ffi_v1(empty.as_ptr(), empty.len(), &mut error_output) };
    assert_eq!(error_status, buffer_status::ERROR);
    let error = unsafe { copy_buffer(error_output) }.expect("copy error");
    unsafe { free_buffer(error_output) };
    assert_eq!(
        String::from_utf8(error).expect("UTF-8 error"),
        "echo expects at least one argument"
    );
}
