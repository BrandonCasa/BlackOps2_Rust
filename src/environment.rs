use crate::{GameState, assets::GameDefinition, gameplay::GameStep, player::Player};
use avian3d::prelude::*;
use bevy::prelude::*;
pub mod brushes;
pub mod digging;
pub mod hazards;
pub mod parts;
pub mod support;
pub mod triggers;
pub mod weather;
#[derive(Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct EnvironmentDefinition {
    pub mud: Option<MudDefinition>,
    pub triggers: Vec<triggers::TriggerDefinition>,
    pub brush_entities: Vec<brushes::BrushDefinition>,
    pub ownership: Vec<support::OwnershipDefinition>,
    pub digging: Option<digging::DigDefinition>,
    pub parts: Vec<parts::PartDefinition>,
    pub crafting_benches: Vec<crate::crafting::BenchDefinition>,
}
#[derive(Clone, serde::Deserialize)]
pub struct MudDefinition {
    pub scale: f32,
    pub stamin_scale: f32,
    pub step: f32,
    pub stamin_step: f32,
    pub interval: f32,
    pub volumes: Vec<VolumeDefinition>,
}
#[derive(Clone, serde::Deserialize)]
pub struct VolumeDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
    #[serde(default)]
    pub contents: u32,
    #[serde(default)]
    pub surface_flags: Vec<u32>,
}
pub fn solids(excluded: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(LayerMask::DEFAULT).with_excluded_entities(excluded)
}
pub const PLAYER_INGRESS_LAYER: u32 = 4;
pub const MONSTER_CLIP_LAYER: u32 = 64;
pub fn actor_solids(excluded: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(1u32 | MONSTER_CLIP_LAYER).with_excluded_entities(excluded)
}
pub fn player_solids(excluded: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(1u32 | PLAYER_INGRESS_LAYER).with_excluded_entities(excluded)
}
#[derive(Component)]
pub struct MudVolume;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootstepSurface {
    Solid,
    Mud,
}
#[derive(Message)]
pub struct Footstep {
    pub owner: Entity,
    pub surface: FootstepSurface,
    pub position: Vec3,
}
#[derive(Component)]
pub struct TerrainMotion {
    pub muddy: bool,
    pub scale: f32,
    pub clock: f32,
}
impl Default for TerrainMotion {
    fn default() -> Self {
        Self {
            muddy: false,
            scale: 1.,
            clock: 0.,
        }
    }
}
pub struct EnvironmentPlugin;
impl Plugin for EnvironmentPlugin {
    fn build(&self, app: &mut App) {
        triggers::register(app);
        app.add_message::<Footstep>()
            .add_plugins(support::SupportPlugin)
            .init_resource::<parts::Parts>()
            .register_required_components::<Player, hazards::BoundaryMonitor>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                (triggers::spawn, spawn, hazards::spawn)
                    .in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (hazards::boundaries, hazards::damage)
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                feedback
                    .in_set(GameStep::Movement)
                    .after(crate::player::move_player)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                slow.in_set(GameStep::Movement)
                    .before(crate::player::move_player)
                    .run_if(in_state(GameState::Playing)),
            );
        app.add_plugins((weather::WeatherPlugin, digging::DiggingPlugin));
    }
}
fn spawn(mut commands: Commands, game: Res<GameDefinition>, existing: Query<(), With<MudVolume>>) {
    if !existing.is_empty() {
        return;
    }
    if let Some(mud) = &game.environment.mud {
        for volume in &mud.volumes {
            if let Some(shape) = Collider::convex_hull(
                &volume
                    .vertices
                    .iter()
                    .copied()
                    .map(Vec3::from_array)
                    .collect::<Vec<_>>(),
            ) {
                commands.spawn((
                    MudVolume,
                    crate::gameplay::SessionEntity,
                    shape,
                    Sensor,
                    CollisionLayers::new(2u32, LayerMask::NONE),
                    Transform::from_translation(Vec3::from_array(volume.position)),
                    Name::new(volume.id.clone()),
                ));
            }
        }
    }
}
pub(crate) fn slow(
    game: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    query: SpatialQuery,
    volumes: Query<(), With<MudVolume>>,
    mut players: Query<
        (
            &Transform,
            &Collider,
            &crate::solo_revive::PerkInventory,
            &crate::life::Vitality,
            &mut TerrainMotion,
        ),
        With<Player>,
    >,
) {
    let Some(mud) = &game.environment.mud else {
        return;
    };
    for (pose, shape, owned, health, mut motion) in &mut players {
        let touching = query.shape_intersections(
            shape,
            pose.translation,
            Quat::IDENTITY,
            &SpatialQueryFilter::from_mask(2u32),
        );
        let muddy = touching.iter().any(|entity| volumes.contains(*entity))
            && matches!(
                health.state(),
                crate::life::LifeState::Standing | crate::life::LifeState::Downed
            );
        motion.muddy = muddy;
        motion.clock += time.delta_secs();
        while motion.clock + 1e-6 >= mud.interval {
            motion.clock = (motion.clock - mud.interval).max(0.);
            let stamin = game
                .content
                .perks
                .iter()
                .any(|p| p.sprint.is_some() && owned.owned.contains(&p.id));
            let (target, step) = if !muddy {
                (1., 1.)
            } else if stamin {
                (mud.stamin_scale, mud.stamin_step)
            } else {
                (mud.scale, mud.step)
            };
            if motion.scale != target {
                motion.scale = if motion.scale > target + step {
                    motion.scale - step
                } else {
                    target
                };
            }
        }
    }
}

fn feedback(
    game: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &crate::life::Vitality,
        &TerrainMotion,
        &crate::movement::SprintState,
        &crate::movement::Mobility,
        &crate::weapons::WeaponInventory,
        &mut crate::movement::MotionFeedback,
    )>,
    mut steps: MessageWriter<Footstep>,
) {
    for (owner, player, pose, health, terrain, sprint, mobility, inventory, mut motion) in
        &mut players
    {
        motion.previous_bob = motion.bob;
        let distance = motion
            .position
            .map_or(0., |p| p.xz().distance(pose.translation.xz()));
        motion.position = Some(pose.translation);
        let moving = player.grounded
            && distance > 0.0001
            && distance < 0.5
            && !mobility.busy()
            && health.state() == crate::life::LifeState::Standing
            && !player.prone;
        let tuning = &game.tuning.mobility;
        let mut target = Vec2::ZERO;
        if moving {
            let stride = if sprint.sprinting {
                tuning.stride[1]
            } else {
                tuning.stride[0]
            };
            let before = (motion.phase / std::f32::consts::PI).floor();
            motion.phase += distance / stride * std::f32::consts::TAU;
            if (motion.phase / std::f32::consts::PI).floor() != before {
                let surface = if terrain.muddy {
                    FootstepSurface::Mud
                } else {
                    FootstepSurface::Solid
                };
                steps.write(Footstep {
                    owner,
                    surface,
                    position: pose.translation - Vec3::Y * player.height * 0.5,
                });
            }
            motion.phase %= std::f32::consts::TAU * 2.;
            let amplitude = Vec2::from_array(if sprint.sprinting {
                tuning.sprint_bob
            } else {
                tuning.walk_bob
            });
            let scale = if player.crouched { 0.5 } else { 1. } * (1. - inventory.ads * 0.85);
            target = Vec2::new((motion.phase * 0.5).sin(), motion.phase.cos()) * amplitude * scale;
        }
        motion.bob = motion.bob.lerp(target, (12. * time.delta_secs()).min(1.));
    }
}
