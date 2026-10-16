use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{Session, SessionEntity},
    origins::ContentId,
    presentation::cues::CueParameters,
};
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct CrystalProfile {
    initial_state: [f32; 4],
    sample_seconds: f32,
    zombie_controlled: FlickerProfile,
    player_controlled: FlickerProfile,
    crystals: Vec<CrystalDefinition>,
}

#[derive(Clone, Deserialize)]
struct FlickerProfile {
    range: [f32; 2],
    transition_seconds: [f32; 2],
    hold_seconds: [f32; 2],
}

#[derive(Clone, Deserialize)]
struct CrystalDefinition {
    generator: ContentId,
    target: String,
    presentation: ContentId,
    source_model: String,
    position: [f32; 3],
    yaw: f32,
}

#[derive(Component)]
struct CrystalGlow {
    index: usize,
    controlled: bool,
    value: f32,
    seed: u64,
    sample: u64,
    remaining_steps: u32,
    increment: f32,
    holding: bool,
    wait: f64,
    previous: ([f32; 4], bool),
}

impl CrystalGlow {
    fn new(index: usize, seed: u64, profile: &CrystalProfile, previous: ([f32; 4], bool)) -> Self {
        let mut glow = Self {
            index,
            controlled: false,
            value: profile.initial_state[0],
            seed,
            sample: 0,
            remaining_steps: 0,
            increment: 0.,
            holding: false,
            wait: 0.,
            previous,
        };
        glow.transition(profile);
        glow
    }

    fn random(&mut self, range: [f32; 2]) -> f32 {
        let value = crate::weapons::sample(self.seed.wrapping_add(self.sample));
        self.sample = self.sample.wrapping_add(1);
        range[0] + (range[1] - range[0]) * value
    }

    fn transition(&mut self, profile: &CrystalProfile) {
        let phase = if self.controlled {
            &profile.player_controlled
        } else {
            &profile.zombie_controlled
        };
        let target = self.random(phase.range);
        let seconds = self.random(phase.transition_seconds);
        self.remaining_steps = (seconds / profile.sample_seconds) as u32;
        self.increment = (target - self.value) / self.remaining_steps as f32;
        self.holding = false;
        self.wait = 0.;
    }

    fn advance(&mut self, delta: f64, controlled: bool, profile: &CrystalProfile) {
        if self.controlled != controlled {
            self.controlled = controlled;
            self.transition(profile);
        }
        let mut remaining = delta;
        while remaining >= self.wait {
            remaining -= self.wait;
            if self.remaining_steps > 0 {
                self.value += self.increment;
                self.remaining_steps -= 1;
                self.wait = f64::from(profile.sample_seconds);
            } else if self.holding {
                self.transition(profile);
            } else {
                let phase = if self.controlled {
                    &profile.player_controlled
                } else {
                    &profile.zombie_controlled
                };
                self.wait = f64::from(self.random(phase.hold_seconds));
                self.holding = true;
            }
        }
        self.wait -= remaining;
    }
}

pub(super) fn reset(world: &mut World) {
    let existing = world
        .query::<(Entity, &CrystalGlow)>()
        .iter(world)
        .map(|(entity, glow)| (entity, glow.previous))
        .collect::<Vec<_>>();
    for (entity, (vector, available)) in existing {
        if let Some(mut inputs) = world.get_mut::<CueParameters>(entity) {
            inputs.luminous_channel = Some((0, 0));
            inputs.script_vectors[0] = vector;
            inputs.script_available = (inputs.script_available & !1) | u8::from(available);
        }
        world.entity_mut(entity).remove::<CrystalGlow>();
    }
}

pub(super) struct CrystalPlugin;
impl Plugin for CrystalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::StartingSession),
            assemble.in_set(crate::session::SessionSetup::World),
        )
        .add_systems(Update, animate.run_if(in_state(GameState::Playing)));
    }
}

fn assemble(
    mut commands: Commands,
    game: Res<GameDefinition>,
    session: Res<Session>,
    server: Res<AssetServer>,
    existing: Query<(Entity, &Name, &GlobalTransform), With<crate::assets::StaticPropRoot>>,
    mut inputs: Query<&mut CueParameters>,
) {
    let Some(profile) = &game.generator_crystals else {
        return;
    };
    for (index, crystal) in profile.crystals.iter().enumerate() {
        let candidates = existing
            .iter()
            .filter_map(|(entity, name, pose)| {
                (name.as_str().eq_ignore_ascii_case(&crystal.source_model)
                    && pose
                        .translation()
                        .distance_squared(Vec3::from_array(crystal.position))
                        < 0.0001)
                    .then_some(entity)
            })
            .collect::<Vec<_>>();
        if candidates.len() > 1 {
            panic!(
                "Source crystal {} resolves multiple existing scene owners",
                crystal.target
            );
        }
        let entity = if let Some(&entity) = candidates.first() {
            entity
        } else {
            let Some(scene) = game
                .content
                .presentations
                .iter()
                .find(|presentation| presentation.id == crystal.presentation)
                .and_then(|presentation| presentation.scene.as_ref())
            else {
                panic!(
                    "Source crystal {} presentation is unavailable",
                    crystal.target
                );
            };
            commands
                .spawn((
                    SessionEntity,
                    WorldAssetRoot(server.load(scene.clone())),
                    Transform::from_translation(Vec3::from_array(crystal.position))
                        .with_rotation(Quat::from_rotation_y(crystal.yaw)),
                    Visibility::Inherited,
                    Name::new(crystal.target.clone()),
                ))
                .id()
        };
        let previous = if let Ok(mut inputs) = inputs.get_mut(entity) {
            let previous = (inputs.script_vectors[0], inputs.script_available & 1 != 0);
            inputs.luminous_channel = Some((0, 0));
            inputs.script_vectors[0] = profile.initial_state;
            inputs.script_available |= 1;
            previous
        } else {
            commands.entity(entity).insert(CueParameters {
                script_vectors: [profile.initial_state, [0.; 4], [0.; 4], [0.; 4]],
                script_available: 1,
                ..default()
            });
            ([0.; 4], false)
        };
        commands.entity(entity).insert(CrystalGlow::new(
            index,
            session.seed ^ entity.to_bits(),
            profile,
            previous,
        ));
    }
}

fn animate(
    time: Res<Time>,
    game: Res<GameDefinition>,
    generators: Res<super::Generators>,
    mut crystals: Query<(&mut CrystalGlow, &mut CueParameters)>,
) {
    let Some(profile) = &game.generator_crystals else {
        return;
    };
    for (mut glow, mut inputs) in &mut crystals {
        let Some(crystal) = profile.crystals.get(glow.index) else {
            continue;
        };
        glow.advance(
            time.delta_secs_f64(),
            generators.powered(&crystal.generator),
            profile,
        );
        inputs.luminous_channel = Some((0, 0));
        inputs.script_vectors[0] = [
            glow.value,
            profile.initial_state[1],
            profile.initial_state[2],
            profile.initial_state[3],
        ];
        inputs.script_available |= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> CrystalProfile {
        CrystalProfile {
            initial_state: [0.65, 0., 0., 0.],
            sample_seconds: 0.0167,
            zombie_controlled: FlickerProfile {
                range: [0.65, 0.85],
                transition_seconds: [2., 4.],
                hold_seconds: [0.5, 3.],
            },
            player_controlled: FlickerProfile {
                range: [0., 0.2],
                transition_seconds: [0.7, 1.],
                hold_seconds: [0.05, 0.35],
            },
            crystals: Vec::new(),
        }
    }

    #[test]
    fn capture_switches_crystal_range_without_resetting_the_current_glow() {
        let profile = profile();
        let mut glow = CrystalGlow::new(0, 7, &profile, ([0.; 4], false));
        for _ in 0..600 {
            glow.advance(0.0167, false, &profile);
            assert!((0.64999..=0.85001).contains(&glow.value));
        }
        let previous = glow.value;
        glow.advance(0., true, &profile);
        assert!(glow.value < previous && glow.value > 0.6);
        for _ in 0..600 {
            glow.advance(0.0167, true, &profile);
        }
        assert!((-0.00001..=0.20001).contains(&glow.value));
    }

    #[test]
    fn session_cleanup_restores_existing_scene_input_without_removing_other_vectors() {
        let profile = profile();
        let mut world = World::new();
        let owner = world
            .spawn((
                CrystalGlow::new(0, 7, &profile, ([0.2, 0.3, 0.4, 0.5], false)),
                CueParameters {
                    script_vectors: [profile.initial_state, [0.7; 4], [0.; 4], [0.; 4]],
                    script_available: 3,
                    ..default()
                },
            ))
            .id();
        reset(&mut world);
        assert!(world.get::<CrystalGlow>(owner).is_none());
        let inputs = world.get::<CueParameters>(owner).unwrap();
        assert_eq!(inputs.script_vectors[0], [0.2, 0.3, 0.4, 0.5]);
        assert_eq!(inputs.script_vectors[1], [0.7; 4]);
        assert_eq!(inputs.script_available, 2);
    }
}
