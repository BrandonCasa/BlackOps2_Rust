use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Forward,
    Backward,
    Left,
    Right,
    Sprint,
    Crouch,
    Prone,
    Jump,
    Fire,
    Aim,
    Melee,
    Reload,
    Interact,
    CycleWeapon,
    WeaponOne,
    WeaponTwo,
    WeaponThree,
    Lethal,
    Tactical,
    Mine,
    Scoreboard,
    Pause,
    Restart,
    Buildable,
    StaffSecondary,
}
impl Action {
    pub const ALL: [Self; 25] = [
        Self::Forward,
        Self::Backward,
        Self::Left,
        Self::Right,
        Self::Sprint,
        Self::Crouch,
        Self::Prone,
        Self::Jump,
        Self::Fire,
        Self::Aim,
        Self::Melee,
        Self::Reload,
        Self::Interact,
        Self::CycleWeapon,
        Self::WeaponOne,
        Self::WeaponTwo,
        Self::WeaponThree,
        Self::Lethal,
        Self::Tactical,
        Self::Mine,
        Self::Scoreboard,
        Self::Pause,
        Self::Restart,
        Self::Buildable,
        Self::StaffSecondary,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Forward => "Move forward",
            Self::Backward => "Move backward",
            Self::Left => "Move left",
            Self::Right => "Move right",
            Self::Sprint => "Sprint",
            Self::Crouch => "Crouch",
            Self::Prone => "Prone / dive",
            Self::Jump => "Jump",
            Self::Fire => "Fire",
            Self::Aim => "Aim down sights",
            Self::Melee => "Melee",
            Self::Reload => "Reload",
            Self::Interact => "Interact",
            Self::CycleWeapon => "Cycle weapon",
            Self::WeaponOne => "Weapon slot 1",
            Self::WeaponTwo => "Weapon slot 2",
            Self::WeaponThree => "Weapon slot 3",
            Self::Lethal => "Lethal equipment",
            Self::Tactical => "Tactical equipment",
            Self::Mine => "Claymore",
            Self::Scoreboard => "Scoreboard",
            Self::Pause => "Pause",
            Self::Restart => "Restart after game over",
            Self::Buildable => "Buildable equipment",
            Self::StaffSecondary => "Staff secondary mode",
        }
    }
    pub fn accepts_wheel(self) -> bool {
        matches!(
            self,
            Self::Jump
                | Self::Prone
                | Self::Melee
                | Self::Reload
                | Self::CycleWeapon
                | Self::WeaponOne
                | Self::WeaponTwo
                | Self::WeaponThree
        )
    }
    pub fn gameplay(self) -> bool {
        !matches!(self, Self::Pause | Self::Restart)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Binding {
    Key(KeyCode),
    Mouse(MouseButton),
    WheelUp,
    WheelDown,
}
impl Binding {
    pub fn label(self) -> String {
        match self {
            Self::Key(key) => match key {
                KeyCode::ShiftLeft => "Left Shift".into(),
                KeyCode::ShiftRight => "Right Shift".into(),
                KeyCode::ControlLeft => "Left Ctrl".into(),
                KeyCode::ControlRight => "Right Ctrl".into(),
                KeyCode::AltLeft => "Left Alt".into(),
                KeyCode::AltRight => "Right Alt".into(),
                _ => format!("{key:?}")
                    .trim_start_matches("Key")
                    .trim_start_matches("Digit")
                    .to_owned(),
            },
            Self::Mouse(button) => match button {
                MouseButton::Left => "Left mouse".into(),
                MouseButton::Right => "Right mouse".into(),
                MouseButton::Middle => "Middle mouse".into(),
                MouseButton::Back => "Mouse back".into(),
                MouseButton::Forward => "Mouse forward".into(),
                MouseButton::Other(n) => format!("Mouse {n}"),
            },
            Self::WheelUp => "Wheel up".into(),
            Self::WheelDown => "Wheel down".into(),
        }
    }
    pub fn held(self, keys: &ButtonInput<KeyCode>, mouse: &ButtonInput<MouseButton>) -> bool {
        match self {
            Self::Key(key) => keys.pressed(key),
            Self::Mouse(button) => mouse.pressed(button),
            _ => false,
        }
    }
    pub fn just_pressed(
        self,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Self::Key(key) => keys.just_pressed(key),
            Self::Mouse(button) => mouse.just_pressed(button),
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PressMode {
    #[default]
    Hold,
    Toggle,
}

#[derive(Resource, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ControlSettings {
    pub version: u32,
    pub bindings: BTreeMap<Action, Vec<Binding>>,
    pub sensitivity: f32,
    pub ads_sensitivity: f32,
    pub invert_mouse_y: bool,
    pub aim_mode: PressMode,
    pub crouch_mode: PressMode,
    pub sprint_mode: PressMode,
    pub audio: crate::audio::AudioSettings,
}
impl Default for ControlSettings {
    fn default() -> Self {
        use Action::*;
        let bindings = [
            (Forward, KeyCode::KeyW),
            (Backward, KeyCode::KeyS),
            (Left, KeyCode::KeyA),
            (Right, KeyCode::KeyD),
            (Sprint, KeyCode::ShiftLeft),
            (Crouch, KeyCode::ControlLeft),
            (Prone, KeyCode::KeyZ),
            (Jump, KeyCode::Space),
            (Melee, KeyCode::KeyV),
            (Reload, KeyCode::KeyR),
            (Interact, KeyCode::KeyF),
            (CycleWeapon, KeyCode::KeyQ),
            (WeaponOne, KeyCode::Digit1),
            (WeaponTwo, KeyCode::Digit2),
            (WeaponThree, KeyCode::Digit3),
            (Lethal, KeyCode::KeyG),
            (Tactical, KeyCode::KeyT),
            (Mine, KeyCode::Digit4),
            (Scoreboard, KeyCode::Tab),
            (Pause, KeyCode::Escape),
            (Restart, KeyCode::KeyR),
            (Buildable, KeyCode::Digit5),
            (StaffSecondary, KeyCode::KeyX),
        ]
        .into_iter()
        .map(|(action, key)| (action, vec![Binding::Key(key)]))
        .chain([
            (Fire, vec![Binding::Mouse(MouseButton::Left)]),
            (Aim, vec![Binding::Mouse(MouseButton::Right)]),
        ])
        .collect::<BTreeMap<_, _>>();
        let mut result = Self {
            version: 1,
            bindings,
            sensitivity: 1.,
            ads_sensitivity: 1.,
            invert_mouse_y: false,
            aim_mode: PressMode::Hold,
            crouch_mode: PressMode::Hold,
            sprint_mode: PressMode::Hold,
            audio: crate::audio::AudioSettings::default(),
        };
        result
            .bindings
            .get_mut(&CycleWeapon)
            .unwrap()
            .extend([Binding::WheelUp, Binding::WheelDown]);
        result
    }
}
impl ControlSettings {
    pub fn valid(&self) -> bool {
        self.version == 1
            && [
                self.audio.master,
                self.audio.effects,
                self.audio.music,
                self.audio.dialogue,
            ]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            && [self.sensitivity, self.ads_sensitivity]
                .iter()
                .all(|s| s.is_finite() && (0.1..=10.).contains(s))
            && self.bindings.len() == Action::ALL.len()
            && Action::ALL.iter().all(|a| {
                self.bindings.get(a).is_some_and(|bindings| {
                    bindings.len() <= 4
                        && bindings.iter().enumerate().all(|(i, b)| {
                            !bindings[..i].contains(b)
                                && self.binding_allowed(*a, *b)
                                && (!a.gameplay()
                                    || !self.bindings.iter().any(|(other, bs)| {
                                        other != a && other.gameplay() && bs.contains(b)
                                    }))
                        })
                })
            })
    }
    fn binding_allowed(&self, action: Action, binding: Binding) -> bool {
        match binding {
            Binding::WheelUp | Binding::WheelDown => action.accepts_wheel(),
            Binding::Key(KeyCode::Escape) => action == Action::Pause,
            Binding::Key(
                KeyCode::F6
                | KeyCode::F7
                | KeyCode::F8
                | KeyCode::F9
                | KeyCode::F10
                | KeyCode::F11
                | KeyCode::F12
                | KeyCode::Unidentified(_),
            ) => false,
            _ => true,
        }
    }
    pub fn rebind(&mut self, action: Action, binding: Option<Binding>) -> Result<(), String> {
        if let Some(binding) = binding {
            if !self.binding_allowed(action, binding) {
                return Err("That input is reserved or needs a held button.".into());
            }
            if action.gameplay()
                && let Some((other, _)) = self.bindings.iter().find(|(other, bs)| {
                    **other != action && other.gameplay() && bs.contains(&binding)
                })
            {
                return Err(format!(
                    "{} is already assigned to {}.",
                    binding.label(),
                    other.label()
                ));
            }
        }
        self.bindings.insert(action, binding.into_iter().collect());
        Ok(())
    }
    pub fn label(&self, action: Action) -> String {
        let labels = self.bindings[&action]
            .iter()
            .map(|b| b.label())
            .collect::<Vec<_>>();
        if labels.is_empty() {
            "Unbound".into()
        } else {
            labels.join(" / ")
        }
    }
    pub fn just_pressed(
        &self,
        action: Action,
        keys: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        self.bindings[&action]
            .iter()
            .any(|b| b.just_pressed(keys, mouse))
    }
    pub fn mode(&self, action: Action) -> PressMode {
        match action {
            Action::Aim => self.aim_mode,
            Action::Crouch => self.crouch_mode,
            Action::Sprint => self.sprint_mode,
            _ => PressMode::Hold,
        }
    }
}
