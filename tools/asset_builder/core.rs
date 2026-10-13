pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
pub fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
pub fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
pub fn push16(bytes: &mut Vec<u8>, value: u16) { bytes.extend(value.to_le_bytes()); }
pub fn push32(bytes: &mut Vec<u8>, value: u32) { bytes.extend(value.to_le_bytes()); }
pub fn push64(bytes: &mut Vec<u8>, value: u64) { bytes.extend(value.to_le_bytes()); }
