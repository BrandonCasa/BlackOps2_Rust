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

    fn image(&mut self, root: Address) -> Result<()> {
        let pointer = self.streams.u32(root)?;
        if pointer == 0 {
            return Ok(());
        }
        if pointer != u32::MAX && pointer != u32::MAX - 1 {
            let address = self.streams.alias(pointer)?;
            self.streams.set_u32(root, address.encode()?)?;
            return Ok(());
        }
        self.streams.push(0)?;
        let address = self.streams.alloc(3)?;
        self.streams.set_u32(root, address.encode()?)?;
        let slot = if pointer == u32::MAX - 1 {
            Some(self.streams.insert_slot()?)
        } else {
            None
        };
        self.streams.load(true, address, 12)?;
        let length = self.streams.u32(address.add(8)?)? as usize;
        self.streams.load(true, address.add(12)?, length)?;
        let address = self.retain(
            address,
            length.checked_add(12).ok_or("Image size overflow")?,
        )?;
        self.streams.set_u32(root, address.encode()?)?;
        if let Some(slot) = slot {
            self.streams.set_u32(slot, address.encode()?)?;
        }
        self.streams.pop()?;
        Ok(())
    }

    fn vehicle(&mut self, root: Address) -> Result<()> {
        for offset in [
            0x16c, 0x19c, 0x1a0, 0x1a4, 0x1a8, 0x298, 0x29c, 0x2a8, 0x2ac, 0x2b0, 700, 0x2c8,
        ] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [
            0x2ec, 0x2f0, 0x2f4, 0x2f8, 0x304, 0x308, 0x30c, 0x310, 0x324, 0x328, 0x32c, 0x330,
        ] {
            self.asset(5, root.add(offset)?)?;
        }
        self.asset(33, root.add(0x338)?)?;
        for index in 0..32 {
            self.asset(33, root.add(0x340 + index * 4)?)?;
        }
        self.asset(33, root.add(0x3c0)?)?;
        self.streams.cstring(root.add(0x3c8)?)?;
        for index in 0..4 {
            self.asset(33, root.add(0x3cc + index * 4)?)?;
        }
        self.asset(33, root.add(0x3e4)?)?;
        self.asset(33, root.add(0x3ec)?)?;
        for offset in [0x400, 0x404, 0x414, 0x450] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.asset(6, root.add(0x454)?)?;
        for offset in [
            0x458, 0x460, 0x468, 0x470, 0x478, 0x480, 0x488, 0x490, 0x498, 0x4a0, 0x4a8, 0x5b0,
        ] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for index in 0..40 {
            self.streams.cstring(root.add(0x644 + index * 12)?)?;
        }
        for base in [0x858, 0x8e4] {
            for index in 0..5 {
                self.streams.cstring(root.add(base + index * 28)?)?;
            }
        }
        self.streams.cstring(root.add(0xa10)?)?;
        Ok(())
    }

    fn clip_map(&mut self, root: Address) -> Result<()> {
        self.collision_data(root.add(8)?)?;
        self.collision_pointer(root.add(80)?)?;
        let count = self.streams.u32(root.add(84)?)? as usize;
        if let Some(models) = self.data(
            root.add(88)?,
            3,
            count
                .checked_mul(84)
                .ok_or("Collision model count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.asset(5, models.add(index * 84 + 4)?)?;
            }
        }
        let count = self.streams.u32(root.add(92)?)? as usize;
        if let Some(nodes) = self.data(
            root.add(96)?,
            3,
            count
                .checked_mul(8)
                .ok_or("Collision node count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.data(nodes.add(index * 8)?, 3, 20, true)?;
            }
        }
        for (field, count, stride, mask) in [(104, 100, 44, 3), (112, 108, 12, 3), (120, 116, 6, 1)]
        {
            self.counted_data(root, field, count, stride, mask, false)?;
        }
        let triangles = self.streams.u32(root.add(116)?)? as usize;
        self.data(
            root.add(124)?,
            0,
            (triangles
                .checked_mul(3)
                .and_then(|size| size.checked_add(31))
                .ok_or("Triangle edge count overflow")?
                >> 3)
                & !3,
            false,
        )?;
        self.counted_data(root, 132, 128, 16, 3, false)?;
        self.counted_data(root, 140, 136, 32, 15, false)?;
        let count = self.streams.u32(root.add(144)?)? as usize;
        if let Some(models) = self.data(
            root.add(148)?,
            3,
            count.checked_mul(76).ok_or("Clip model count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.collision_pointer(models.add(index * 76 + 28)?)?;
            }
        }
        let clusters = self.streams.u32(root.add(152)?)? as usize;
        let bytes = self.streams.u32(root.add(156)?)? as usize;
        self.data(
            root.add(160)?,
            0,
            clusters
                .checked_mul(bytes)
                .ok_or("Collision visibility size overflow")?,
            false,
        )?;
        self.asset(16, root.add(168)?)?;
        if let Some(brush) = self.data(root.add(172)?, 15, 96, true)? {
            self.collision_brush(brush)?;
        }
        self.collision_pointer(root.add(204)?)?;
        for (field, count_offset) in [(264, 254), (268, 256)] {
            let count = self.streams.u16(root.add(count_offset)?)? as usize;
            if let Some(entities) = self.data(root.add(field)?, 3, count * 84, false)? {
                for index in 0..count {
                    let entity = entities.add(index * 84)?;
                    self.asset(5, entity.add(32)?)?;
                    self.asset(5, entity.add(36)?)?;
                    self.asset(33, entity.add(44)?)?;
                    if let Some(pieces) = self.data(entity.add(52)?, 3, 12, true)? {
                        self.streams.cstring(pieces)?;
                        let count = self.streams.u32(pieces.add(4)?)? as usize;
                        if let Some(models) = self.data(
                            pieces.add(8)?,
                            3,
                            count.checked_mul(16).ok_or("Model piece count overflow")?,
                            false,
                        )? {
                            for index in 0..count {
                                self.asset(5, models.add(index * 16)?)?;
                            }
                        }
                    }
                    self.asset(1, entity.add(56)?)?;
                }
            }
        }
        for (field, count_offset, stride) in [
            (272, 254, 32),
            (276, 256, 32),
            (280, 254, 20),
            (284, 256, 20),
            (288, 258, 8),
            (292, 260, 8),
            (296, 254, 32),
            (300, 256, 32),
            (304, 258, 32),
            (308, 260, 32),
        ] {
            self.streams.push(1)?;
            self.data(
                root.add(field)?,
                3,
                self.streams.u16(root.add(count_offset)?)? as usize * stride,
                false,
            )?;
            self.streams.pop()?;
        }
        let count = self.streams.u32(root.add(312)?)? as usize;
        if let Some(constraints) = self.data(
            root.add(316)?,
            3,
            count
                .checked_mul(168)
                .ok_or("Map constraint count overflow")?,
            false,
        )? {
            for index in 0..count {
                let constraint = constraints.add(index * 168)?;
                self.streams.cstring(constraint.add(20)?)?;
                self.streams.cstring(constraint.add(36)?)?;
                self.asset(6, constraint.add(140)?)?;
            }
        }
        self.streams.push(1)?;
        self.counted_data(root, 324, 320, 3188, 3, false)?;
        self.streams.pop()?;
        Ok(())
    }

    fn collision_pointer(&mut self, field: Address) -> Result<()> {
        let pointer = self.streams.u32(field)?;
        if pointer == 0 {
            return Ok(());
        }
        if pointer != u32::MAX && pointer != u32::MAX - 1 {
            let address = self.streams.alias(pointer)?;
            self.streams.set_u32(field, address.encode()?)?;
            return Ok(());
        }
        self.streams.push(0)?;
        let root = self.streams.alloc(3)?;
        self.streams.set_u32(field, root.encode()?)?;
        let slot = if pointer == u32::MAX - 1 {
            Some(self.streams.insert_slot()?)
        } else {
            None
        };
        self.streams.load(true, root, 72)?;
        self.collision_data(root)?;
        let retained = self.retain(root, 72)?;
        self.streams.set_u32(field, retained.encode()?)?;
        if let Some(slot) = slot {
            self.streams.set_u32(slot, retained.encode()?)?;
        }
        self.streams.pop()?;
        Ok(())
    }

    fn collision_data(&mut self, root: Address) -> Result<()> {
        self.counted_data(root, 4, 0, 20, 3, true)?;
        let count = self.streams.u32(root.add(8)?)? as usize;
        if let Some(materials) = self.data(
            root.add(12)?,
            3,
            count
                .checked_mul(12)
                .ok_or("Clip material count overflow")?,
            true,
        )? {
            for index in 0..count {
                self.streams.cstring(materials.add(index * 12)?)?;
            }
        }
        let count = self.streams.u32(root.add(16)?)? as usize;
        if let Some(sides) = self.data(
            root.add(20)?,
            3,
            count.checked_mul(12).ok_or("Clip side count overflow")?,
            true,
        )? {
            for index in 0..count {
                self.data(sides.add(index * 12)?, 3, 20, true)?;
            }
        }
        let count = self.streams.u32(root.add(24)?)? as usize;
        if let Some(nodes) = self.data(
            root.add(28)?,
            3,
            count.checked_mul(20).ok_or("Brush node count overflow")?,
            true,
        )? {
            for index in 0..count {
                let node = nodes.add(index * 20)?;
                let count = self.streams.u16(node.add(2)?)? as i16;
                if count > 0 {
                    self.data(node.add(8)?, 1, count as usize * 2, true)?;
                }
            }
        }
        self.counted_data(root, 36, 32, 2, 1, true)?;
        self.counted_data(root, 44, 40, 12, 3, false)?;
        self.counted_data(root, 52, 48, 2, 1, false)?;
        let count = self.streams.u16(root.add(56)?)? as usize;
        if let Some(brushes) = self.data(root.add(60)?, 127, count * 96, true)? {
            for index in 0..count {
                self.collision_brush(brushes.add(index * 96)?)?;
            }
        }
        self.data(root.add(64)?, 127, count * 24, true)?;
        self.data(root.add(68)?, 3, count * 4, true)?;
        Ok(())
    }

    fn collision_brush(&mut self, root: Address) -> Result<()> {
        if let Some(side) = self.data(root.add(32)?, 3, 12, true)? {
            self.data(side, 3, 20, true)?;
        }
        self.data(root.add(88)?, 3, 12, true)?;
        Ok(())
    }

    fn map_triggers(&mut self, root: Address) -> Result<()> {
        for (field, count, stride) in [(4, 0, 8), (12, 8, 32), (20, 16, 20)] {
            self.counted_data(root, field, count, stride, 3, true)?;
        }
        Ok(())
    }

    fn path_data(&mut self, root: Address) -> Result<()> {
        let count = (self.streams.u32(root)? as usize)
            .checked_add(128)
            .ok_or("Path node count overflow")?;
        if let Some(nodes) = self.data(
            root.add(8)?,
            3,
            count.checked_mul(144).ok_or("Path node size overflow")?,
            false,
        )? {
            for index in 0..count {
                let node = nodes.add(index * 144)?;
                self.data(
                    node.add(64)?,
                    3,
                    self.streams.u16(node.add(60)?)? as usize * 16,
                    false,
                )?;
            }
        }
        self.streams.push(1)?;
        self.data(
            root.add(12)?,
            15,
            count
                .checked_mul(16)
                .ok_or("Path base node size overflow")?,
            false,
        )?;
        self.streams.pop()?;
        self.counted_data(root, 20, 16, 1, 0, false)?;
        self.counted_data(root, 28, 24, 1, 0, false)?;
        let count = self.streams.u32(root.add(32)?)? as usize;
        if let Some(trees) = self.data(
            root.add(36)?,
            3,
            count.checked_mul(16).ok_or("Path tree count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.path_tree(trees.add(index * 16)?)?;
            }
        }
        Ok(())
    }

    fn path_tree(&mut self, root: Address) -> Result<()> {
        if (self.streams.u32(root)? as i32) < 0 {
            self.counted_data(root, 12, 8, 2, 1, false)?;
        } else {
            for offset in [8, 12] {
                if let Some(tree) = self.data(root.add(offset)?, 3, 16, true)? {
                    self.path_tree(tree)?;
                }
            }
        }
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

    fn world(&mut self, root: Address) -> Result<()> {
        self.streams.cstring(root.add(4)?)?;
        self.counted_data(root, 0x18, 0x14, 48, 15, false)?;
        self.counted_data(root, 0x20, 0x1c, 4, 3, false)?;
        self.streams.cstring(root.add(0x24)?)?;
        if let Some(light) = self.data(root.add(0x100)?, 15, 352, true)? {
            self.asset(18, light.add(336)?)?;
        }
        for (field, stride) in [
            (0x110, 32),
            (0x118, 16),
            (0x120, 16),
            (0x128, 24),
            (0x130, 16),
            (0x138, 100),
            (0x140, 16),
            (0x148, 48),
            (0x150, 16),
            (0x158, 36),
            (0x160, 16),
        ] {
            self.counted_data(root, field, field - 4, stride, 3, false)?;
        }
        self.counted_data(root, 0x178, 8, 20, 3, true)?;
        self.counted_data(root, 0x17c, 12, 2, 1, false)?;
        self.streams.push(1)?;
        self.counted_data(root, 0x180, 0x174, 2048, 3, false)?;
        self.streams.pop()?;
        let cell_count = self.streams.u32(root.add(0x174)?)? as usize;
        if let Some(cells) = self.data(
            root.add(0x188)?,
            3,
            cell_count.checked_mul(48).ok_or("Cell count overflow")?,
            false,
        )? {
            for index in 0..cell_count {
                self.world_cell(cells.add(index * 48)?)?;
            }
        }
        self.world_draw(root.add(0x18c)?)?;
        self.world_light_grid(root.add(0x1d0)?)?;
        self.counted_data(root, 0x21c, 0x218, 64, 15, false)?;
        let count = self.streams.u32(root.add(0x23c)?)? as usize;
        if let Some(materials) = self.data(
            root.add(0x240)?,
            3,
            count
                .checked_mul(8)
                .ok_or("Material memory count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.asset(6, materials.add(index * 8)?)?;
            }
        }
        self.asset(6, root.add(0x248)?)?;
        self.asset(6, root.add(0x24c)?)?;
        self.asset(8, root.add(0x2e4)?)?;
        for (field, count, stride) in [
            (
                0x2e8,
                cell_count.checked_add(31).ok_or("Cell count overflow")? / 32,
                cell_count.checked_mul(4).ok_or("Cell bits overflow")?,
            ),
            (0x2ec, self.streams.u32(root.add(0x38c)?)? as usize, 8),
            (0x2f0, self.streams.u32(root.add(0x390)?)? as usize, 4),
        ] {
            self.streams.push(1)?;
            self.data(
                root.add(field)?,
                3,
                count
                    .checked_mul(stride)
                    .ok_or("World visibility size overflow")?,
                false,
            )?;
            self.streams.pop()?;
        }
        for (field, stride) in [
            (0x2f4, 0x8000),
            (0x2f8, self.streams.u32(root.add(0x38c)?)? as usize * 4),
            (0x2fc, self.streams.u32(root.add(0x390)?)? as usize * 4),
        ] {
            if self.streams.u32(root.add(field)?)? != 0 {
                let count = self
                    .streams
                    .u32(root.add(0x108)?)?
                    .checked_sub(self.streams.u32(root.add(0x104)?)?)
                    .and_then(|count| count.checked_sub(1))
                    .ok_or("Invalid primary light range")? as usize;
                self.streams.push(1)?;
                self.data(
                    root.add(field)?,
                    3,
                    count
                        .checked_mul(stride)
                        .ok_or("Light visibility size overflow")?,
                    false,
                )?;
                self.streams.pop()?;
            }
        }
        let count = self.streams.u32(root.add(0x300)?)? as usize;
        if let Some(instances) = self.data(
            root.add(0x304)?,
            15,
            count
                .checked_mul(96)
                .ok_or("Siege instance count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.siege_instance(instances.add(index * 96)?)?;
            }
        }
        let count = self.streams.u32(root.add(0x108)?)? as usize;
        if let Some(geometry) = self.data(
            root.add(0x308)?,
            3,
            count
                .checked_mul(12)
                .ok_or("Shadow geometry count overflow")?,
            false,
        )? {
            for index in 0..count {
                let geometry = geometry.add(index * 12)?;
                for (field, count) in [(4, 0), (8, 2)] {
                    self.data(
                        geometry.add(field)?,
                        1,
                        self.streams.u16(geometry.add(count)?)? as usize * 2,
                        false,
                    )?;
                }
            }
        }
        if let Some(regions) = self.data(
            root.add(0x30c)?,
            3,
            count.checked_mul(8).ok_or("Light region count overflow")?,
            false,
        )? {
            for index in 0..count {
                let region = regions.add(index * 8)?;
                let count = self.streams.u32(region)? as usize;
                if let Some(hulls) = self.data(
                    region.add(4)?,
                    3,
                    count.checked_mul(80).ok_or("Light hull count overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        self.counted_data(hulls.add(index * 80)?, 76, 72, 20, 3, false)?;
                    }
                }
            }
        }
        self.world_static(root)?;
        let dynamic = root.add(0x384)?;
        for (field, count, stride, mask) in [
            (16, 0, cell_count * 4, 3),
            (20, 4, cell_count * 4, 3),
            (24, 0, 32, 15),
            (36, 4, 32, 15),
            (28, 0, 32, 15),
            (40, 4, 32, 15),
            (32, 0, 32, 15),
            (44, 4, 32, 15),
        ] {
            self.streams.push(1)?;
            self.counted_data(dynamic, field, count, stride, mask, false)?;
            self.streams.pop()?;
        }
        for index in 0..2 {
            let buffer = root.add(0x3bc + index * 8)?;
            self.data(
                buffer.add(4)?,
                3,
                self.streams.u32(buffer)? as usize & !15,
                false,
            )?;
        }
        for field in [0x3cc, 0x3d0, 0x3d4, 0x3d8] {
            self.asset(6, root.add(field)?)?;
        }
        for (field, count, stride) in [
            (0x3e0, 0x3dc, 68),
            (0x3e8, 0x3e4, 24),
            (0x3f4, 0x3ec, 56),
            (0x3f8, 0x3f0, 32),
        ] {
            self.counted_data(root, field, count, stride, 3, false)?;
        }
        Ok(())
    }

    fn world_cell(&mut self, cell: Address) -> Result<()> {
        let count = self.streams.u32(cell.add(24)?)? as usize;
        if let Some(trees) = self.data(
            cell.add(28)?,
            3,
            count.checked_mul(40).ok_or("Cell tree count overflow")?,
            false,
        )? {
            for index in 0..count {
                let tree = trees.add(index * 40)?;
                self.data(
                    tree.add(32)?,
                    1,
                    self.streams.u16(tree.add(30)?)? as usize * 2,
                    true,
                )?;
            }
        }
        let count = self.streams.u32(cell.add(32)?)? as usize;
        if let Some(portals) = self.data(
            cell.add(36)?,
            3,
            count.checked_mul(92).ok_or("Portal count overflow")?,
            false,
        )? {
            for index in 0..count {
                let portal = portals.add(index * 92)?;
                if let Some(cell) = self.data(portal.add(32)?, 3, 48, true)? {
                    self.world_cell(cell)?;
                }
                self.data(
                    portal.add(36)?,
                    3,
                    self.streams.u8(portal.add(40)?)? as usize * 12,
                    false,
                )?;
            }
        }
        self.data(
            cell.add(44)?,
            0,
            self.streams.u8(cell.add(40)?)? as usize,
            false,
        )?;
        Ok(())
    }

    fn world_draw(&mut self, draw: Address) -> Result<()> {
        let count = self.streams.u32(draw)? as usize;
        if let Some(probes) = self.data(
            draw.add(4)?,
            3,
            count
                .checked_mul(76)
                .ok_or("Reflection probe count overflow")?,
            false,
        )? {
            for index in 0..count {
                let probe = probes.add(index * 76)?;
                self.asset(8, probe.add(60)?)?;
                self.counted_data(probe, 64, 68, 96, 3, false)?;
            }
        }
        self.streams.push(1)?;
        self.counted_data(draw, 8, 0, 4, 3, false)?;
        self.streams.pop()?;
        let count = self.streams.u32(draw.add(12)?)? as usize;
        if let Some(lightmaps) = self.data(
            draw.add(16)?,
            3,
            count.checked_mul(8).ok_or("Lightmap count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.asset(8, lightmaps.add(index * 8)?)?;
                self.asset(8, lightmaps.add(index * 8 + 4)?)?;
            }
        }
        for field in [20, 24] {
            self.streams.push(1)?;
            self.counted_data(draw, field, 12, 4, 3, false)?;
            self.streams.pop()?;
        }
        self.counted_data(draw, 36, 32, 1, 127, false)?;
        self.counted_data(draw, 48, 44, 1, 127, false)?;
        self.counted_data(draw, 60, 56, 2, 1, false)?;
        Ok(())
    }

    fn world_light_grid(&mut self, grid: Address) -> Result<()> {
        if self.streams.u32(grid.add(28)?)? != 0 {
            let axis = self.streams.u32(grid.add(20)?)? as usize;
            if axis > 2 {
                return Err("Invalid light grid row axis".into());
            }
            let first = self.streams.u16(grid.add(4 + axis * 2)?)? as usize;
            let last = self.streams.u16(grid.add(10 + axis * 2)?)? as usize;
            let count = last
                .checked_sub(first)
                .and_then(|count| count.checked_add(1))
                .ok_or("Invalid light grid bounds")?;
            self.data(grid.add(28)?, 1, count * 2, false)?;
        }
        for (field, count, stride, mask) in [
            (36, 32, 1, 3),
            (44, 40, 4, 3),
            (52, 48, 168, 3),
            (60, 56, 54, 3),
            (68, 64, 40, 3),
        ] {
            self.counted_data(grid, field, count, stride, mask, false)?;
        }
        Ok(())
    }

    fn world_static(&mut self, world: Address) -> Result<()> {
        let root = world.add(0x310)?;
        for (field, count) in [
            (48, 40),
            (52, 40),
            (56, 40),
            (60, 44),
            (64, 44),
            (68, 44),
            (72, 40),
            (76, 44),
            (100, 44),
            (104, 44),
        ] {
            self.streams.push(1)?;
            self.counted_data(root, field, count, 1, 127, false)?;
            self.streams.pop()?;
        }
        self.counted_data(root, 108, 40, 1, 127, false)?;
        self.counted_data(root, 80, 4, 2, 1, false)?;
        self.counted_data(root, 84, 0, 36, 3, false)?;
        let count = self.streams.u32(world.add(16)?)? as usize;
        if let Some(surfaces) = self.data(
            root.add(88)?,
            15,
            count
                .checked_mul(80)
                .ok_or("World surface count overflow")?,
            false,
        )? {
            for index in 0..count {
                self.asset(6, surfaces.add(index * 80 + 48)?)?;
            }
        }
        let count = self.streams.u32(root)? as usize;
        if let Some(models) = self.data(
            root.add(92)?,
            3,
            count
                .checked_mul(152)
                .ok_or("Static model count overflow")?,
            false,
        )? {
            for index in 0..count {
                let model = models.add(index * 152)?;
                self.asset(5, model.add(56)?)?;
                for index in 0..4 {
                    let lighting = model.add(104 + index * 12)?;
                    self.data(
                        lighting,
                        3,
                        self.streams.u16(lighting.add(8)?)? as usize * 4,
                        false,
                    )?;
                }
            }
        }
        self.streams.push(1)?;
        self.counted_data(root, 96, 4, 8, 3, false)?;
        self.streams.pop()?;
        Ok(())
    }

    fn siege_instance(&mut self, root: Address) -> Result<()> {
        if let Some(shaders) = self.data(root.add(64)?, 3, 20, true)? {
            self.counted_data(shaders, 0, 12, 1, 127, false)?;
            self.counted_data(shaders, 4, 12, 1, 3, false)?;
            self.counted_data(shaders, 8, 16, 1, 3, false)?;
        }
        if let Some(model) = self.data(root.add(68)?, 3, 16, true)? {
            self.counted_data(model, 8, 0, 16, 3, false)?;
            self.counted_data(model, 12, 4, 6, 1, false)?;
        }
        if let Some(animation) = self.data(root.add(72)?, 3, 16, true)? {
            let count = self.streams.u32(animation.add(4)?)? as usize;
            let frames = self.streams.u32(animation.add(8)?)? as usize;
            self.data(
                animation.add(12)?,
                127,
                count
                    .checked_mul(frames)
                    .and_then(|count| count.checked_mul(16))
                    .ok_or("Siege animation size overflow")?,
                false,
            )?;
        }
        if self.streams.u32(root.add(76)?)? != 0 {
            let model = Address::decode(self.streams.u32(root.add(68)?)?)?;
            self.data(
                root.add(76)?,
                3,
                self.streams.u32(model)? as usize * 16,
                false,
            )?;
        }
        Ok(())
    }

    fn destructible(&mut self, root: Address) -> Result<()> {
        self.asset(5, root.add(4)?)?;
        self.asset(5, root.add(8)?)?;
        let count = self.streams.u32(root.add(12)?)? as usize;
        if let Some(pieces) = self.data(
            root.add(16)?,
            3,
            count
                .checked_mul(312)
                .ok_or("Destructible piece count overflow")?,
            false,
        )? {
            for index in 0..count {
                let piece = pieces.add(index * 312)?;
                for index in 0..5 {
                    let stage = piece.add(index * 48)?;
                    self.asset(33, stage.add(16)?)?;
                    for offset in [20, 24, 28] {
                        self.streams.cstring(stage.add(offset)?)?;
                    }
                    for offset in [32, 36, 40] {
                        self.asset(5, stage.add(offset)?)?;
                    }
                    self.asset(1, stage.add(44)?)?;
                }
                self.asset(2, piece.add(268)?)?;
                self.streams.cstring(piece.add(276)?)?;
                self.asset(33, piece.add(280)?)?;
                self.streams.cstring(piece.add(284)?)?;
            }
        }
        Ok(())
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

    fn camo(&mut self, root: Address) -> Result<()> {
        self.asset(8, root.add(4)?)?;
        self.asset(8, root.add(8)?)?;
        let count = self.streams.u32(root.add(16)?)? as usize;
        if let Some(sets) = self.data(
            root.add(12)?,
            3,
            count.checked_mul(20).ok_or("Camo set size overflow")?,
            false,
        )? {
            for index in 0..count {
                self.asset(8, sets.add(index * 20)?)?;
                self.asset(8, sets.add(index * 20 + 4)?)?;
            }
        }
        let count = self.streams.u32(root.add(24)?)? as usize;
        if let Some(sets) = self.data(
            root.add(20)?,
            3,
            count.checked_mul(8).ok_or("Camo material sets overflow")?,
            false,
        )? {
            for index in 0..count {
                let set = sets.add(index * 8)?;
                let count = self.streams.u32(set)? as usize;
                if let Some(materials) = self.data(
                    set.add(4)?,
                    3,
                    count
                        .checked_mul(44)
                        .ok_or("Camo material array overflow")?,
                    false,
                )? {
                    for index in 0..count {
                        let material = materials.add(index * 44)?;
                        let count = self.streams.u16(material.add(2)?)? as usize;
                        self.asset_array(material.add(4)?, count, 6, false)?;
                        self.asset_array(material.add(8)?, count, 6, false)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn attachment(&mut self, root: Address) -> Result<()> {
        for offset in [20, 28] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.data(root.add(36)?, 1, 32 * 2, true)?;
        for offset in [40, 44, 48, 52, 56] {
            self.asset(5, root.add(offset)?)?;
        }
        for offset in [60, 64] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.asset(31, root.add(164)?)?;
        self.asset(6, root.add(196)?)?;
        self.asset(6, root.add(200)?)?;
        self.string_array(root.add(232)?, 88)?;
        self.data(root.add(248)?, 3, 21 * 4, true)?;
        for offset in (256..312).step_by(4) {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [316, 320] {
            self.asset(33, root.add(offset)?)?;
        }
        for offset in [324, 328] {
            self.asset(53, root.add(offset)?)?;
        }
        Ok(())
    }

    fn weapon(&mut self, root: Address) -> Result<()> {
        if let Some(definition) = self.data(root.add(8)?, 3, 2448, true)? {
            self.weapon_definition(definition)?;
        }
        for offset in [12, 16, 20] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.asset_array(root.add(24)?, 63, 29, true)?;
        self.asset_array(root.add(28)?, 95, 30, true)?;
        self.string_array(root.add(32)?, 88)?;
        self.data(root.add(36)?, 1, 32 * 2, true)?;
        self.asset_array(root.add(40)?, 8, 5, true)?;
        self.asset_array(root.add(44)?, 8, 5, true)?;
        self.string_array(root.add(48)?, 8)?;
        self.string_array(root.add(52)?, 8)?;
        for offset in [508, 512, 520] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [596, 600, 604] {
            self.asset(6, root.add(offset)?)?;
        }
        Ok(())
    }

    fn weapon_definition(&mut self, root: Address) -> Result<()> {
        self.streams.cstring(root)?;
        self.asset_array(root.add(4)?, 16, 5, true)?;
        self.asset(5, root.add(8)?)?;
        self.streams.cstring(root.add(12)?)?;
        for offset in [16, 20] {
            self.data(root.add(offset)?, 1, 20 * 2, true)?;
        }
        self.streams.cstring(root.add(64)?)?;
        for offset in [108, 112, 116] {
            self.asset(33, root.add(offset)?)?;
        }
        for offset in (148..436).step_by(4) {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.string_array(root.add(436)?, 32)?;
        for offset in [440, 444, 448] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [464, 468, 472, 476] {
            self.asset(33, root.add(offset)?)?;
        }
        for offset in [528, 532] {
            self.asset(6, root.add(offset)?)?;
        }
        self.asset_array(root.add(916)?, 16, 5, true)?;
        for offset in [920, 924, 928, 932] {
            self.asset(5, root.add(offset)?)?;
        }
        for offset in [936, 940, 956] {
            self.asset(6, root.add(offset)?)?;
        }
        self.streams.cstring(root.add(980)?)?;
        for offset in (1100..1124).step_by(4) {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.streams.cstring(root.add(1388)?)?;
        self.asset(6, root.add(1632)?)?;
        self.asset(6, root.add(948)?)?;
        for offset in [1652, 1656] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.asset(5, root.add(1768)?)?;
        for offset in [1776, 1784, 1792, 1800, 1808, 1816] {
            self.asset(33, root.add(offset)?)?;
        }
        for offset in [1820, 1824, 1828, 1832] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [1880, 1884] {
            self.data(root.add(offset)?, 3, 32 * 4, true)?;
        }
        for offset in [1888, 1916] {
            self.asset(33, root.add(offset)?)?;
        }
        for offset in [1920, 2124] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for (name, count_offset, offsets) in
            [(None, 2148, [2132, 2140]), (Some(2128), 2152, [2136, 2144])]
        {
            if let Some(name) = name {
                self.streams.cstring(root.add(name)?)?;
            }
            let count = self.streams.u32(root.add(count_offset)?)? as usize;
            for offset in offsets {
                self.data(
                    root.add(offset)?,
                    3,
                    count
                        .checked_mul(8)
                        .ok_or("Weapon accuracy graph size overflow")?,
                    true,
                )?;
            }
        }
        for offset in [2236, 2240, 2284] {
            self.streams.cstring(root.add(offset)?)?;
        }
        self.data(root.add(2300)?, 3, 21 * 4, true)?;
        for offset in [2304, 2308, 2312, 2316] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [2320, 2324] {
            self.asset(53, root.add(offset)?)?;
        }
        for offset in [2356, 2360] {
            self.streams.cstring(root.add(offset)?)?;
        }
        for offset in [2364, 2368] {
            if let Some(table) = self.data(root.add(offset)?, 3, 484, true)? {
                self.streams.cstring(table.add(432)?)?;
                for offset in (436..468).step_by(4) {
                    self.asset(6, table.add(offset)?)?;
                }
                for offset in (468..484).step_by(4) {
                    self.streams.cstring(table.add(offset)?)?;
                }
            }
        }
        for offset in [2372, 2376, 2404, 2408, 2412] {
            self.asset(33, root.add(offset)?)?;
        }
        self.streams.cstring(root.add(2416)?)?;
        self.asset(31, root.add(2420)?)?;
        Ok(())
    }

    fn expression(&mut self, root: Address) -> Result<()> {
        self.streams.cstring(root)?;
        let count = self.streams.u32(root.add(8)?)? as usize;
        if let Some(array) = self
            .data(
                root.add(12)?,
                3,
                count
                    .checked_mul(12)
                    .ok_or("Menu expression size overflow")?,
                true,
            )
            .map_err(|error| format!("Expression at {root:?}, count {count}: {error}"))?
        {
            for index in 0..count {
                let operand = array.add(index * 12)?;
                if self.streams.u32(operand)? == 0 && self.streams.u32(operand.add(4)?)? == 2 {
                    self.streams.cstring(operand.add(8)?)?;
                }
            }
        }
        Ok(())
    }

    fn menu(&mut self, root: Address) -> Result<()> {
        self.streams.cstring(root.add(52)?)?;
        self.asset(6, root.add(160)?)?;
        self.streams.cstring(root.add(164)?)?;
        self.menu_handlers(root.add(268)?, false)?;
        self.menu_handlers(root.add(272)?, true)?;
        self.expression(root.add(276)?)?;
        self.streams.cstring(root.add(312)?)?;
        self.streams.cstring(root.add(316)?)?;
        self.expression(root.add(360)?)?;
        self.expression(root.add(376)?)?;
        let count = self.streams.u32(root.add(176)?)? as usize;
        if let Some(array) = self.data(
            root.add(392)?,
            3,
            count.checked_mul(4).ok_or("Menu item array overflow")?,
            false,
        )? {
            for index in 0..count {
                if let Some(item) = self.data(array.add(index * 4)?, 7, 272, false)? {
                    self.menu_item(item)?;
                }
            }
        }
        Ok(())
    }

    fn menu_handlers(&mut self, mut field: Address, key: bool) -> Result<()> {
        while let Some(handler) = self.data(field, 3, 12, false)? {
            if !key {
                self.streams.cstring(handler)?;
            }
            self.menu_scripts(handler.add(4)?)?;
            field = handler.add(8)?;
        }
        Ok(())
    }

    fn menu_scripts(&mut self, mut field: Address) -> Result<()> {
        while let Some(script) = self.data(field, 3, 44, false)? {
            let mut condition_field = script;
            while let Some(condition) = self.data(condition_field, 3, 16, false)? {
                condition_field = condition.add(12)?;
            }
            self.expression(script.add(4)?)
                .map_err(|error| format!("Menu script at {script:?}: {error}"))?;
            self.streams.cstring(script.add(28)?)?;
            field = script.add(40)?;
        }
        Ok(())
    }

    fn menu_item(&mut self, root: Address) -> Result<()> {
        self.streams.cstring(root)?;
        self.streams.cstring(root.add(52)?)?;
        self.asset(6, root.add(160)?)?;
        for offset in [176, 180, 184, 188] {
            self.streams.cstring(root.add(offset)?)?;
        }
        let kind = self.streams.u32(root.add(164)?)?;
        match kind {
            1 | 3..=5 | 7..=16 | 18 | 20 | 22 => {
                if let Some(text) = self.data(root.add(196)?, 3, 140, false)? {
                    self.streams.cstring(text.add(128)?)?;
                    if let Some(expression) = self.data(text.add(132)?, 3, 16, false)? {
                        self.expression(expression)?;
                    }
                    if kind == 15 {
                        self.data(text.add(136)?, 3, 8, false)?;
                    } else if kind != 1 && kind != 18 {
                        self.menu_focus(text.add(136)?, kind)?;
                    }
                }
            }
            2 | 6 => {
                if let Some(expression) = self.data(root.add(196)?, 3, 16, false)? {
                    self.expression(expression)?;
                }
            }
            19 | 21 => self.menu_focus(root.add(196)?, kind)?,
            _ => (),
        }
        if let Some(rect) = self.data(root.add(204)?, 3, 64, false)? {
            for offset in [0, 16, 32, 48] {
                self.expression(rect.add(offset)?)?;
            }
        }
        self.expression(root.add(208)?)?;
        self.expression(root.add(240)?)?;
        self.menu_handlers(root.add(260)?, false)?;
        if let Some(animation) = self.data(root.add(264)?, 3, 236, false)? {
            let count = self.streams.u32(animation)? as usize;
            if let Some(states) = self.data(
                animation.add(4)?,
                3,
                count
                    .checked_mul(4)
                    .ok_or("Menu animation array overflow")?,
                false,
            )? {
                for index in 0..count {
                    if let Some(state) = self.data(states.add(index * 4)?, 3, 108, false)? {
                        self.streams.cstring(state)?;
                        self.menu_handlers(state.add(104)?, false)?;
                    }
                }
            }
            for offset in [8, 116] {
                self.streams.cstring(animation.add(offset)?)?;
                self.menu_handlers(animation.add(offset + 104)?, false)?;
            }
        }
        Ok(())
    }

    fn menu_focus(&mut self, field: Address, kind: u32) -> Result<()> {
        if let Some(focus) = self.data(field, 3, 24, false)? {
            for offset in [0, 4, 8, 12] {
                self.streams.cstring(focus.add(offset)?)?;
            }
            self.menu_handlers(focus.add(16)?, true)?;
            let field = focus.add(20)?;
            match kind {
                10 | 22 => {
                    if let Some(multi) = self.data(field, 3, 396, false)? {
                        for index in 0..64 {
                            self.streams.cstring(multi.add(index * 4)?)?;
                        }
                    }
                }
                11 => {
                    if let Some(value) = self.data(field, 3, 4, false)? {
                        self.streams.cstring(value)?;
                    }
                }
                5 | 7..=9 | 12..=14 | 16 | 30 => {
                    self.data(field, 3, 36, false)?;
                }
                4 if self.streams.u32(field)? != 0 => {
                    return Err("Menu list-box traversal is not implemented".into());
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn sound_bank(&mut self, root: Address) -> Result<()> {
        let count = self.streams.u32(root.add(4)?)? as usize;
        if let Some(array) = self.data(
            root.add(8)?,
            3,
            count.checked_mul(20).ok_or("Sound list size overflow")?,
            false,
        )? {
            for index in 0..count {
                let list = array.add(index * 20)?;
                self.streams.cstring(list)?;
                let count = self.streams.u32(list.add(12)?)? as usize;
                if let Some(aliases) = self.data(
                    list.add(8)?,
                    3,
                    count.checked_mul(96).ok_or("Sound alias size overflow")?,
                    true,
                )? {
                    for index in 0..count {
                        for offset in [0, 8, 12, 20] {
                            self.streams.cstring(aliases.add(index * 96 + offset)?)?;
                        }
                    }
                }
            }
        }
        self.data(
            root.add(12)?,
            3,
            count.checked_mul(4).ok_or("Sound list index overflow")?,
            false,
        )?;
        let count = self.streams.u32(root.add(16)?)? as usize;
        self.data(
            root.add(20)?,
            3,
            count.checked_mul(100).ok_or("Sound reverb size overflow")?,
            false,
        )?;
        let count = self.streams.u32(root.add(24)?)? as usize;
        if let Some(array) = self.data(
            root.add(28)?,
            3,
            count.checked_mul(76).ok_or("Sound duck size overflow")?,
            false,
        )? {
            for index in 0..count {
                for offset in [64, 68] {
                    self.data(array.add(index * 76 + offset)?, 15, 128, false)?;
                }
            }
        }
        for offset in [0x20, 0x24, 0x942, 0x946, 0x1264, 0x1268] {
            self.streams.cstring(root.add(offset)?)?;
        }
        let count = self.streams.u32(root.add(0x1270)?)? as usize;
        self.data(
            root.add(0x1274)?,
            3,
            count.checked_mul(20).ok_or("Sound entry size overflow")?,
            false,
        )?;
        let size = self.streams.u32(root.add(0x1278)?)? as usize;
        self.streams.push(5)?;
        self.data(root.add(0x127c)?, 0x7ff, size, false)?;
        self.streams.pop()?;
        let count = self.streams.u32(root.add(0x1280)?)? as usize;
        self.data(
            root.add(0x1284)?,
            3,
            count
                .checked_mul(8)
                .ok_or("Sound dependency size overflow")?,
            false,
        )?;
        Ok(())
    }

    fn model(&mut self, root: Address) -> Result<()> {
        let bones = self.streams.u8(root.add(4)?)? as usize;
        let roots = self.streams.u8(root.add(5)?)? as usize;
        let surfaces = self.streams.u8(root.add(6)?)? as usize;
        let non_roots = bones
            .checked_sub(roots)
            .ok_or("XModel root count exceeds bone count")?;
        for (offset, mask, length) in [
            (8, 1, bones * 2),
            (12, 0, non_roots),
            (16, 1, non_roots * 8),
            (20, 3, non_roots * 16),
            (24, 0, bones),
            (28, 3, bones * 32),
        ] {
            self.data(root.add(offset)?, mask, length, true)?;
        }
        if let Some(array) = self.data(root.add(32)?, 15, surfaces * 80, false)? {
            for index in 0..surfaces {
                self.surface(array.add(index * 80)?)?;
            }
        }
        if let Some(array) = self.data(root.add(36)?, 3, surfaces * 4, false)? {
            for index in 0..surfaces {
                self.asset(6, array.add(index * 4)?)?;
            }
        }
        let collision_count = self.streams.u32(root.add(156)?)? as usize;
        if let Some(array) = self.data(
            root.add(152)?,
            3,
            collision_count
                .checked_mul(44)
                .ok_or("Model collision count overflow")?,
            false,
        )? {
            for index in 0..collision_count {
                let item = array.add(index * 44)?;
                let count = self.streams.u32(item.add(4)?)? as usize;
                self.data(
                    item,
                    3,
                    count
                        .checked_mul(48)
                        .ok_or("Model collision size overflow")?,
                    false,
                )?;
            }
        }
        self.data(root.add(164)?, 3, bones * 44, false)?;
        self.data(root.add(200)?, 3, surfaces * 4, false)?;
        self.asset(1, root.add(216)?)?;
        let count = self.streams.u8(root.add(220)?)? as usize;
        if let Some(maps) = self.data(root.add(224)?, 3, count * 4, false)? {
            for index in 0..count {
                if let Some(map) = self.data(maps.add(index * 4)?, 3, 12, false)? {
                    let count = self.streams.u32(map)? as usize;
                    if let Some(geometries) = self.data(
                        map.add(4)?,
                        15,
                        count
                            .checked_mul(68)
                            .ok_or("Physics geometry size overflow")?,
                        false,
                    )? {
                        for index in 0..count {
                            if let Some(brush) =
                                self.data(geometries.add(index * 68)?, 15, 96, true)?
                            {
                                let sides = self.streams.u32(brush.add(28)?)? as usize;
                                if let Some(array) = self.data(
                                    brush.add(32)?,
                                    3,
                                    sides.checked_mul(12).ok_or("Brush side size overflow")?,
                                    false,
                                )? {
                                    for index in 0..sides {
                                        self.data(array.add(index * 12)?, 3, 20, true)?;
                                    }
                                }
                                let vertices = self.streams.u32(brush.add(84)?)? as usize;
                                self.data(
                                    brush.add(88)?,
                                    3,
                                    vertices
                                        .checked_mul(12)
                                        .ok_or("Brush vertex size overflow")?,
                                    true,
                                )?;
                                self.data(
                                    brush.add(92)?,
                                    3,
                                    sides.checked_mul(20).ok_or("Brush plane size overflow")?,
                                    true,
                                )?;
                            }
                        }
                    }
                }
            }
        }
        self.asset(2, root.add(228)?)?;
        Ok(())
    }

    fn animation(&mut self, root: Address) -> Result<()> {
        let bones = self.streams.u8(root.add(33)?)? as usize;
        self.data(root.add(64)?, 1, bones * 2, false)?;
        let notifies = self.streams.u8(root.add(34)?)? as usize;
        self.data(root.add(96)?, 3, notifies * 8, false)?;
        let wide = self.streams.u16(root.add(14)?)? >= 256;
        if let Some(delta) = self.data(root.add(100)?, 3, 12, false)? {
            for channel in 0..3 {
                if let Some(track) = self.data(delta.add(channel * 4)?, 3, 4, false)? {
                    let count = self.streams.u16(track)? as usize;
                    let tail = track.add(4)?;
                    if count == 0 {
                        self.streams.load(true, tail, [12, 4, 8][channel])?;
                    } else {
                        let header = if channel == 0 { 28 } else { 4 };
                        self.streams.load(true, tail, header)?;
                        self.streams.load(
                            true,
                            tail.add(header)?,
                            (count + 1) * if wide { 2 } else { 1 },
                        )?;
                        let (field, mask, stride) = if channel == 0 {
                            let small = self.streams.u8(track.add(2)?)? != 0;
                            (
                                tail.add(24)?,
                                if small { 0 } else { 3 },
                                if small { 3 } else { 6 },
                            )
                        } else {
                            (tail, 3, if channel == 1 { 4 } else { 8 })
                        };
                        self.data(field, mask, (count + 1) * stride, false)?;
                    }
                }
            }
        }
        for (pointer, count, stride, mask) in [(68, 4, 1, 0), (72, 6, 2, 1), (76, 8, 4, 3)] {
            let count = self.streams.u16(root.add(count)?)? as usize;
            self.data(root.add(pointer)?, mask, count * stride, false)?;
        }
        let count = self.streams.u32(root.add(40)?)? as usize;
        self.data(
            root.add(80)?,
            1,
            count
                .checked_mul(2)
                .ok_or("Animation random-short size overflow")?,
            false,
        )?;
        for (pointer, count, stride, mask) in [(84, 10, 1, 0), (88, 12, 4, 3)] {
            let count = self.streams.u16(root.add(count)?)? as usize;
            self.data(root.add(pointer)?, mask, count * stride, false)?;
        }
        let count = self.streams.u32(root.add(44)?)? as usize;
        self.data(
            root.add(92)?,
            if wide { 1 } else { 0 },
            count
                .checked_mul(if wide { 2 } else { 1 })
                .ok_or("Animation index size overflow")?,
            false,
        )?;
        Ok(())
    }

    fn surface(&mut self, root: Address) -> Result<()> {
        let info = root.add(16)?;
        let counts = (0..4)
            .map(|index| {
                self.streams
                    .u16(info.add(index * 2)?)
                    .map(|count| count as i16)
            })
            .collect::<Result<Vec<_>>>()?;
        if counts.iter().any(|count| *count < 0) {
            return Err("Negative XSurface blend count".into());
        }
        let blend = counts
            .iter()
            .zip([1, 3, 5, 7])
            .map(|(count, weight)| *count as usize * weight)
            .sum::<usize>();
        self.data(info.add(8)?, 1, blend * 2, true)?;
        self.data(
            info.add(12)?,
            3,
            counts.iter().map(|count| *count as usize).sum::<usize>() * 4,
            true,
        )?;
        if self.streams.u8(root.add(2)?)? & 1 == 0 {
            let count = self.streams.u16(root.add(4)?)? as usize;
            self.data(root.add(32)?, 15, count * 32, true)?;
        }
        let rigid_count = self.streams.u8(root.add(1)?)? as usize;
        if let Some(array) = self.data(root.add(40)?, 3, rigid_count * 12, true)? {
            for index in 0..rigid_count {
                let field = array.add(index * 12 + 8)?;
                if let Some(tree) = self.data(field, 3, 40, true)? {
                    let nodes = self.streams.u32(tree.add(24)?)? as usize;
                    let leaves = self.streams.u32(tree.add(32)?)? as usize;
                    self.data(
                        tree.add(28)?,
                        15,
                        nodes.checked_mul(16).ok_or("Surface tree size overflow")?,
                        false,
                    )?;
                    self.data(
                        tree.add(36)?,
                        1,
                        leaves.checked_mul(2).ok_or("Surface leaf size overflow")?,
                        false,
                    )?;
                }
            }
        }
        let triangles = self.streams.u16(root.add(6)?)? as usize;
        self.data(root.add(12)?, 15, triangles * 6, true)?;
        Ok(())
    }

    fn material(&mut self, root: Address) -> Result<()> {
        self.asset(7, root.add(92)?)?;
        let textures = self.streams.u8(root.add(84)?)? as usize;
        if let Some(array) = self.data(root.add(96)?, 3, textures * 16, true)? {
            for index in 0..textures {
                self.asset(8, array.add(index * 16 + 12)?)?;
            }
        }
        let constants = self.streams.u8(root.add(85)?)? as usize;
        self.data(root.add(100)?, 15, constants * 32, true)?;
        let states = self.streams.u8(root.add(86)?)? as usize;
        self.data(root.add(104)?, 7, states * 20, true)?;
        self.asset(6, root.add(108)?)?;
        Ok(())
    }

    fn effect_visual(&mut self, kind: u8, field: Address) -> Result<()> {
        match kind {
            7 => self.asset(5, field)?,
            9 => self.asset(18, field)?,
            10 | 12 => {
                self.streams.cstring(field)?;
            }
            8 => (),
            _ => self.asset(6, field)?,
        }
        Ok(())
    }

    fn effect(&mut self, root: Address) -> Result<()> {
        let mut count = 0usize;
        for offset in [8, 10, 12] {
            let value = self.streams.u16(root.add(offset)?)? as i16;
            count = count
                .checked_add(usize::try_from(value).map_err(|_| "Negative effect element count")?)
                .ok_or("Effect element count overflow")?;
        }
        if let Some(array) = self.data(
            root.add(28)?,
            3,
            count
                .checked_mul(292)
                .ok_or("Effect element size overflow")?,
            false,
        )? {
            for index in 0..count {
                let element = array.add(index * 292)?;
                let samples = self.streams.u8(element.add(186)?)? as usize;
                self.data(element.add(188)?, 3, (samples * 3 + 3) * 32, false)?;
                let samples = self.streams.u8(element.add(187)?)? as usize;
                self.data(element.add(192)?, 3, (samples * 3 + 3) * 16, false)?;
                let kind = self.streams.u8(element.add(184)?)?;
                let visuals = self.streams.u8(element.add(185)?)? as usize;
                let field = element.add(196)?;
                if kind == 11 {
                    if let Some(array) = self.data(field, 3, visuals * 8, false)? {
                        for index in 0..visuals * 2 {
                            self.asset(6, array.add(index * 4)?)?;
                        }
                    }
                } else if visuals < 2 {
                    self.effect_visual(kind, field)?;
                } else if let Some(array) = self.data(field, 3, visuals * 4, false)? {
                    for index in 0..visuals {
                        self.effect_visual(kind, array.add(index * 4)?)?;
                    }
                }
                for offset in [224, 228, 232, 252] {
                    self.streams.cstring(element.add(offset)?)?;
                }
                let field = element.add(256)?;
                match kind {
                    5 => {
                        if let Some(trail) = self.data(field, 3, 28, false)? {
                            let vertices = self.streams.u32(trail.add(12)?)? as usize;
                            self.data(
                                trail.add(16)?,
                                3,
                                vertices
                                    .checked_mul(20)
                                    .ok_or("Effect trail vertices overflow")?,
                                false,
                            )?;
                            let indices = self.streams.u32(trail.add(20)?)? as usize;
                            self.data(
                                trail.add(24)?,
                                1,
                                indices
                                    .checked_mul(2)
                                    .ok_or("Effect trail indices overflow")?,
                                false,
                            )?;
                        }
                    }
                    9 => {
                        self.data(field, 3, 12, false)?;
                    }
                    _ => {
                        self.data(field, 0, 1, false)?;
                    }
                }
                self.streams.cstring(element.add(280)?)?;
            }
        }
        Ok(())
    }

    fn techniques(&mut self, root: Address) -> Result<()> {
        for index in 0..36 {
            if let Some(technique) = self.data(root.add(8 + index * 4)?, 3, 8, true)? {
                let passes = self.streams.u16(technique.add(6)?)? as usize;
                let pass_array = technique.add(8)?;
                self.streams.load(true, pass_array, passes * 24)?;
                for pass in 0..passes {
                    let pass = pass_array.add(pass * 24)?;
                    self.shader(pass.add(4)?)?;
                    self.data(pass, 3, 116, true)?;
                    self.shader(pass.add(8)?)?;
                    let count = (12..15)
                        .map(|offset| self.streams.u8(pass.add(offset)?).map(usize::from))
                        .collect::<Result<Vec<_>>>()?
                        .into_iter()
                        .sum::<usize>();
                    if let Some(arguments) = self.data(pass.add(20)?, 3, count * 12, false)? {
                        for index in 0..count {
                            let argument = arguments.add(index * 12)?;
                            let kind = self.streams.u16(argument)? as i16;
                            if matches!(kind, 1 | 7) {
                                self.data(argument.add(8)?, 3, 16, true)?;
                            }
                        }
                    }
                }
                self.streams.cstring(technique)?;
            }
        }
        Ok(())
    }

    fn shader(&mut self, field: Address) -> Result<()> {
        if let Some(shader) = self.data(field, 3, 16, true)? {
            self.streams.cstring(shader)?;
            let count = self.streams.u32(shader.add(12)?)? as usize;
            self.data(shader.add(8)?, 0, count, false)?;
        }
        Ok(())
    }

    fn physics_constraints(&mut self, root: Address) -> Result<()> {
        for index in 0..16 {
            let constraint = root.add(8 + index * 168)?;
            self.streams.cstring(constraint.add(20)?)?;
            self.streams.cstring(constraint.add(36)?)?;
            self.asset(6, constraint.add(140)?)?;
        }
        Ok(())
    }

    fn structured_data(&mut self, mut definition: Address) -> Result<()> {
        loop {
            let enum_count = self.streams.u32(definition.add(12)?)? as usize;
            if let Some(array) = self.data(
                definition.add(8)?,
                3,
                enum_count
                    .checked_mul(20)
                    .ok_or("Structured enum size overflow")?,
                false,
            )? {
                for index in 0..enum_count {
                    let item = array.add(index * 20)?;
                    self.streams.cstring(item)?;
                    let count = self.streams.u32(item.add(8)?)? as usize;
                    if let Some(entries) = self.data(
                        item.add(12)?,
                        3,
                        count
                            .checked_mul(44)
                            .ok_or("Structured enum entries overflow")?,
                        false,
                    )? {
                        for index in 0..count {
                            self.streams.cstring(entries.add(index * 44)?)?;
                        }
                    }
                    self.data(
                        item.add(16)?,
                        3,
                        count
                            .checked_mul(8)
                            .ok_or("Structured enum index overflow")?,
                        false,
                    )?;
                }
            }
            let struct_count = self.streams.u32(definition.add(20)?)? as usize;
            if let Some(array) = self.data(
                definition.add(16)?,
                3,
                struct_count
                    .checked_mul(16)
                    .ok_or("Structured type size overflow")?,
                false,
            )? {
                for index in 0..struct_count {
                    let item = array.add(index * 16)?;
                    self.streams.cstring(item)?;
                    let count = self.streams.u32(item.add(4)?)? as usize;
                    if let Some(entries) = self.data(
                        item.add(8)?,
                        3,
                        count
                            .checked_mul(4)
                            .ok_or("Structured field size overflow")?,
                        false,
                    )? {
                        for index in 0..count {
                            self.streams.cstring(entries.add(index * 4)?)?;
                        }
                    }
                    self.data(
                        item.add(12)?,
                        3,
                        count
                            .checked_mul(8)
                            .ok_or("Structured field index overflow")?,
                        false,
                    )?;
                }
            }
            match self.data(definition.add(24)?, 3, 28, false)? {
                Some(next) => definition = next,
                None => break,
            }
        }
        Ok(())
    }
}
