use crate::{GameState, gameplay::GameStep, life::{LifeState, Vitality}, player::Player};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct MovingPlatform;

#[derive(Component, Default)]
pub struct PlatformSupport {
    pub entity: Option<Entity>,
    local_point: Vec3,
    previous_point: Vec3,
}

pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Player, PlatformSupport>()
            .add_systems(FixedUpdate, carry
                .in_set(GameStep::Movement)
                .before(crate::player::move_player)
                .run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, track
                .in_set(GameStep::Movement)
                .after(crate::player::move_player)
                .run_if(in_state(GameState::Playing)));
    }
}

fn carry(
    time: Res<Time<Fixed>>,
    input: Res<crate::input::GameInput>,
    platforms: Query<&Transform, (With<MovingPlatform>, Without<Player>)>,
    mut players: Query<(Entity, &Player, &Vitality, &Collider, &mut Transform, &mut PlatformSupport)>,
    slide: MoveAndSlide,
) {
    for (entity, player, health, collider, mut pose, mut support) in &mut players {
        if input.jump || !player.grounded || !matches!(health.state(), LifeState::Standing | LifeState::Downed) {
            support.entity = None;
            continue;
        }
        let Some(platform) = support.entity else { continue };
        let Ok(platform_pose) = platforms.get(platform) else { support.entity = None; continue };
        let point = platform_pose.compute_affine().transform_point3(support.local_point);
        let displacement = point - support.previous_point;
        support.previous_point = point;
        if displacement.length_squared() < 0.0000001 { continue; }
        let result = slide.move_and_slide(
            collider, pose.translation, Quat::IDENTITY,
            displacement / time.delta_secs().max(0.00001), time.delta(),
            &MoveAndSlideConfig::default(),
            &crate::environment::player_solids([entity, platform]),
            |_| MoveAndSlideHitResponse::Accept,
        );
        pose.translation = result.position;
    }
}

fn track(
    query: SpatialQuery,
    platforms: Query<&Transform, (With<MovingPlatform>, Without<Player>)>,
    mut players: Query<(Entity, &Player, &Transform, &mut PlatformSupport)>,
) {
    for (entity, player, pose, mut support) in &mut players {
        if !player.grounded || player.velocity.y > 0. {
            support.entity = None;
            continue;
        }
        let feet = pose.translation - Vec3::Y * (player.height * 0.5 - 0.1);
        let hit = query.cast_ray(feet, Dir3::NEG_Y, 0.3, false,
            &crate::environment::player_solids([entity]));
        let Some((hit, platform)) = hit.and_then(|hit| platforms.get(hit.entity).ok().map(|pose| (hit, pose))) else {
            support.entity = None;
            continue;
        };
        support.entity = Some(hit.entity);
        support.local_point = platform.compute_affine().inverse().transform_point3(feet);
        support.previous_point = feet;
    }
}
