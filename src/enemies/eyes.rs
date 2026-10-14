use super::Zombie;
use crate::{
    life::{LifeState, Vitality},
    rendering::NativeArtInputs,
};
use bevy::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum EyeSpawn {
    Ordinary,
    Ground,
    Dug,
}

#[derive(Component, Default)]
#[require(NativeArtInputs)]
pub(super) struct ActorEyes {
    born: Option<i32>,
    rise_started: Option<i32>,
    enabled: bool,
    started: bool,
}

impl ActorEyes {
    fn set(&mut self, enabled: bool, inputs: &mut NativeArtInputs) {
        if enabled == self.enabled {
            return;
        }
        self.enabled = enabled;
        inputs.script_vectors[2] = [0., if enabled { 3. } else { 0.25 }, 0., 0.];
        inputs.script_available |= 1 << 2;
    }

    fn tick(
        &mut self,
        milliseconds: i32,
        spawn: EyeSpawn,
        rising: bool,
        rise_elapsed: f32,
        alive: bool,
        headless: bool,
        panzer: bool,
        inputs: &mut NativeArtInputs,
    ) {
        let born = *self.born.get_or_insert(milliseconds);
        if !alive || headless {
            self.set(false, inputs);
            return;
        }
        if self.started {
            return;
        }
        let ready = if panzer {
            true
        } else {
            match spawn {
                EyeSpawn::Ordinary => milliseconds - born >= 500,
                EyeSpawn::Ground => {
                    if rising && rise_elapsed > 0. {
                        self.rise_started.get_or_insert(milliseconds);
                    }
                    self.rise_started.is_some_and(|start| {
                        let created = i64::from(start) + 600;
                        let poll = i64::from(born)
                            + (created - i64::from(born) + 99).div_euclid(100) * 100;
                        i64::from(milliseconds) >= poll
                    })
                }
                EyeSpawn::Dug => !rising,
            }
        };
        if ready {
            self.started = true;
            self.set(true, inputs);
        }
    }
}

pub(super) fn update(
    clock: Res<crate::rendering::opaque::NativeClock>,
    mut commands: Commands,
    mut removed: RemovedComponents<Zombie>,
    mut former: Query<&mut NativeArtInputs, Without<Zombie>>,
    mut actors: Query<(
        Ref<Zombie>,
        &Vitality,
        &crate::combat_effects::Wounds,
        Has<crate::panzer::Panzer>,
        &mut ActorEyes,
        &mut NativeArtInputs,
    )>,
) {
    for entity in removed.read() {
        if let Ok(mut inputs) = former.get_mut(entity) {
            inputs.script_available &= !(1 << 2);
            inputs.script_vectors[2] = [0.; 4];
            commands.entity(entity).remove::<ActorEyes>();
        }
    }
    for (zombie, vitality, wounds, panzer, mut eyes, mut inputs) in &mut actors {
        if zombie.is_added() {
            *eyes = ActorEyes::default();
            inputs.script_available &= !(1 << 2);
            inputs.script_vectors[2] = [0.; 4];
        }
        let Some(milliseconds) = clock.server_milliseconds() else {
            continue;
        };
        let rising = zombie
            .traversal
            .as_ref()
            .is_some_and(|motion| motion.alias == "rise");
        eyes.tick(
            milliseconds,
            zombie.eye_spawn,
            rising,
            zombie.elapsed,
            vitality.state() == LifeState::ActorAlive,
            wounds.headless,
            panzer,
            &mut inputs,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recycled_actor_clears_only_its_eye_override_and_removes_retired_state() {
        let mut app = App::new();
        app.init_resource::<crate::rendering::opaque::NativeClock>()
            .add_systems(Update, update);
        let actor = app
            .world_mut()
            .spawn((
                Zombie::new(
                    Vec3::ZERO,
                    crate::origins::content::ContentId("enemy.test".into()),
                    crate::rounds::Gait::Walk,
                    1.,
                ),
                Vitality::ordinary_actor(100, 100).unwrap(),
                NativeArtInputs {
                    script_vectors: [[2.; 4]; 4],
                    script_available: 15,
                    ..default()
                },
            ))
            .id();
        app.update();
        let inputs = app.world().get::<NativeArtInputs>(actor).unwrap();
        assert_eq!(inputs.script_available, 11);
        assert_eq!(inputs.script_vectors[2], [0.; 4]);
        assert_eq!(inputs.script_vectors[1], [2.; 4]);
        app.world_mut()
            .get_mut::<NativeArtInputs>(actor)
            .unwrap()
            .script_available = 15;
        app.world_mut().entity_mut(actor).remove::<Zombie>();
        app.update();
        assert!(app.world().get::<ActorEyes>(actor).is_none());
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(actor)
                .unwrap()
                .script_available,
            11
        );
        app.world_mut().entity_mut(actor).insert(Zombie::new(
            Vec3::ZERO,
            crate::origins::content::ContentId("enemy.test".into()),
            crate::rounds::Gait::Walk,
            1.,
        ));
        app.update();
        assert!(app.world().get::<ActorEyes>(actor).is_some());
        assert_eq!(
            app.world()
                .get::<NativeArtInputs>(actor)
                .unwrap()
                .script_available
                & 4,
            0
        );
    }

    #[test]
    fn actor_eye_callbacks_distinguish_initial_zero_delayed_enable_and_later_disable() {
        let mut eyes = ActorEyes::default();
        let mut inputs = NativeArtInputs::default();
        for milliseconds in [1000, 1499] {
            eyes.tick(
                milliseconds,
                EyeSpawn::Ordinary,
                false,
                0.,
                true,
                false,
                false,
                &mut inputs,
            );
            assert_eq!(inputs.script_available, 0);
        }
        eyes.tick(
            1500,
            EyeSpawn::Ordinary,
            false,
            0.,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2], [0., 3., 0., 0.]);
        assert_eq!(inputs.script_available, 4);
        eyes.tick(
            1500,
            EyeSpawn::Ordinary,
            false,
            0.,
            true,
            true,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2], [0., 0.25, 0., 0.]);
        eyes.tick(
            2000,
            EyeSpawn::Ordinary,
            false,
            0.,
            false,
            true,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 0.25);
    }

    #[test]
    fn ground_dug_and_panzer_eye_callbacks_follow_distinct_source_boundaries() {
        let mut ground = ActorEyes::default();
        let mut inputs = NativeArtInputs::default();
        ground.tick(
            0,
            EyeSpawn::Ground,
            true,
            0.,
            true,
            false,
            false,
            &mut inputs,
        );
        ground.tick(
            500,
            EyeSpawn::Ground,
            true,
            0.01,
            true,
            false,
            false,
            &mut inputs,
        );
        ground.tick(
            1099,
            EyeSpawn::Ground,
            true,
            0.6,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_available, 0);
        ground.tick(
            1100,
            EyeSpawn::Ground,
            true,
            0.61,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 3.);
        let mut off_phase = ActorEyes::default();
        inputs = NativeArtInputs::default();
        off_phase.tick(
            0,
            EyeSpawn::Ground,
            true,
            0.,
            true,
            false,
            false,
            &mut inputs,
        );
        off_phase.tick(
            50,
            EyeSpawn::Ground,
            true,
            0.01,
            true,
            false,
            false,
            &mut inputs,
        );
        off_phase.tick(
            650,
            EyeSpawn::Ground,
            true,
            0.61,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_available, 0);
        off_phase.tick(
            700,
            EyeSpawn::Ground,
            true,
            0.66,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 3.);
        let mut dug = ActorEyes::default();
        inputs = NativeArtInputs::default();
        dug.tick(
            2000,
            EyeSpawn::Dug,
            true,
            2.,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_available, 0);
        dug.tick(
            2050,
            EyeSpawn::Dug,
            false,
            0.,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 3.);
        let mut panzer = ActorEyes::default();
        inputs = NativeArtInputs::default();
        panzer.tick(
            0,
            EyeSpawn::Ordinary,
            false,
            0.,
            true,
            false,
            true,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 3.);
        panzer.tick(
            50,
            EyeSpawn::Ordinary,
            false,
            0.,
            false,
            false,
            true,
            &mut inputs,
        );
        assert_eq!(inputs.script_vectors[2][1], 0.25);
        let mut recycled = ActorEyes::default();
        inputs.script_available &= !4;
        recycled.tick(
            50,
            EyeSpawn::Ordinary,
            false,
            0.,
            true,
            false,
            false,
            &mut inputs,
        );
        assert_eq!(inputs.script_available & 4, 0);
    }
}
