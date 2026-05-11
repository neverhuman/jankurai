use std::ffi::{CStr, CString};
use std::mem::{self, MaybeUninit};

pub fn demo(bytes: &[u8], ptr: *const u8) -> usize {
    let _ = unsafe { mem::transmute::<u8, u16>(bytes[0]) };
    let mut slot = MaybeUninit::<u8>::uninit();
    let _ = unsafe { slot.assume_init() };
    let _ = unsafe { mem::zeroed::<usize>() };
    let _ = unsafe { bytes.get_unchecked(0) };
    let _ = unsafe { bytes.first().unwrap_unchecked() };
    let _ = unsafe { std::hint::unreachable_unchecked() };
    let _ = unsafe { std::str::from_utf8_unchecked(bytes) };
    let _ = unsafe { std::slice::from_raw_parts(ptr, 1) };
    let _ = unsafe { Box::from_raw(ptr as *mut u8) };
    let _ = unsafe { Vec::from_raw_parts(ptr as *mut u8, 1, 1) };
    let _ = unsafe { CString::from_raw(ptr as *mut i8) };
    let _ = unsafe { CStr::from_ptr(ptr as *const i8) };
    1
}
