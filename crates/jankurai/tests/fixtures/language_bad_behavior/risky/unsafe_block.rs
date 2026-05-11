pub fn read(ptr: *const u8) -> u8 {
    unsafe { *ptr }
}
