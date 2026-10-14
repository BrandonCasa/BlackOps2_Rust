use super::*;

impl Zone {
    pub(super) fn clip_map(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn collision_pointer(&mut self, field: Address) -> Result<()> {
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

    pub(super) fn collision_data(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn collision_brush(&mut self, root: Address) -> Result<()> {
        if let Some(side) = self.data(root.add(32)?, 3, 12, true)? {
            self.data(side, 3, 20, true)?;
        }
        self.data(root.add(88)?, 3, 12, true)?;
        Ok(())
    }

    pub(super) fn map_triggers(&mut self, root: Address) -> Result<()> {
        for (field, count, stride) in [(4, 0, 8), (12, 8, 32), (20, 16, 20)] {
            self.counted_data(root, field, count, stride, 3, true)?;
        }
        Ok(())
    }

    pub(super) fn path_data(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn path_tree(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn physics_constraints(&mut self, root: Address) -> Result<()> {
        for index in 0..16 {
            let constraint = root.add(8 + index * 168)?;
            self.streams.cstring(constraint.add(20)?)?;
            self.streams.cstring(constraint.add(36)?)?;
            self.asset(6, constraint.add(140)?)?;
        }
        Ok(())
    }
}
