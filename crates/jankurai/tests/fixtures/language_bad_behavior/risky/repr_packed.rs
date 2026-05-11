#[repr(packed)]
pub struct Packet {
    pub tag: u8,
    pub value: u32,
}
