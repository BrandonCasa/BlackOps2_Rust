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
#[require(
    RigidBody::Kinematic,
    CustomPositionIntegration,
    SessionEntity = SessionEntity,
    crate::environment::TerrainMotion,
    crate::movement::MotionFeedback,
    crate::player_health::Recovery,
    crate::solo_revive::PerkInventory,
    crate::perk_effects::CherryState,
    crate::mines::MineInventory,
    crate::equipment::TacticalInventory,
    crate::crafting::BuildableInventory,
    crate::panzer::CapturedByPanzer,
)]
pub struct Player {
    pub yaw: f32,
    pub pitch: f32,
    pub velocity: Vec3,
    pub height: f32,
    pub grounded: bool,
    pub crouched: bool,
    pub prone: bool,
    pub eye_height: f32,
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
        app.add_plugins(crate::movement::platforms::PlatformPlugin);
        app.init_resource::<TickActions>()
            .init_resource::<crate::movement::MovementEvidence>()
            .add_message::<crate::movement::Landing>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                spawn_player.in_set(crate::session::SessionSetup::Player),
            )
            .add_systems(
                PreUpdate,
                look.after(crate::input::collect)
                    .after(crate::verification::drive),
            )
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
            prone: false,
            eye_height: definition.tuning.eye_height,
        },
        capsule(definition.tuning.radius, height),
        crate::movement::SprintState::new(definition.tuning.sprint_seconds),
        crate::movement::Mobility::new(definition.tuning.eye_height),
        vitality,
        inventory,
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
    settings: Res<crate::input::ControlSettings>,
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
    let hip_sensitivity = definition.tuning.mouse_sensitivity * settings.sensitivity;
    let ads_sensitivity = definition.tuning.ads_mouse_sensitivity * settings.ads_sensitivity;
    let sensitivity = hip_sensitivity + (ads_sensitivity - hip_sensitivity) * inventory.ads;
    player.yaw -= input.look.x * sensitivity;
    let vertical = if settings.invert_mouse_y { -1. } else { 1. };
    player.pitch = (player.pitch - input.look.y * sensitivity * vertical).clamp(-1.5, 1.5);
    input.look = Vec2::ZERO;
}

pub(crate) fn move_player(
    mut input: ResMut<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut actions: ResMut<TickActions>,
    definition: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    reserved: Query<&crate::actions::HandReservation>,
    robot_locks: Query<
        (),
        Or<(
            With<crate::robots::StompReaction>,
            With<crate::robots::RobotTransit>,
            With<crate::tank::TankRescue>,
        )>,
    >,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Collider,
        &crate::weapons::WeaponInventory,
        &Vitality,
        Option<&crate::perks::PerkDrinking>,
        &crate::solo_revive::PerkInventory,
        &mut crate::movement::SprintState,
        &mut crate::movement::Mobility,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
        &crate::mines::MineInventory,
        &crate::environment::TerrainMotion,
        &crate::panzer::CapturedByPanzer,
    )>,
    mut commands: Commands,
    slide: MoveAndSlide,
    mut landings: MessageWriter<crate::movement::Landing>,
    mut evidence: ResMut<crate::movement::MovementEvidence>,
) {
    let prone_edge = std::mem::take(&mut input.prone);
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
    let Ok((
        entity,
        mut player,
        mut transform,
        collider,
        inventory,
        vitality,
        drinking,
        perks,
        mut sprint,
        mut mobility,
        lethal,
        tactical,
        mines,
        terrain,
        captured,
    )) = players.single_mut()
    else {
        return;
    };
    if drinking.is_some() || reserved.contains(entity) {
        *actions = TickActions::default();
    }
    let downed = vitality.state() == crate::life::LifeState::Downed;
    if downed {
        actions.melee = false;
        actions.ads = false;
        actions.cycle_weapon = false;
        actions.weapon_slot = None;
    }
    if !matches!(
        vitality.state(),
        crate::life::LifeState::Standing | crate::life::LifeState::Downed
    ) {
        *actions = TickActions::default();
        return;
    }
    if robot_locks.contains(entity)
        || captured.0.is_some()
        || reserved
            .get(entity)
            .is_ok_and(|r| r.action == crate::actions::HandAction::Portal)
    {
        player.velocity = Vec3::ZERO;
        return;
    }
    let mut collider = collider.clone();
    let tuning = &definition.tuning;
    let dt = time.delta_secs();
    let filter = crate::environment::player_solids([entity]);
    let m = &tuning.mobility;
    mobility.recovery = (mobility.recovery - dt).max(0.);
    mobility.jump_recovery = (mobility.jump_recovery - dt).max(0.);
    mobility.landing_dip = (mobility.landing_dip - m.landing_dip / m.landing_seconds * dt).max(0.);
    mobility.previous_eye = player.eye_height;
    let mut requested = mobility.stance;
    let mut launch_dive = false;
    if downed {
        mobility.dive = None;
        mobility.recovery = 0.;
        mobility.airborne = false;
        requested = crate::movement::Stance::Crouched;
    } else if drinking.is_none() && !reserved.contains(entity) && !mobility.busy() {
        if prone_edge {
            requested = if mobility.stance == crate::movement::Stance::Prone {
                crate::movement::Stance::Standing
            } else {
                crate::movement::Stance::Prone
            };
            launch_dive = sprint.sprinting
                && player.grounded
                && mobility.stance == crate::movement::Stance::Standing
                && inventory.action == crate::weapons::WeaponAction::Ready
                && !lethal.busy()
                && !tactical.busy()
                && !mines.busy()
                && !input.grenade_held
                && !input.tactical_held
                && !input.mine_held;
        } else if input.crouch {
            requested = crate::movement::Stance::Crouched;
        } else if mobility.stance == crate::movement::Stance::Crouched {
            requested = crate::movement::Stance::Standing;
        }
    }
    let desired_height = if drinking.is_some() {
        player.height
    } else if downed {
        definition.solo_revive.downed_height
    } else {
        match requested {
            crate::movement::Stance::Standing => tuning.standing_height,
            crate::movement::Stance::Crouched => tuning.crouch_height,
            crate::movement::Stance::Prone => m.prone_height,
        }
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
            mobility.stance = requested;
            evidence.record(serde_json::json!({"event":"stance_changed","stance":format!("{requested:?}"),"height":desired_height,"feet":(center.y-desired_height*0.5)}));
        } else if prone_edge || input.crouch {
            evidence.record(
                serde_json::json!({"event":"stance_blocked","requested":format!("{requested:?}")}),
            );
        }
    }
    if desired_height == player.height && downed {
        mobility.stance = requested;
    }
    player.crouched = mobility.stance != crate::movement::Stance::Standing || downed;
    player.prone = mobility.stance == crate::movement::Stance::Prone && !downed;
    let ground = slide.spatial_query.cast_shape(
        &collider,
        transform.translation,
        Quat::IDENTITY,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(0.13),
        &filter,
    );
    let support = slide.spatial_query.cast_ray(
        transform.translation - Vec3::Y * (player.height * 0.5 - 0.1),
        Dir3::NEG_Y,
        0.25,
        false,
        &filter,
    );
    player.grounded = (ground.is_some_and(|hit| hit.normal1.y > 0.65)
        || support.is_some_and(|hit| hit.normal.y > 0.65))
        && (player.velocity.y <= 0. || !mobility.airborne);
    let feet = transform.translation - Vec3::Y * player.height * 0.5;
    if mobility.airborne && player.grounded && !launch_dive {
        let height = (mobility.peak - feet.y).max(0.);
        let diving = mobility.dive.take().is_some();
        landings.write(crate::movement::Landing {
            owner: entity,
            position: feet,
            height,
            speed: mobility.fall_speed,
            diving,
        });
        mobility.airborne = false;
        mobility.jump_recovery = m.jump_recovery;
        if diving {
            mobility.recovery = m.dive_recovery;
        }
        mobility.landing_dip = (mobility.fall_speed / 20.).clamp(0., 1.) * m.landing_dip;
        evidence.record(serde_json::json!({"event":"landed","height":height,"diving":diving,"position":feet.to_array(),"recovery":mobility.recovery}));
    }
    if launch_dive {
        mobility.dive = Some(crate::movement::Dive {
            direction: Quat::from_rotation_y(player.yaw) * Vec3::NEG_Z,
        });
        mobility.airborne = true;
        mobility.peak = feet.y;
        mobility.fall_speed = 0.;
        evidence.record(serde_json::json!({"event":"dive_started","feet":feet.to_array()}));
    }
    if mobility.busy() {
        *actions = TickActions::default();
        input.use_held = false;
        input.grenade_held = false;
        input.mine_held = false;
        input.tactical_held = false;
    }
    let target_eye = if downed {
        player.height * 0.8
    } else if player.prone {
        m.prone_eye
    } else if player.crouched {
        tuning.crouch_height * 0.9
    } else {
        tuning.eye_height
    };
    let target_eye = target_eye - mobility.landing_dip;
    let step = (tuning.eye_height - m.prone_eye) / m.stance_seconds * dt;
    player.eye_height += (target_eye - player.eye_height).clamp(-step, step);
    let (duration_scale, speed_scale) = definition
        .content
        .perks
        .iter()
        .filter(|p| perks.owned.contains(&p.id))
        .filter_map(|p| p.sprint.as_ref())
        .fold((1_f32, 1_f32), |(duration, speed), p| {
            (duration.max(p.duration_scale), speed.max(p.speed_scale))
        });
    let eligible = !downed
        && !player.crouched
        && !mobility.busy()
        && drinking.is_none()
        && !input.ads
        && inventory.ads < 0.05
        && inventory.action == crate::weapons::WeaponAction::Ready
        && !matches!(
            inventory.signal,
            crate::weapons::WeaponSignal::Fire | crate::weapons::WeaponSignal::DryFire
        )
        && !actions.fire
        && !actions.fire_held
        && !actions.melee
        && !actions.reload
        && input.movement.y > 0.1;
    sprint.update(
        dt,
        input.sprint,
        eligible,
        tuning.sprint_seconds * duration_scale,
        tuning.sprint_minimum,
        tuning.sprint_recovery_rate,
        tuning.sprint_recovery_delay,
        settings.sprint_mode == crate::input::PressMode::Hold,
    );
    let speed = if downed {
        definition.solo_revive.downed_speed
    } else if player.prone {
        m.prone_speed
    } else if player.crouched {
        tuning.crouch_speed
    } else if sprint.sprinting {
        tuning.sprint_speed * speed_scale
    } else {
        tuning.walk_speed * speed_scale
    } * terrain.scale
        * inventory
            .definition(&definition.content.weapons)
            .map_or(1., |weapon| {
                weapon.mechanics.movement[0]
                    + (weapon.mechanics.movement[1] - weapon.mechanics.movement[0]) * inventory.ads
            });
    let desired = Quat::from_rotation_y(player.yaw)
        * Vec3::new(input.movement.x, 0., -input.movement.y).clamp_length_max(1.)
        * speed;
    if let Some(dive) = mobility.dive {
        player.velocity.x = dive.direction.x * m.dive_speed * terrain.scale;
        player.velocity.z = dive.direction.z * m.dive_speed * terrain.scale;
    } else {
        let desired = if mobility.recovery > 0. {
            Vec2::ZERO
        } else {
            desired.xz()
        };
        let rate = if player.grounded {
            if desired.length_squared() > 0. {
                m.acceleration
            } else {
                m.deceleration
            }
        } else if desired.length_squared() > 0. {
            m.air_acceleration
        } else {
            m.air_deceleration
        };
        let velocity =
            player.velocity.xz() + (desired - player.velocity.xz()).clamp_length_max(rate * dt);
        player.velocity.x = velocity.x;
        player.velocity.z = velocity.y;
    }
    if launch_dive {
        player.velocity.y = m.dive_lift;
    } else if player.grounded {
        player.velocity.y = if jump
            && !downed
            && !player.prone
            && !mobility.busy()
            && mobility.jump_recovery == 0.
            && drinking.is_none()
        {
            tuning.jump_speed
        } else {
            -0.5
        };
    } else {
        player.velocity.y = (player.velocity.y - tuning.gravity * dt).max(-50.);
    }
    if !player.grounded || player.velocity.y > 0. {
        if !mobility.airborne {
            mobility.airborne = true;
            mobility.peak = feet.y;
            mobility.fall_speed = 0.;
        }
        mobility.peak = mobility.peak.max(feet.y);
        mobility.fall_speed = mobility.fall_speed.max(-player.velocity.y);
    }
    let result = slide_character(
        &slide,
        &slide.spatial_query,
        MotionShape {
            collider: &collider,
            radius: tuning.radius,
            step_height: if player.prone || mobility.dive.is_some() {
                m.prone_step
            } else {
                tuning.step_height
            },
            grounded: player.grounded,
        },
        transform.translation,
        player.velocity,
        time.delta(),
        &filter,
    );
    transform.translation = result.position;
    player.velocity.y = result.projected_velocity.y;
    if transform.translation.y < -100. {
        transform.translation =
            Vec3::from_array(definition.spawn) + Vec3::Y * (player.height * 0.5);
        player.velocity = Vec3::ZERO;
        mobility.airborne = false;
        mobility.dive = None;
    }
}

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
