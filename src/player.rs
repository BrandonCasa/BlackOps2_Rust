use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    life::{PlayerLethalPolicy, Vitality},
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(Component)]
#[require(RigidBody::Kinematic, CustomPositionIntegration)]
pub struct Player {
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub height: f32,
    pub grounded: bool,
    pub crouched: bool,
}

#[derive(Resource, Default)]
pub struct TickActions {
    pub fire: bool,
    pub melee: bool,
    pub reload: bool,
    pub fire_held: bool,
    pub ads: bool,
    pub cycle_weapon: bool,
    pub weapon_slot: Option<usize>,
}

pub struct PlayerPlugin;
impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickActions>()
            .add_systems(OnEnter(GameState::Playing), spawn_player)
            .add_systems(PreUpdate, look.after(crate::verification::drive))
            .add_systems(
                FixedUpdate,
                move_player
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn spawn_player(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    let height = definition.tuning.standing_height;
    let mut inventory = crate::weapons::WeaponInventory::default();
    if let Some(weapon) = definition
        .content
        .weapons
        .iter()
        .find(|w| w.id.0 == "weapon.c96_zm")
    {
        let _ = inventory.grant(weapon);
    }
    let Ok(vitality) = Vitality::human(
        definition.player_health.maximum,
        definition.player_health.maximum,
        1,
        PlayerLethalPolicy::SoloOrLastEligiblePlayer,
    ) else {
        return;
    };
    commands.spawn((
        Player {
            yaw: definition.yaw,
            pitch: 0.,
            velocity: Vec3::ZERO,
            height,
            grounded: false,
            crouched: false,
        },
        capsule(definition.tuning.radius, height),
        vitality,
        crate::player_health::Recovery::default(),
        inventory,
        crate::mines::MineInventory::default(),
        crate::equipment::GrenadeInventory::new(
            crate::origins::content::WeaponId("weapon.frag_grenade_zm".into()),
            definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.frag_grenade_zm")
                .and_then(|w| w.throwable.as_ref())
                .expect("validated starting grenade"),
        ),
        SessionEntity,
        Transform::from_translation(Vec3::from_array(definition.spawn) + Vec3::Y * (height * 0.5)),
        Name::new("Player"),
    ));
}

pub fn capsule(radius: f32, height: f32) -> Collider {
    Collider::capsule(radius, height - 2. * radius)
}

pub(crate) fn look(
    mut input: ResMut<GameInput>,
    definition: Option<Res<GameDefinition>>,
    mut players: Query<(&mut Player, &crate::weapons::WeaponInventory)>,
    state: Res<State<GameState>>,
) {
    if **state != GameState::Playing {
        input.look = Vec2::ZERO;
        return;
    }
    let (Some(definition), Ok((mut player, inventory))) = (definition, players.single_mut()) else {
        return;
    };
    let sensitivity = definition.tuning.mouse_sensitivity
        + (definition.tuning.ads_mouse_sensitivity - definition.tuning.mouse_sensitivity)
            * inventory.ads;
    player.yaw -= input.look.x * sensitivity;
    player.pitch = (player.pitch - input.look.y * sensitivity).clamp(-1.5, 1.5);
    input.look = Vec2::ZERO;
}

pub fn move_player(
    mut input: ResMut<GameInput>,
    mut actions: ResMut<TickActions>,
    definition: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Collider,
        &crate::weapons::WeaponInventory,
    )>,
    mut commands: Commands,
    slide: MoveAndSlide,
    query: SpatialQuery,
) {
    let (jump, fire, melee, reload) = input.take_edges();
    *actions = TickActions {
        fire,
        melee,
        reload,
        fire_held: input.fire_held,
        ads: input.ads,
        cycle_weapon: std::mem::take(&mut input.cycle_weapon),
        weapon_slot: input.weapon_slot.take(),
    };
    let Ok((entity, mut player, mut transform, collider, inventory)) = players.single_mut() else {
        return;
    };
    let mut collider = collider.clone();
    let tuning = &definition.tuning;
    let dt = time.delta_secs();
    let filter = SpatialQueryFilter::from_excluded_entities([entity]);
    let desired_height = if input.crouch {
        tuning.crouch_height
    } else {
        tuning.standing_height
    };
    if desired_height != player.height {
        let shape = capsule(tuning.radius, desired_height);
        let center = transform.translation + Vec3::Y * ((desired_height - player.height) * 0.5);
        let mut blocked = false;
        slide.intersections(
            &shape,
            center,
            Quat::IDENTITY,
            0.,
            &filter,
            |_, point, _| {
                blocked |= point.penetration > 0.015;
                true
            },
        );
        if desired_height < player.height || !blocked {
            transform.translation = center;
            collider = shape;
            commands.entity(entity).insert(collider.clone());
            player.height = desired_height;
            player.crouched = input.crouch;
        }
    }
    let ground = query.cast_shape(
        &collider,
        transform.translation,
        Quat::IDENTITY,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(0.13),
        &filter,
    );
    let support = query.cast_ray(
        transform.translation - Vec3::Y * (player.height * 0.5 - 0.1),
        Dir3::NEG_Y,
        0.25,
        false,
        &filter,
    );
    player.grounded = (ground.is_some_and(|hit| hit.normal1.y > 0.65)
        || support.is_some_and(|hit| hit.normal.y > 0.65))
        && player.velocity.y <= 0.;
    let speed = if player.crouched {
        tuning.crouch_speed
    } else if input.sprint
        && !input.ads
        && inventory.ads < 0.05
        && inventory.action == crate::weapons::WeaponAction::Ready
    {
        tuning.sprint_speed
    } else {
        tuning.walk_speed
    } * inventory
        .definition(&definition.content.weapons)
        .map_or(1., |weapon| {
            weapon.mechanics.movement[0]
                + (weapon.mechanics.movement[1] - weapon.mechanics.movement[0]) * inventory.ads
        });
    let desired = Quat::from_rotation_y(player.yaw)
        * Vec3::new(input.movement.x, 0., -input.movement.y)
        * speed;
    player.velocity.x = desired.x;
    player.velocity.z = desired.z;
    if player.grounded {
        player.velocity.y = if jump { tuning.jump_speed } else { -0.5 };
    } else {
        player.velocity.y = (player.velocity.y - tuning.gravity * dt).max(-50.);
    }
    let result = slide_character(
        &slide,
        &query,
        MotionShape {
            collider: &collider,
            radius: tuning.radius,
            step_height: tuning.step_height,
            grounded: player.grounded,
        },
        transform.translation,
        player.velocity,
        time.delta(),
        &filter,
    );
    transform.translation = result.position;
    if result.projected_velocity.y < player.velocity.y {
        player.velocity.y = result.projected_velocity.y;
    }
    if transform.translation.y < -100. {
        transform.translation =
            Vec3::from_array(definition.spawn) + Vec3::Y * (player.height * 0.5);
        player.velocity = Vec3::ZERO;
    }
}

/// Shared capsule stepping over ordinary map obstacles using Avian queries.
pub struct MotionShape<'a> {
    pub collider: &'a Collider,
    pub radius: f32,
    pub step_height: f32,
    pub grounded: bool,
}
pub fn slide_character(
    slide: &MoveAndSlide,
    query: &SpatialQuery,
    motor: MotionShape<'_>,
    start: Vec3,
    velocity: Vec3,
    delta: std::time::Duration,
    filter: &SpatialQueryFilter,
) -> MoveAndSlideOutput {
    let dt = delta.as_secs_f32();
    let mut result = slide.move_and_slide(
        motor.collider,
        start,
        Quat::IDENTITY,
        velocity,
        delta,
        &MoveAndSlideConfig::default(),
        filter,
        |_| MoveAndSlideHitResponse::Accept,
    );
    let expected_horizontal = Vec3::new(velocity.x, 0., velocity.z).length() * dt;
    let actual_horizontal =
        Vec2::new(result.position.x - start.x, result.position.z - start.z).length();
    let mut position = result.position;
    if motor.grounded && expected_horizontal > 0.01 && actual_horizontal < expected_horizontal * 0.6
    {
        let raised = start + Vec3::Y * motor.step_height;
        if query
            .cast_shape(
                motor.collider,
                start,
                Quat::IDENTITY,
                Dir3::Y,
                &ShapeCastConfig {
                    ignore_origin_penetration: true,
                    ..ShapeCastConfig::from_max_distance(motor.step_height)
                },
                filter,
            )
            .is_none()
        {
            let stepped = slide.move_and_slide(
                motor.collider,
                raised,
                Quat::IDENTITY,
                Vec3::new(velocity.x, 0., velocity.z),
                std::time::Duration::from_secs_f32(
                    dt.max((motor.radius + 0.1) / velocity.with_y(0.).length().max(0.1)),
                ),
                &MoveAndSlideConfig::default(),
                filter,
                |_| MoveAndSlideHitResponse::Accept,
            );
            if let Some(hit) = query.cast_shape(
                motor.collider,
                stepped.position,
                Quat::IDENTITY,
                Dir3::NEG_Y,
                &ShapeCastConfig::from_max_distance(motor.step_height + 0.13),
                filter,
            ) && hit.normal1.y > 0.65
                && Vec2::new(stepped.position.x - start.x, stepped.position.z - start.z).length()
                    > actual_horizontal
            {
                let landing = stepped.position - Vec3::Y * hit.distance;
                position = start + Vec3::new(velocity.x, 0., velocity.z) * dt;
                position.y = landing.y;
            }
        }
    }
    result.position = position;
    result
}
