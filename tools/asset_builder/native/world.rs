use super::*;

impl Zone {
    pub(super) fn world(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn world_cell(&mut self, cell: Address) -> Result<()> {
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

    pub(super) fn world_draw(&mut self, draw: Address) -> Result<()> {
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

    pub(super) fn world_light_grid(&mut self, grid: Address) -> Result<()> {
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

    pub(super) fn world_static(&mut self, world: Address) -> Result<()> {
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
}
