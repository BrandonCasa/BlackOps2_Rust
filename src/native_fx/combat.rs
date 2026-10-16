//! Original staff client-field effects, attached to the authored actor joints.
use super::*;
use crate::{
    combat_effects::Wounds,
    enemies::{ActorRig, Zombie},
    presentation::cues::CueParameters,
};

#[derive(Component, Default)]
#[require(CueParameters)]
struct ActorEffects {
    fire: Option<[Entity; 3]>,
    ice: Option<Entity>,
    lightning: Option<Entity>,
    appearance: Option<(Appearance, f32)>,
}

#[derive(Clone, Copy)]
enum Appearance {
    Burn(f32),
    Freeze,
    Thaw,
}
impl Appearance {
    fn sample(self, age: f32) -> Option<[f32; 4]> {
        // The original client scripts update scriptVector3 in 50 ms steps.
        let poll = ((age + 0.000001) / 0.05) as u32;
        let (amount, frozen) = match self {
            Self::Burn(rate) => {
                let index = poll.checked_sub(1)?;
                let steps = (0.25 / rate).floor() as u32 + 1;
                let amount = if index < steps * 4 {
                    if index / steps % 2 == 0 {
                        0.6 + (index % steps) as f32 * rate
                    } else {
                        0.85 - (index % steps) as f32 * rate
                    }
                } else {
                    0.6 + (index - steps * 4).min((0.4 / rate).floor() as u32) as f32 * rate
                };
                (amount, 0.)
            }
            Self::Freeze => (0.6 + poll.min(30) as f32 * 0.01, 1.),
            Self::Thaw if poll < 9 => (1. - poll as f32 * 0.05, 1.),
            Self::Thaw => (0., 0.),
        };
        Some([amount, frozen, 0., 0.])
    }
}
impl ActorEffects {
    fn stop(&mut self, commands: &mut Commands) {
        if let Some([torso, head, sound]) = self.fire.take() {
            stop(commands, torso);
            stop(commands, head);
            commands.entity(sound).try_despawn();
        }
        for effect in [self.ice.take(), self.lightning.take()]
            .into_iter()
            .flatten()
        {
            stop(commands, effect);
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.register_required_components::<Zombie, ActorEffects>()
        .add_observer(retire)
        .add_systems(
            Update,
            update
                .before(super::spawn)
                .run_if(in_state(GameState::Playing)),
        );
}

fn retire(
    removed: On<Remove<Zombie>>,
    mut commands: Commands,
    mut actors: Query<(&mut ActorEffects, &mut CueParameters)>,
) {
    if let Ok((mut effects, mut inputs)) = actors.get_mut(removed.entity) {
        effects.stop(&mut commands);
        inputs.script_available &= !(1 << 3);
        inputs.script_vectors[3] = [0.; 4];
        commands.entity(removed.entity).try_remove::<ActorEffects>();
    }
}

fn update(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut session: ResMut<crate::gameplay::Session>,
    mut actors: Query<
        (Entity, &Wounds, &mut ActorEffects, &mut CueParameters),
        Without<crate::panzer::Panzer>,
    >,
    rigs: Query<&ActorRig>,
) {
    for (owner, wounds, mut effects, mut inputs) in &mut actors {
        if let Some((_, age)) = &mut effects.appearance {
            *age += time.delta_secs();
        }
        let joint = |name| rigs.iter().find_map(|rig| rig.socket(owner, name));
        if wounds.burning > 0. && effects.fire.is_none() {
            if let (Some(torso), Some(head)) = (joint("j_spinelower"), joint("j_head")) {
                let torso = attach(
                    &mut commands,
                    "env/fire/fx_fire_zombie_torso",
                    torso,
                    Transform::default(),
                );
                let head = attach(
                    &mut commands,
                    "env/fire/fx_fire_zombie_md",
                    head,
                    Transform::default(),
                );
                let sound = crate::audio::emit_loop(
                    &mut commands,
                    "zmb_fire_loop",
                    crate::audio::SoundSource::attached(owner),
                );
                commands.entity(sound).insert((
                    ChildOf(owner),
                    crate::audio::SoundControl {
                        volume: 0.5,
                        ..default()
                    },
                ));
                effects.fire = Some([torso, head, sound]);
                let rate = 0.01 + 0.005 * (session.random() as f32 / u32::MAX as f32);
                effects.appearance = Some((Appearance::Burn(rate), 0.));
            }
        } else if wounds.burning <= 0.
            && let Some([torso, head, sound]) = effects.fire.take()
        {
            stop(&mut commands, torso);
            stop(&mut commands, head);
            commands.entity(sound).try_despawn();
        }
        if wounds.frozen > 0. && effects.ice.is_none() {
            if let Some(spine) = joint("j_spine4") {
                effects.ice = Some(attach(
                    &mut commands,
                    "weapon/zmb_staff/fx_zmb_staff_ice_trail_bolt",
                    spine,
                    Transform::default(),
                ));
                crate::audio::emit(
                    &mut commands,
                    "wpn_waterstaff_freeze_zombie",
                    crate::audio::SoundSource::attached(owner),
                );
                effects.appearance = Some((Appearance::Freeze, 0.));
            }
        } else if wounds.frozen <= 0.
            && let Some(effect) = effects.ice.take()
        {
            stop(&mut commands, effect);
            effects.appearance = Some((Appearance::Thaw, 0.));
        }
        if wounds.electrical > 0. && effects.lightning.is_none() {
            if let Some(spine) = joint("j_spineupper") {
                effects.lightning = Some(attach(
                    &mut commands,
                    "weapon/zmb_staff/fx_zmb_staff_elec_ug_impact_hit_torso",
                    spine,
                    Transform::default(),
                ));
                crate::audio::emit(
                    &mut commands,
                    "wpn_imp_lightningstaff",
                    crate::audio::SoundSource::attached(owner),
                );
            }
        } else if wounds.electrical <= 0.
            && let Some(effect) = effects.lightning.take()
        {
            stop(&mut commands, effect);
        }
        if let Some((appearance, age)) = effects.appearance
            && let Some(value) = appearance.sample(age)
            && (inputs.script_vectors[3] != value || inputs.script_available & (1 << 3) == 0)
        {
            inputs.script_vectors[3] = value;
            inputs.script_available |= 1 << 3;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_staff_materials_keep_script_polling_and_distinct_char_and_frost_values() {
        assert_eq!(Appearance::Burn(0.01).sample(0.049), None);
        assert_eq!(Appearance::Burn(0.01).sample(0.05), Some([0.6, 0., 0., 0.]));
        assert_eq!(
            Appearance::Burn(0.01).sample(0.099),
            Appearance::Burn(0.01).sample(0.05)
        );
        let charred = Appearance::Burn(0.012).sample(100.).unwrap();
        assert!(charred[0] <= 1. && charred[0] > 0.98);
        assert_eq!(charred[1], 0.);
        assert_eq!(Appearance::Freeze.sample(0.), Some([0.6, 1., 0., 0.]));
        assert_eq!(Appearance::Freeze.sample(2.), Some([0.9, 1., 0., 0.]));
        assert_eq!(Appearance::Thaw.sample(0.4), Some([0.6, 1., 0., 0.]));
        assert_eq!(Appearance::Thaw.sample(0.45), Some([0.; 4]));
    }
}
