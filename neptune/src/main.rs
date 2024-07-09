use libc::{c_char, c_int};
/// a test demo for pipy
use std::ffi::CString;

fn main() {
    let args: Vec<CString> = std::env::args()
        .map(|arg| CString::new(arg).unwrap())
        .collect();

    let c_args: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr()).collect();

    unsafe {
        neptune::pipy_main(c_args.len() as c_int, c_args.as_ptr());
    }
}
