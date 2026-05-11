pub fn read(ptr: *const u8) -> u8 {
    // SAFETY: caller guarantees ptr points to one readable byte.
    unsafe { *ptr }
}
