use std::io::{self, Read, Seek, SeekFrom};

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn word(bytes: &[u8], offset: usize) -> io::Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or_else(|| invalid("Truncated IPAK word"))?
            .try_into()
            .unwrap(),
    ))
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub data_key: u32,
    pub name_key: u32,
    offset: u64,
    size: usize,
}

pub struct Ipak<R> {
    input: R,
    pub entries: Vec<Entry>,
}

impl<R: Read + Seek> Ipak<R> {
    pub fn read(mut input: R) -> io::Result<Self> {
        let size = input.seek(SeekFrom::End(0))?;
        input.seek(SeekFrom::Start(0))?;
        let mut header = [0; 2048];
        input.read_exact(&mut header)?;
        if word(&header, 0)? != 0x4950414b
            || word(&header, 4)? & 0xffff0000 != 0x50000
            || u64::from(word(&header, 8)?) != size
        {
            return Err(invalid("Invalid PC IPAK header"));
        }
        let sections = word(&header, 12)? as usize;
        if sections > (header.len() - 16) / 16 {
            return Err(invalid("IPAK section count exceeds header"));
        }
        let mut index = None;
        let mut data = None;
        for i in 0..sections {
            let p = 16 + i * 16;
            let start = u64::from(word(&header, p + 4)?);
            let length = u64::from(word(&header, p + 8)?);
            let count = word(&header, p + 12)? as usize;
            if start + length > size {
                return Err(invalid("IPAK section exceeds input"));
            }
            match word(&header, p)? {
                1 if index.is_none() => index = Some((start, length, count)),
                2 if data.is_none() => data = Some((start, length)),
                1 | 2 => return Err(invalid("Duplicate IPAK section")),
                _ => (),
            }
        }
        let (start, length, count) = index.ok_or_else(|| invalid("IPAK index is absent"))?;
        let (data_start, data_length) = data.ok_or_else(|| invalid("IPAK data is absent"))?;
        if count as u64 * 16 != length {
            return Err(invalid("IPAK index length disagrees"));
        }
        input.seek(SeekFrom::Start(start))?;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let mut row = [0; 16];
            input.read_exact(&mut row)?;
            let offset = u64::from(word(&row, 8)?);
            let size = (word(&row, 12)? & 0x0fffffff) as usize;
            if offset + size as u64 > data_length {
                return Err(invalid("IPAK entry exceeds data section"));
            }
            entries.push(Entry {
                data_key: word(&row, 0)?,
                name_key: word(&row, 4)?,
                offset: data_start + offset,
                size,
            });
        }
        Ok(Self { input, entries })
    }

    pub fn image(&mut self, entry: Entry) -> io::Result<Vec<u8>> {
        let mut expected: usize = 256 * 1024 * 1024;
        self.input.seek(SeekFrom::Start(entry.offset))?;
        let mut bytes = vec![0; entry.size];
        self.input.read_exact(&mut bytes)?;
        let mut output = Vec::new();
        let mut offset = 0;
        while offset < bytes.len() {
            let header = bytes
                .get(offset..offset + 128)
                .ok_or_else(|| invalid("Truncated IPAK block header"))?;
            let first = word(header, 0)?;
            let count = (first >> 24) as usize;
            let mut destination = (first & 0xffffff) as usize;
            if count > 31 {
                return Err(invalid("IPAK command count exceeds block header"));
            }
            let mut cursor = offset + 128;
            for i in 0..count {
                let command = word(header, 4 + i * 4)?;
                let size = (command & 0xffffff) as usize;
                let payload = bytes
                    .get(cursor..cursor + size)
                    .ok_or_else(|| invalid("Truncated IPAK command"))?;
                cursor += size;
                let decoded = match command >> 24 {
                    0 => payload.to_vec(),
                    1 => lzo(payload, expected.saturating_sub(destination))?,
                    _ => continue,
                };
                if destination != output.len() || output.len() + decoded.len() > expected {
                    return Err(invalid(
                        "IPAK block destination or decoded length disagrees",
                    ));
                }
                destination += decoded.len();
                output.extend(decoded);
                if output.len() >= 64 {
                    if output.get(..4) != Some(b"IWi\x1b") {
                        return Err(invalid("IPAK image is not T6 IWI v27"));
                    }
                    expected = word(&output, 32)? as usize;
                    if expected < output.len() || expected > 256 * 1024 * 1024 {
                        return Err(invalid("Invalid IPAK IWI length"));
                    }
                }
            }
            offset = (cursor + 127) & !127;
        }
        if output.len() != expected {
            return Err(invalid("IPAK image length disagrees"));
        }
        Ok(output)
    }
}

fn lzo(data: &[u8], limit: usize) -> io::Result<Vec<u8>> {
    struct Decoder<'a> {
        data: &'a [u8],
        cursor: usize,
        output: Vec<u8>,
        limit: usize,
    }
    impl Decoder<'_> {
        fn byte(&mut self) -> io::Result<usize> {
            let value = *self
                .data
                .get(self.cursor)
                .ok_or_else(|| invalid("LZO input exhausted"))?;
            self.cursor += 1;
            Ok(usize::from(value))
        }
        fn literal(&mut self, size: usize) -> io::Result<()> {
            let data = self
                .data
                .get(self.cursor..self.cursor + size)
                .ok_or_else(|| invalid("Truncated LZO literal"))?;
            if self.output.len() + size > self.limit {
                return Err(invalid("LZO literal exceeds destination"));
            }
            self.output.extend(data);
            self.cursor += size;
            Ok(())
        }
        fn extended(&mut self, mut size: usize) -> io::Result<usize> {
            loop {
                let value = self.byte()?;
                if value != 0 {
                    return Ok(size + value);
                }
                size += 255;
                if size > self.limit {
                    return Err(invalid("LZO length exceeds destination"));
                }
            }
        }
    }
    let mut d = Decoder {
        data,
        cursor: 0,
        output: Vec::new(),
        limit,
    };
    let mut state = 1;
    if data.first().is_some_and(|value| *value > 17) {
        let n = d.byte()? - 17;
        d.literal(n)?;
        state = if n < 4 { 0 } else { 2 };
    }
    loop {
        let t = d.byte()?;
        if t < 16 && state == 1 {
            let n = if t == 0 { d.extended(15)? } else { t };
            d.literal(n + 3)?;
            state = 2;
            continue;
        }
        let (distance, length, trailing) = if t < 16 {
            (
                1 + (t >> 2) + (d.byte()? << 2) + if state == 2 { 2048 } else { 0 },
                if state == 2 { 3 } else { 2 },
                t & 3,
            )
        } else if t >= 64 {
            (1 + ((t >> 2) & 7) + (d.byte()? << 3), (t >> 5) + 1, t & 3)
        } else {
            let mask = if t >= 32 { 31 } else { 7 };
            let n = if t & mask == 0 {
                d.extended(mask)?
            } else {
                t & mask
            };
            let lo = d.byte()?;
            let hi = d.byte()?;
            let mut distance = (lo | hi << 8) >> 2;
            if t >= 32 {
                distance += 1;
            } else {
                distance += (t & 8) << 11;
                if distance == 0 {
                    if d.cursor != data.len() {
                        return Err(invalid("Trailing LZO input"));
                    }
                    return Ok(d.output);
                }
                distance += 16384;
            }
            (distance, n + 2, lo & 3)
        };
        if distance > d.output.len() || d.output.len() + length > limit {
            return Err(invalid("LZO match exceeds source or destination"));
        }
        for _ in 0..length {
            d.output.push(d.output[d.output.len() - distance]);
        }
        d.literal(trailing)?;
        state = if trailing != 0 { 0 } else { 1 };
    }
}
