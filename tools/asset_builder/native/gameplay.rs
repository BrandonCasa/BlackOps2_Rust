use super::*;

impl Zone {
    pub(super) fn vehicle(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn siege_instance(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn destructible(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn attachment(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn weapon(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn weapon_definition(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn structured_data(&mut self, mut definition: Address) -> Result<()> {
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
