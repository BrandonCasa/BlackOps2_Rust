use super::*;

impl Zone {
    pub(super) fn expression(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn menu(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn menu_handlers(&mut self, mut field: Address, key: bool) -> Result<()> {
        while let Some(handler) = self.data(field, 3, 12, false)? {
            if !key {
                self.streams.cstring(handler)?;
            }
            self.menu_scripts(handler.add(4)?)?;
            field = handler.add(8)?;
        }
        Ok(())
    }

    pub(super) fn menu_scripts(&mut self, mut field: Address) -> Result<()> {
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

    pub(super) fn menu_item(&mut self, root: Address) -> Result<()> {
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

    pub(super) fn menu_focus(&mut self, field: Address, kind: u32) -> Result<()> {
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
}
