use super::*;

impl Zone {
    pub(super) fn sound_bank(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn effect_visual(&mut self, kind: u8, field: Address) -> Result<()> {
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

    pub(super) fn effect(&mut self, root: Address) -> Result<()> {
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
}
