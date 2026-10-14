use super::fastfile::{Address, Fastfile, Result, Streams};
use std::collections::BTreeMap;

pub struct Asset {
    pub kind: u32,
    pub name: String,
    pub root: Address,
    pub reference: bool,
}

pub struct Zone {
    pub streams: Streams,
    pub assets: Vec<Asset>,
    names: BTreeMap<(u32, String), Address>,
}

mod audio_fx;
mod collision;
mod gameplay;
mod material;
mod model;
mod ui;
mod world;

impl Zone {
    pub fn load(file: Fastfile) -> Result<Self> {
        if file.block_sizes[7] != 0 {
            return Err(
                "Native block 7 is occupied; asset retention must preserve that block".into(),
            );
        }
        let mut streams = Streams::new(file);
        streams.capacity[7] = 0x1fff_fffe;
        let entries = streams.asset_list()?;
        let mut zone = Self {
            streams,
            assets: Vec::new(),
            names: BTreeMap::new(),
        };
        for (index, (kind, field)) in entries.into_iter().enumerate() {
            zone.asset(kind, field).map_err(|error| {
                format!(
                    "Native asset {index}, type {kind}, serialized byte {}, after {}: {error}",
                    zone.streams.serialized_offset,
                    zone.assets
                        .last()
                        .map(|asset| format!("type {} {}", asset.kind, asset.name))
                        .unwrap_or_else(|| "zone header".into())
                )
            })?;
        }
        if zone.streams.serialized_offset != zone.streams.serialized.len() {
            return Err(format!(
                "Native traversal consumed {} of {} serialized bytes",
                zone.streams.serialized_offset,
                zone.streams.serialized.len()
            ));
        }
        zone.streams.pop()?;
        Ok(zone)
    }

    fn string(&self, pointer: u32) -> Result<String> {
        self.streams.text(Address::decode(pointer)?)
    }

    fn retain(&mut self, root: Address, size: usize) -> Result<Address> {
        let bytes = self.streams.read(root, size)?.to_vec();
        let offset = self.streams.memory[7].len();
        let address = Address { block: 7, offset };
        address.add(size)?.encode()?;
        self.streams.memory[7].extend_from_slice(&bytes);
        Ok(address)
    }

    fn data(
        &mut self,
        field: Address,
        mask: usize,
        size: usize,
        relocate: bool,
    ) -> Result<Option<Address>> {
        let pointer = self.streams.u32(field)?;
        if pointer == 0 {
            return Ok(None);
        }
        if relocate && pointer != u32::MAX {
            Address::decode(pointer)?;
            return Ok(None);
        }
        let address = self.streams.alloc(mask)?;
        self.streams.set_u32(field, address.encode()?)?;
        self.streams.load(true, address, size)?;
        Ok(Some(address))
    }

    fn asset(&mut self, kind: u32, field: Address) -> Result<()> {
        let pointer = self.streams.u32(field)?;
        if pointer == 0 {
            return Ok(());
        }
        if pointer != u32::MAX && pointer != u32::MAX - 1 {
            let address = self.streams.alias(pointer).map_err(|error| {
                format!("Type {kind} alias {pointer:#010x} at {field:?}: {error}")
            })?;
            self.streams.set_u32(field, address.encode()?)?;
            return Ok(());
        }
        let size = match kind {
            1 => 84,
            2 => 2696,
            3 => 24,
            4 => 104,
            5 => 248,
            6 => 112,
            7 => 152,
            8 => 80,
            9 => 0x1294,
            10 => 12,
            11 | 12 => 332,
            13 => 16,
            14 | 15 => 44,
            16 => 36,
            17 => 1028,
            18 => 16,
            20 => 36,
            21 => 20,
            22 => 12,
            23 => 400,
            24 => 8,
            25 => 716,
            29 => 284,
            30 => 424,
            31 => 28,
            32 => 68,
            33 => 76,
            34 => 8,
            41 => 12,
            42 => 20,
            43 => 36,
            45 => 8,
            48 | 49 => 12,
            50 => 2604,
            53 => 128,
            54 => 8,
            57 => 900,
            58 => 132,
            59 => 560,
            _ => {
                return Err(format!(
                    "The recovered loader for native asset type {kind} has not been implemented"
                ));
            }
        };
        self.streams.push(0)?;
        let root = self.streams.alloc(if kind == 23 { 7 } else { 3 })?;
        self.streams.set_u32(field, root.encode()?)?;
        let slot = if pointer == u32::MAX - 1 {
            Some(self.streams.insert_slot()?)
        } else {
            None
        };
        self.streams.load(true, root, size)?;
        self.streams.push(5)?;
        if kind == 24 {
            self.streams.cstring(root)?;
        }
        let name_offset = match kind {
            8 => 72,
            24 => 4,
            _ => 0,
        };
        self.streams.cstring(root.add(name_offset)?)?;
        let name = self
            .string(self.streams.u32(root.add(name_offset)?)?)
            .map_err(|error| format!("Type {kind} name before nested loading: {error}"))?;
        match kind {
            1 => {
                self.streams.cstring(root.add(28)?)?;
            }
            2 => self.physics_constraints(root)?,
            3 => self.destructible(root)?,
            4 => self.animation(root)?,
            5 => self.model(root)?,
            6 => self.material(root)?,
            7 => self.techniques(root)?,
            8 => self.image(root)?,
            9 => self.sound_bank(root)?,
            10 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                self.data(
                    root.add(8)?,
                    3,
                    count.checked_mul(4).ok_or("Sound patch size overflow")?,
                    false,
                )?;
            }
            13 => {
                let count = self.streams.u32(root.add(8)?)? as usize;
                if let Some(lights) = self.data(
                    root.add(12)?,
                    3,
                    count
                        .checked_mul(196)
                        .ok_or("Primary light count overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.streams.cstring(lights.add(index * 196 + 192)?)?;
                    }
                }
            }
            17 => self.world(root)?,
            11 | 12 => self.clip_map(root)?,
            14 | 15 => self.path_data(root.add(4)?)?,
            16 => {
                self.counted_data(root, 4, 8, 1, 0, false)?;
                self.map_triggers(root.add(12)?)?;
            }
            18 => self.asset(8, root.add(4)?)?,
            20 => {
                self.asset(6, root.add(20)?)?;
                self.asset(6, root.add(24)?)?;
                let glyphs = self.streams.u32(root.add(12)?)? as usize;
                self.data(
                    root.add(28)?,
                    3,
                    glyphs.checked_mul(24).ok_or("Font glyph size overflow")?,
                    true,
                )?;
                let kernings = self.streams.u32(root.add(16)?)? as usize;
                self.data(
                    root.add(32)?,
                    3,
                    kernings
                        .checked_mul(8)
                        .ok_or("Font kerning size overflow")?,
                    true,
                )?;
            }
            21 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                if let Some(array) = self.data(
                    root.add(12)?,
                    3,
                    count.checked_mul(24).ok_or("Font icon size overflow")?,
                    true,
                )? {
                    for index in 0..count {
                        let icon = array.add(index * 24)?;
                        self.streams.cstring(icon)?;
                        self.asset(6, icon.add(8)?)?;
                    }
                }
                let count = self.streams.u32(root.add(8)?)? as usize;
                self.data(
                    root.add(16)?,
                    3,
                    count.checked_mul(8).ok_or("Font icon index overflow")?,
                    true,
                )?;
            }
            22 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                if let Some(array) = self.data(
                    root.add(8)?,
                    3,
                    count.checked_mul(4).ok_or("Menu list size overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.asset(23, array.add(index * 4)?)?;
                    }
                }
            }
            24 => (),
            25 => self.weapon(root)?,
            29 => {
                self.streams.cstring(root.add(4)?)?;
            }
            30 => self.attachment(root)?,
            31 => self.camo(root)?,
            23 => self.menu(root)?,
            32 => {
                for (offset, stride) in [
                    (4, 80),
                    (12, 100),
                    (20, 60),
                    (28, 36),
                    (36, 36),
                    (44, 240),
                    (52, 60),
                    (60, 100),
                ] {
                    let count = self.streams.u32(root.add(offset)?)? as usize;
                    self.data(
                        root.add(offset + 4)?,
                        3,
                        count
                            .checked_mul(stride)
                            .ok_or("Native array size overflow")?,
                        false,
                    )?;
                }
            }
            33 => self.effect(root)?,
            34 => {
                if let Some(array) = self.data(root.add(4)?, 3, 21 * 144, false)? {
                    for index in 0..21 * 36 {
                        self.asset(33, array.add(index * 4)?)?;
                    }
                }
            }
            41 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                self.data(
                    root.add(8)?,
                    15,
                    count.checked_add(1).ok_or("Rawfile size overflow")?,
                    false,
                )?;
            }
            42 => {
                let rows = self.streams.u32(root.add(8)?)? as usize;
                let columns = self.streams.u32(root.add(4)?)? as usize;
                let count = rows
                    .checked_mul(columns)
                    .ok_or("String table dimensions overflow")?;
                if let Some(array) = self.data(
                    root.add(12)?,
                    3,
                    count.checked_mul(8).ok_or("String table size overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.streams.cstring(array.add(index * 8)?)?;
                    }
                }
                self.data(
                    root.add(16)?,
                    1,
                    count.checked_mul(2).ok_or("String table index overflow")?,
                    false,
                )?;
            }
            45 => {
                if let Some(definition) = self.data(root.add(4)?, 3, 28, false)? {
                    self.structured_data(definition)?;
                }
            }
            43 => {
                let count = self.streams.u32(root.add(8)?)? as usize;
                if let Some(columns) = self.data(
                    root.add(24)?,
                    3,
                    count
                        .checked_mul(44)
                        .ok_or("Leaderboard column count overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.streams.cstring(columns.add(index * 44)?)?;
                        self.streams.cstring(columns.add(index * 44 + 16)?)?;
                        self.streams.cstring(columns.add(index * 44 + 32)?)?;
                    }
                }
            }
            48 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                self.data(
                    root.add(8)?,
                    31,
                    count.checked_add(1).ok_or("Script size overflow")?,
                    false,
                )?;
            }
            49 => {
                let count = self.streams.u32(root.add(4)?)? as usize;
                if let Some(array) = self.data(
                    root.add(8)?,
                    3,
                    count.checked_mul(12).ok_or("Key-value count overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.streams.cstring(array.add(index * 12 + 8)?)?;
                    }
                }
            }
            50 => self.vehicle(root)?,
            53 => self.asset(6, root.add(8)?)?,
            54 => (),
            57 => (),
            58 => {
                for index in 0..32 {
                    self.asset(33, root.add(4 + index * 4)?)?;
                }
            }
            59 => {
                for index in 0..6 {
                    let board = root.add(80 + index * 80)?;
                    for offset in [0, 4, 8] {
                        self.asset(5, board.add(offset)?)?;
                    }
                    self.streams.cstring(board.add(12)?)?;
                    self.streams.cstring(board.add(16)?)?;
                    self.asset(33, board.add(20)?)?;
                    self.asset(33, board.add(24)?)?;
                }
            }
            _ => unreachable!(),
        }
        if self.string(self.streams.u32(root.add(name_offset)?)?)? != name {
            return Err(format!(
                "Nested loading overwrote native type {kind} {name}"
            ));
        }
        self.streams.pop()?;
        let reference = name.starts_with(',');
        let name = name.strip_prefix(',').unwrap_or(&name).to_owned();
        let retained = if reference {
            if let Some(address) = self.names.get(&(kind, name.clone())) {
                *address
            } else {
                self.retain(root, size)?
            }
        } else {
            let address = self.retain(root, size)?;
            self.names.insert((kind, name.clone()), address);
            address
        };
        self.assets.push(Asset {
            kind,
            name,
            root: retained,
            reference,
        });
        self.streams.set_u32(field, retained.encode()?)?;
        if let Some(slot) = slot {
            self.streams.set_u32(slot, retained.encode()?)?;
        }
        self.streams.pop()?;
        Ok(())
    }

    fn counted_data(
        &mut self,
        root: Address,
        field: usize,
        count: usize,
        stride: usize,
        mask: usize,
        relocate: bool,
    ) -> Result<Option<Address>> {
        let count = self.streams.u32(root.add(count)?)? as usize;
        self.data(
            root.add(field)?,
            mask,
            count
                .checked_mul(stride)
                .ok_or("Native array size overflow")?,
            relocate,
        )
    }

    fn asset_array(
        &mut self,
        field: Address,
        count: usize,
        kind: u32,
        relocate: bool,
    ) -> Result<()> {
        if let Some(array) = self.data(
            field,
            3,
            count.checked_mul(4).ok_or("Asset array size overflow")?,
            relocate,
        )? {
            for index in 0..count {
                self.asset(kind, array.add(index * 4)?)?;
            }
        }
        Ok(())
    }

    fn string_array(&mut self, field: Address, count: usize) -> Result<()> {
        if let Some(array) = self.data(
            field,
            3,
            count.checked_mul(4).ok_or("String array size overflow")?,
            true,
        )? {
            for index in 0..count {
                self.streams.cstring(array.add(index * 4)?)?;
            }
        }
        Ok(())
    }
}
