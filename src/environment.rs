//! Authored query volumes extend Avian; source profiles supply map-specific rules.
use crate::{GameState, assets::GameDefinition, gameplay::GameStep, player::Player};
use avian3d::prelude::*;
use bevy::prelude::*;
pub mod brushes;
pub mod triggers;
#[derive(Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvironmentDefinition {
    pub mud: Option<MudDefinition>,
    pub triggers: Vec<triggers::TriggerDefinition>,
    pub brush_entities: Vec<brushes::BrushDefinition>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MudDefinition {
    pub scale: f32,
    pub stamin_scale: f32,
    pub step: f32,
    pub stamin_step: f32,
    pub interval: f32,
    pub volumes: Vec<VolumeDefinition>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeDefinition {
    pub id: String,
    pub position: [f32; 3],
    pub vertices: Vec<[f32; 3]>,
}
impl EnvironmentDefinition {
    pub fn valid(&self) -> bool {
        triggers::valid(&self.triggers)
            && brushes::valid(&self.brush_entities)
            && self.mud.as_ref().is_none_or(|m| {
                [m.scale, m.stamin_scale, m.step, m.stamin_step, m.interval]
                    .iter()
                    .all(|v| v.is_finite() && *v > 0.)
                    && m.scale <= m.stamin_scale
                    && m.stamin_scale <= 1.
                    && m.step <= 1.
                    && m.stamin_step <= m.step
                    && !m.volumes.is_empty()
                    && m.volumes.len() <= 4096
                    && m.volumes.iter().all(|v| {
                        !v.id.is_empty()
                            && v.id.len() < 128
                            && v.position.iter().all(|p| p.is_finite())
                            && (4..=128).contains(&v.vertices.len())
                            && v.vertices.iter().flatten().all(|p| p.is_finite())
                            && Collider::convex_hull(
                                &v.vertices
                                    .iter()
                                    .copied()
                                    .map(Vec3::from_array)
                                    .collect::<Vec<_>>(),
                            )
                            .is_some()
                    })
                    && m.volumes
                        .iter()
                        .map(|v| &v.id)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        == m.volumes.len()
            })
    }
}
/// Default Avian membership is solid; query-only sensors occupy a separate layer.
pub fn solids(excluded: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(LayerMask::DEFAULT).with_excluded_entities(excluded)
}
/// Zombie ingress clips stop the player capsule without obstructing actors,
/// projectiles, repair targeting or navigation queries.
pub const PLAYER_INGRESS_LAYER: u32 = 4;
pub fn player_solids(excluded: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
    SpatialQueryFilter::from_mask(1u32 | PLAYER_INGRESS_LAYER).with_excluded_entities(excluded)
}
#[derive(Component)]
pub struct MudVolume(pub String);
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
        app.add_message::<Footstep>()
            .add_systems(OnEnter(GameState::Playing), triggers::spawn)
            .add_systems(OnEnter(GameState::Playing), spawn)
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
                    MudVolume(volume.id.clone()),
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
    volumes: Query<&MudVolume>,
    mut players: Query<
        (
            Entity,
            &Transform,
            &Collider,
            &crate::solo_revive::PerkInventory,
            &crate::life::Vitality,
            &mut TerrainMotion,
        ),
        With<Player>,
    >,
    mut evidence: ResMut<crate::movement::MovementEvidence>,
) {
    let Some(mud) = &game.environment.mud else {
        return;
    };
    for (owner, pose, shape, owned, health, mut motion) in &mut players {
        let touching = query.shape_intersections(
            shape,
            pose.translation,
            Quat::IDENTITY,
            &SpatialQueryFilter::from_mask(2u32),
        );
        let area = touching.iter().find_map(|e| volumes.get(*e).ok());
        let muddy = area.is_some()
            && matches!(
                health.state(),
                crate::life::LifeState::Standing | crate::life::LifeState::Downed
            );
        if motion.muddy != muddy {
            motion.muddy = muddy;
            evidence.record(serde_json::json!({"event":if muddy {"mud_enter"}else{"mud_leave"},"owner":owner.to_bits(),"area":area.map(|v|&v.0),"scale":motion.scale}));
        }
        // Retain native 100 ms staircase rather than smooth the intended decrement.
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
                evidence.record(serde_json::json!({"event":"mud_scale","mud":muddy,"scale":motion.scale,"stamin":stamin}));
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
    mut evidence: ResMut<crate::movement::MovementEvidence>,
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
                evidence.record(serde_json::json!({"event":"footstep","surface":format!("{surface:?}"),"sprint":sprint.sprinting}));
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
