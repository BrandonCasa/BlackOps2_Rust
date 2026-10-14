pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

pub fn parallel_chunks_mut<T: Send>(
    values: &mut [T],
    minimum: usize,
    work: impl Fn(usize, &mut [T]) -> Result<()> + Sync,
) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(values.len().div_ceil(minimum));
    let size = values.len().div_ceil(workers).next_multiple_of(minimum);
    if workers == 1 {
        return work(0, values);
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = values
            .chunks_mut(size)
            .enumerate()
            .map(|(index, chunk)| {
                let work = &work;
                scope.spawn(move || work(index * size, chunk))
            })
            .collect();
        for handle in handles {
            handle
                .join()
                .map_err(|_| "Asset calculation worker panicked")??;
        }
        Ok(())
    })
}

pub fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
pub fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}
pub fn u64_at(bytes: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(bytes[offset..offset + 8].try_into().unwrap())
}
pub fn push16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend(value.to_le_bytes());
}
pub fn push32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend(value.to_le_bytes());
}
pub fn push64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend(value.to_le_bytes());
}
