use flate2::{Decompress, FlushDecompress, Status};

mod crypto;
mod streams;
pub use streams::Streams;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Address {
    pub block: usize,
    pub offset: usize,
}

impl Address {
    pub fn decode(value: u32) -> Result<Self> {
        let value = value.checked_sub(1).ok_or("null packed pointer")?;
        Ok(Self {
            block: (value >> 29) as usize,
            offset: (value & 0x1fff_ffff) as usize,
        })
    }

    pub fn encode(self) -> Result<u32> {
        if self.block >= 8 || self.offset >= 0x1fff_ffff {
            return Err("packed pointer exceeds native range".into());
        }
        Ok((((self.block as u32) << 29) | self.offset as u32).wrapping_add(1))
    }

    pub fn add(self, size: usize) -> Result<Self> {
        Ok(Self {
            offset: self.offset.checked_add(size).ok_or("address overflow")?,
            ..self
        })
    }
}

pub struct Fastfile {
    pub external_size: u32,
    pub block_sizes: [usize; 8],
    pub bytes: Vec<u8>,
    pub compressed_blocks: usize,
}

pub fn decode(input: &[u8], zone: &str) -> Result<Fastfile> {
    if input.get(..8) != Some(b"TAff0100") || input.get(8..12) != Some(&147_u32.to_le_bytes()) {
        return Err("expected installed PC TAff0100 version147 fastfile".into());
    }
    if input.get(12..20) != Some(b"PHEEBs71") || input.len() < 312 {
        return Err("missing native fastfile authentication header".into());
    }
    let embedded_name = input[24..56].split(|b| *b == 0).next().unwrap_or_default();
    if embedded_name != zone.as_bytes() {
        return Err(format!(
            "fastfile embeds a different zone name: {}",
            String::from_utf8_lossy(embedded_name)
        ));
    }
    let mut cipher = crypto::FastfileCipher::new(zone).map_err(str::to_owned)?;
    let mut output = Vec::new();
    let mut position = 312;
    let mut compressed_blocks = 0;
    let mut expected = None;
    while position + 4 <= input.len() {
        let size = u32::from_le_bytes(input[position..position + 4].try_into().unwrap()) as usize;
        position += 4;
        if size == 0 {
            if input[position..].iter().any(|b| *b != 0) {
                return Err("nonzero bytes after fastfile terminator".into());
            }
            break;
        }
        let end = position
            .checked_add(size)
            .ok_or("compressed length overflow")?;
        let mut block = input
            .get(position..end)
            .ok_or("truncated compressed fastfile block")?
            .to_vec();
        position = end;
        cipher.decrypt(&mut block);
        let mut inflater = Decompress::new(false);
        let mut inflated = Vec::with_capacity(0x8000);
        let status = inflater
            .decompress_vec(&block, &mut inflated, FlushDecompress::Finish)
            .map_err(|e| format!("fastfile block{compressed_blocks} raw DEFLATE failed: {e}"))?;
        if status != Status::StreamEnd
            || inflater.total_in() as usize != block.len()
            || inflated.len() > 0x8000
        {
            return Err(format!(
                "invalid complete native DEFLATE block{compressed_blocks}"
            ));
        }
        if compressed_blocks == 0 {
            if inflated.len() != 40 {
                return Err("first fastfile block must contain40-byte XFile header".into());
            }
            expected = Some(u32::from_le_bytes(inflated[..4].try_into().unwrap()) as usize + 40);
            output
                .try_reserve(expected.unwrap())
                .map_err(|e| e.to_string())?;
        }
        if output
            .len()
            .checked_add(inflated.len())
            .ok_or("decoded size overflow")?
            > expected.unwrap_or(0)
        {
            return Err("decoded fastfile exceeds declared XFile size".into());
        }
        output.extend_from_slice(&inflated);
        compressed_blocks += 1;
    }
    if Some(output.len()) != expected
        || position != input.len() && input[position..].iter().any(|b| *b != 0)
    {
        return Err("decoded fastfile length differs from native XFile header".into());
    }
    let external_size = u32::from_le_bytes(output[4..8].try_into().unwrap());
    let block_sizes = std::array::from_fn(|i| {
        u32::from_le_bytes(output[8 + i * 4..12 + i * 4].try_into().unwrap()) as usize
    });
    Ok(Fastfile {
        external_size,
        block_sizes,
        bytes: output[40..].to_vec(),
        compressed_blocks,
    })
}
