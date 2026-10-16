use crate::{
    GameState,
    gameplay::GameStep,
    life::{LifeState, Vitality},
    player::Player,
};
use avian3d::prelude::*;
use avian3d::{
    collider_tree::{ColliderTreeProxyKey, ColliderTrees, MovedProxies},
    collision::collider::{ColliderAabbMargin, EnlargedAabb},
};
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
            .add_systems(
                FixedUpdate,
                sync.in_set(GameStep::Movement)
                    .before(carry)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                carry
                    .in_set(GameStep::Movement)
                    .before(crate::player::move_player)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                track
                    .in_set(GameStep::Movement)
                    .after(crate::player::move_player)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

pub(crate) fn sync(
    mut platforms: Query<
        (
            Entity,
            &Collider,
            &ColliderTreeProxyKey,
            &mut ColliderAabb,
            &mut EnlargedAabb,
            &ColliderAabbMargin,
            Option<&CollisionMargin>,
        ),
        (With<MovingPlatform>, Changed<Transform>),
    >,
    mut physics: PhysicsTransformHelper,
    mut trees: ResMut<ColliderTrees>,
    mut moved: ResMut<MovedProxies>,
    config: Res<NarrowPhaseConfig>,
    length_unit: Res<PhysicsLengthUnit>,
) {
    // Player movement queries Position/Rotation during FixedUpdate, before
    // Avian's ordinary PostUpdate transform synchronization.
    for (entity, shape, key, mut bounds, mut enlarged, margin, collision_margin) in &mut platforms {
        let (position, rotation) = physics
            .update_physics_transform(entity)
            .expect("Moving platform physics pose");
        *bounds = shape.aabb(
            position.0,
            rotation.0,
            length_unit.0 * config.contact_tolerance + collision_margin.map_or(0., |m| m.0),
        );
        if enlarged.update(&bounds, margin.0) {
            let tree = trees.tree_for_type_mut(key.tree_type());
            tree.resize_proxy_aabb(key.id(), enlarged.get().into());
            if moved.insert(*key) {
                tree.moved_proxies.push(key.id());
            }
        }
    }
}

#[cfg(test)]
#[test]
fn raised_platform_floor_is_queryable_before_the_next_physics_frame() {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        PhysicsPlugins::new(Update),
    ))
    .init_asset::<Mesh>();
    let platform = app
        .world_mut()
        .spawn((
            MovingPlatform,
            RigidBody::Kinematic,
            CustomPositionIntegration,
            Collider::cuboid(4., 1., 4.),
            Transform::default(),
        ))
        .id();
    app.finish();
    app.cleanup();
    app.update();
    app.world_mut()
        .get_mut::<Transform>(platform)
        .unwrap()
        .translation
        .y = 0.15;
    let floor = |query: SpatialQuery| {
        query.cast_ray(
            Vec3::new(0., 0.66, 0.),
            Dir3::NEG_Y,
            0.0254,
            false,
            &SpatialQueryFilter::default(),
        )
    };
    use bevy::ecs::system::RunSystemOnce;
    assert!(app.world_mut().run_system_once(floor).unwrap().is_none());
    app.world_mut().run_system_cached(sync).unwrap();
    let hit = app
        .world_mut()
        .run_system_once(floor)
        .unwrap()
        .expect("Updated platform floor");
    assert_eq!(hit.entity, platform);
    assert!((hit.distance - 0.01).abs() < 1e-5);
}

pub(crate) fn carry(
    time: Res<Time<Fixed>>,
    input: Res<crate::input::GameInput>,
    platforms: Query<&Transform, (With<MovingPlatform>, Without<Player>)>,
    mut players: Query<(
        Entity,
        &Player,
        &Vitality,
        &Collider,
        &mut Transform,
        &mut PlatformSupport,
    )>,
    slide: MoveAndSlide,
) {
    for (entity, player, health, collider, mut pose, mut support) in &mut players {
        if input.jump
            || !player.grounded
            || !matches!(health.state(), LifeState::Standing | LifeState::Downed)
        {
            support.entity = None;
            continue;
        }
        let Some(platform) = support.entity else {
            continue;
        };
        let Ok(platform_pose) = platforms.get(platform) else {
            support.entity = None;
            continue;
        };
        let point = platform_pose
            .compute_affine()
            .transform_point3(support.local_point);
        let displacement = point - support.previous_point;
        support.previous_point = point;
        if displacement.length_squared() < 0.0000001 {
            continue;
        }
        let result = slide.move_and_slide(
            collider,
            pose.translation,
            Quat::IDENTITY,
            displacement / time.delta_secs().max(0.00001),
            time.delta(),
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
        let hit = query.cast_ray(
            feet,
            Dir3::NEG_Y,
            0.3,
            false,
            &crate::environment::player_solids([entity]),
        );
        let Some((hit, platform)) =
            hit.and_then(|hit| platforms.get(hit.entity).ok().map(|pose| (hit, pose)))
        else {
            support.entity = None;
            continue;
        };
        support.entity = Some(hit.entity);
        support.local_point = platform.compute_affine().inverse().transform_point3(feet);
        support.previous_point = feet;
    }
}
