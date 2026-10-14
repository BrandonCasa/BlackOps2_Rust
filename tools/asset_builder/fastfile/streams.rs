use super::*;

pub struct Streams {
    pub serialized: Vec<u8>,
    pub serialized_offset: usize,
    pub memory: [Vec<u8>; 8],
    pub capacity: [usize; 8],
    pub cursor: [usize; 8],
    pub active: usize,
    pub deferred: Vec<(Address, usize)>,
    pub script_strings: Vec<Option<Address>>,
    pub dependencies: Vec<String>,
    stack: Vec<(usize, usize)>,
}

impl Streams {
    pub fn new(file: Fastfile) -> Self {
        Self {
            serialized: file.bytes,
            serialized_offset: 0,
            memory: std::array::from_fn(|_| Vec::new()),
            capacity: file.block_sizes,
            cursor: [0; 8],
            active: 0,
            deferred: Vec::new(),
            script_strings: Vec::new(),
            dependencies: Vec::new(),
            stack: Vec::new(),
        }
    }

    pub fn push(&mut self, block: usize) -> Result<()> {
        if block >= 8 {
            return Err("invalid stream block".into());
        }
        self.stack.push((self.active, self.cursor[block]));
        self.active = block;
        Ok(())
    }

    pub fn pop(&mut self) -> Result<()> {
        let (previous, entry) = self.stack.pop().ok_or("empty stream stack")?;
        if self.active == 0 {
            self.cursor[0] = entry;
        }
        self.active = previous;
        Ok(())
    }

    pub fn alloc(&mut self, mask: usize) -> Result<Address> {
        if mask & (mask + 1) != 0 {
            return Err("invalid native alignment mask".into());
        }
        let offset = self.cursor[self.active]
            .checked_add(mask)
            .ok_or("stream alignment overflow")?
            & !mask;
        if offset > self.capacity[self.active] {
            return Err("stream allocation exceeds declared capacity".into());
        }
        self.cursor[self.active] = offset;
        Ok(Address {
            block: self.active,
            offset,
        })
    }

    pub fn reserve(&mut self, size: usize) -> Result<()> {
        let next = self.cursor[self.active]
            .checked_add(size)
            .ok_or("stream cursor overflow")?;
        if next > self.capacity[self.active] {
            return Err("stream exceeds declared capacity".into());
        }
        self.cursor[self.active] = next;
        Ok(())
    }

    fn ensure(&mut self, at: Address, size: usize) -> Result<std::ops::Range<usize>> {
        let end = at.offset.checked_add(size).ok_or("memory range overflow")?;
        if at.block >= 8 {
            return Err("Invalid native memory block".into());
        }
        if end > self.capacity[at.block] {
            return Err(format!(
                "Native block {} range {}..{end} exceeds capacity {}",
                at.block, at.offset, self.capacity[at.block]
            ));
        }
        if self.memory[at.block].len() < end {
            self.memory[at.block].resize(end, 0);
        }
        Ok(at.offset..end)
    }

    pub fn load(&mut self, start: bool, at: Address, size: usize) -> Result<bool> {
        if !start || size == 0 {
            return Ok(true);
        }
        if self.active == 7 {
            return Ok(false);
        }
        let range = self.ensure(at, size)?;
        let read = matches!(self.active, 0 | 5 | 6);
        if read {
            let end = self
                .serialized_offset
                .checked_add(size)
                .ok_or("serialized cursor overflow")?;
            let bytes = self
                .serialized
                .get(self.serialized_offset..end)
                .ok_or_else(|| {
                    format!(
                        "truncated serialized stream at{} size{size}",
                        self.serialized_offset
                    )
                })?;
            self.memory[at.block][range].copy_from_slice(bytes);
            self.serialized_offset = end;
        } else {
            self.memory[at.block][range].fill(0);
            if matches!(self.active, 3 | 4) {
                self.deferred.push((at, size));
            }
        }
        self.reserve(size)?;
        Ok(read)
    }

    pub fn read(&self, at: Address, size: usize) -> Result<&[u8]> {
        self.memory
            .get(at.block)
            .and_then(|b| b.get(at.offset..at.offset.checked_add(size)?))
            .ok_or_else(|| format!("unloaded memory {:?} size{size}", at))
    }

    pub fn text(&self, at: Address) -> Result<String> {
        let bytes = self
            .memory
            .get(at.block)
            .and_then(|block| block.get(at.offset..))
            .ok_or_else(|| format!("String points outside loaded native memory at {at:?}"))?;
        let length = bytes
            .iter()
            .position(|byte| *byte == 0)
            .ok_or("Unterminated native string")?;
        String::from_utf8(bytes[..length].to_vec()).map_err(|error| error.to_string())
    }

    pub fn u8(&self, at: Address) -> Result<u8> {
        Ok(self.read(at, 1)?[0])
    }
    pub fn u16(&self, at: Address) -> Result<u16> {
        Ok(u16::from_le_bytes(self.read(at, 2)?.try_into().unwrap()))
    }
    pub fn u32(&self, at: Address) -> Result<u32> {
        Ok(u32::from_le_bytes(self.read(at, 4)?.try_into().unwrap()))
    }
    pub fn set_u32(&mut self, at: Address, value: u32) -> Result<()> {
        let range = self.ensure(at, 4)?;
        self.memory[at.block][range].copy_from_slice(&value.to_le_bytes());
        Ok(())
    }

    pub fn alias(&self, packed: u32) -> Result<Address> {
        Address::decode(self.u32(Address::decode(packed)?)?)
    }

    pub fn insert_slot(&mut self) -> Result<Address> {
        self.push(5)?;
        let at = self.alloc(3)?;
        self.ensure(at, 4)?;
        self.reserve(4)?;
        self.pop()?;
        Ok(at)
    }

    pub fn cstring(&mut self, field: Address) -> Result<Option<Address>> {
        match self.u32(field)? {
            0 => Ok(None),
            u32::MAX => {
                let at = self.alloc(0)?;
                let length = self.serialized[self.serialized_offset..]
                    .iter()
                    .position(|b| *b == 0)
                    .ok_or("unterminated source string")?
                    + 1;
                self.load(true, at, length)?;
                self.set_u32(field, at.encode()?)?;
                Ok(Some(at))
            }
            packed => Ok(Some(Address::decode(packed)?)),
        }
    }

    pub fn asset_list(&mut self) -> Result<Vec<(u32, Address)>> {
        self.push(5)?;
        let end = self
            .serialized_offset
            .checked_add(24)
            .ok_or("Asset-list size overflow")?;
        let root: [u8; 24] = self
            .serialized
            .get(self.serialized_offset..end)
            .ok_or("Truncated native asset-list header")?
            .try_into()
            .unwrap();
        self.serialized_offset = end;
        let field = |offset| u32::from_le_bytes(root[offset..offset + 4].try_into().unwrap());
        self.push(5)?;
        let count = field(0) as usize;
        if field(4) != 0 {
            let strings = self.alloc(3)?;
            self.load(
                true,
                strings,
                count.checked_mul(4).ok_or("string count overflow")?,
            )?;
            for i in 0..count {
                let address = self.cstring(strings.add(i * 4)?)?;
                self.script_strings.push(address);
            }
        }
        self.pop()?;
        let dependency_count = field(8) as usize;
        if field(12) != 0 {
            let strings = self.alloc(3)?;
            self.load(
                true,
                strings,
                dependency_count
                    .checked_mul(4)
                    .ok_or("dependency count overflow")?,
            )?;
            for i in 0..dependency_count {
                if let Some(address) = self.cstring(strings.add(i * 4)?)? {
                    self.dependencies.push(self.text(address)?);
                }
            }
        }
        let count = field(16) as usize;
        let mut assets = Vec::new();
        if field(20) != 0 {
            let entries = self.alloc(3)?;
            self.load(
                true,
                entries,
                count.checked_mul(8).ok_or("asset count overflow")?,
            )?;
            for i in 0..count {
                let at = entries.add(i * 8)?;
                assets.push((self.u32(at)?, at.add(4)?));
            }
        }
        Ok(assets)
    }
}
