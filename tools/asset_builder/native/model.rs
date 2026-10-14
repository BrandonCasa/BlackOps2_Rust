use super::*;

impl Zone {
    pub(super) fn model(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn animation(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn surface(&mut self, root: Address) -> Result<()> {
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
}
