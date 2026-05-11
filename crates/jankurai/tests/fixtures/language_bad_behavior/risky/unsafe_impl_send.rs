struct NotSend(*mut u8);

unsafe impl Send for NotSend {}
