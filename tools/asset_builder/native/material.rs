use super::*;

impl Zone {
    pub(super) fn image(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn material(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn techniques(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn shader(&mut self, field: Address) -> Result<()> {
        if let Some(shader) = self.data(field, 3, 16, true)? {
            self.streams.cstring(shader)?;
            let count = self.streams.u32(shader.add(12)?)? as usize;
            self.data(shader.add(8)?, 0, count, false)?;
        }
        Ok(())
    }

    pub(super) fn camo(&mut self, root: Address) -> Result<()> {
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
}
