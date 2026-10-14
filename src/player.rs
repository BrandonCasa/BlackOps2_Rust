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

#[derive(Component)]
pub(crate) struct PreviousTranslation(pub Vec3);

fn remember_translation(mut players: Query<(&Transform, &mut PreviousTranslation), With<Player>>) {
    for (pose, mut previous) in &mut players {
        previous.0 = pose.translation;
    }
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
            .init_resource::<crate::movement::physics::PhysicsEvidence>()
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
                remember_translation
                    .before(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                move_player
                    .in_set(GameStep::Movement)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn choose_initial_spawn<'a>(
    spawns: &'a [crate::origins::InitialSpawn],
    session: &mut crate::gameplay::Session,
) -> Option<&'a crate::origins::InitialSpawn> {
    let side = if session.random() % 100 > 50 { 1 } else { 2 };
    let eligible: Vec<_> = spawns
        .iter()
        .filter(|spawn| spawn.excluded_side != side)
        .collect();
    session
        .spawn_index(eligible.len())
        .map(|index| eligible[index])
}

#[cfg(test)]
mod initial_spawn_tests {
    use super::*;

    #[test]
    fn source_solo_spawn_selection_keeps_side_exclusion_and_authored_look() {
        let spawns: Vec<_> = (0..8)
            .map(|index| crate::origins::InitialSpawn {
                position: [index as f32, 0., 0.],
                yaw: index as f32 * 0.1,
                pitch: if index < 6 { 5_f32.to_radians() } else { 0. },
                excluded_side: if index < 4 { 1 } else { 2 },
            })
            .collect();
        let mut sides = [false; 2];
        for seed in 0..1000 {
            let mut expected = crate::gameplay::Session { seed, ..default() };
            let side = if expected.random() % 100 > 50 { 1 } else { 2 };
            sides[(side - 1) as usize] = true;
            let mut session = crate::gameplay::Session { seed, ..default() };
            let selected = choose_initial_spawn(&spawns, &mut session).unwrap();
            assert_ne!(selected.excluded_side, side);
            let index = selected.position[0] as usize;
            assert_eq!(selected.yaw, spawns[index].yaw);
            assert_eq!(selected.pitch, spawns[index].pitch);
            assert_eq!(session.spawn_cursor, 1);
        }
        assert!(sides.into_iter().all(|seen| seen));
        assert!(choose_initial_spawn(&[], &mut crate::gameplay::Session::default()).is_none());
    }
}

fn spawn_player(
    mut commands: Commands,
    mut physics_evidence: ResMut<crate::movement::physics::PhysicsEvidence>,
    definition: Res<GameDefinition>,
    mut session: ResMut<crate::gameplay::Session>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    physics_evidence.samples.clear();
    physics_evidence.projectiles.clear();
    let (spawn, yaw, pitch) =
        if definition.delivery_scope == crate::origins::DeliveryScope::FirstPlayableSolo {
            let selected = choose_initial_spawn(&definition.initial_spawns, &mut session)
                .expect("validated source initial spawn candidates");
            (selected.position, selected.yaw, selected.pitch)
        } else {
            (definition.spawn, definition.yaw, 0.)
        };
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
            yaw,
            pitch,
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
        Transform::from_translation(Vec3::from_array(spawn) + Vec3::Y * (height * 0.5)),
        PreviousTranslation(Vec3::from_array(spawn) + Vec3::Y * (height * 0.5)),
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
    options: Res<crate::assets::LaunchOptions>,
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
    mut physics_evidence: ResMut<crate::movement::physics::PhysicsEvidence>,
    surfaces: Query<&crate::movement::physics::SurfaceFlags>,
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
    mobility.landing_recovery = (mobility.landing_recovery - dt).max(0.);
    mobility.landing_dip = (mobility.landing_dip - m.landing_dip / m.landing_seconds * dt).max(0.);
    mobility.previous_eye = player.eye_height;
    if let Some(age) = mobility.since_dive.as_mut() {
        *age += dt;
    }
    if let Some(dive) = mobility.dive.as_mut() {
        dive.elapsed += dt;
    }
    let finish_dive = if let Some(dive) = mobility.dive.as_mut()
        && let Some(remaining) = dive.slide_remaining.as_mut()
    {
        *remaining = (*remaining - dt).max(0.);
        *remaining == 0. || player.velocity.length_squared() < m.dive_minimum_speed.powi(2)
    } else {
        false
    };
    if finish_dive {
        mobility.dive = None;
        mobility.recovery = m.dive_recovery;
        mobility.since_dive = Some(0.);
        player.velocity = Vec3::ZERO;
        evidence.record(serde_json::json!({"event":"dive_finished","seconds":time.elapsed_secs_f64(),"recovery":mobility.recovery}));
    }
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
                && input.sprint
                && input.movement.y > 0.1
                && !input.ads
                && sprint.continuous_seconds() > m.dive_startup
                && player.velocity.xz().length_squared() > m.dive_minimum_speed.powi(2)
                && mobility
                    .since_dive
                    .is_none_or(|age| age > m.dive_exhaustion)
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
    let step_height = if player.prone || mobility.dive.is_some() {
        m.prone_step
    } else {
        tuning.step_height
    };
    let ground = slide.spatial_query.cast_shape(
        &collider,
        transform.translation,
        Quat::IDENTITY,
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(tuning.physics.ground_probe),
        &filter,
    );
    let support = slide.spatial_query.cast_ray(
        transform.translation - Vec3::Y * (player.height * 0.5 - tuning.physics.skin_width),
        Dir3::NEG_Y,
        tuning.physics.ground_probe + tuning.physics.skin_width,
        false,
        &filter,
    );
    let ground_normal = ground
        .and_then(|hit| {
            crate::movement::physics::walkable_surface(
                &slide.spatial_query,
                hit.point1,
                hit.normal1,
                &filter,
                &tuning.physics,
            )
        })
        .filter(|(_, height)| {
            height - (transform.translation.y - player.height * 0.5)
                <= step_height + tuning.physics.skin_width
        })
        .map(|(normal, _)| normal)
        .or_else(|| {
            support
                .filter(|hit| hit.normal.y >= tuning.physics.ground_normal_min)
                .map(|hit| hit.normal)
        });
    // Avian can project a floor contact to a tiny positive vertical velocity.
    // Accept that contact before a second airborne tick overwrites impact energy.
    player.grounded = ground_normal.is_some() && (player.velocity.y <= 1e-3 || !mobility.airborne);
    let surface_flags = support
        .map(|hit| hit.entity)
        .or_else(|| ground.map(|hit| hit.entity))
        .and_then(|entity| surfaces.get(entity).ok())
        .map_or(0, |surface| surface.0);
    let slick = surface_flags & 2 != 0;
    let feet = transform.translation - Vec3::Y * player.height * 0.5;
    if mobility.airborne && player.grounded && !launch_dive {
        let height = (mobility.peak - feet.y).max(0.);
        let impact_height = (mobility.fall_velocity.powi(2) / (2. * tuning.gravity)
            + mobility.fall_origin
            - feet.y)
            .max(0.);
        let diving = mobility.dive.is_some();
        landings.write(crate::movement::Landing {
            owner: entity,
            position: feet,
            height,
            impact_height,
            speed: mobility.fall_speed,
            diving,
            surface_flags,
        });
        let protected = definition
            .content
            .perks
            .iter()
            .any(|perk| perk.fall_protection && perks.owned.contains(&perk.id));
        let percent = crate::movement::physics::fall_percent(
            impact_height,
            diving,
            protected,
            surface_flags,
            m,
        );
        let (impact_scale, recovery) =
            crate::movement::physics::landing_response(impact_height, percent, slick);
        player.velocity *= impact_scale;
        mobility.landing_recovery = recovery;
        mobility.airborne = false;
        mobility.jump_recovery = m.jump_recovery;
        if diving {
            mobility.dive.as_mut().unwrap().slide_remaining = Some(m.dive_slide);
        }
        mobility.landing_dip = (mobility.fall_speed / 20.).clamp(0., 1.) * m.landing_dip;
        evidence.record(serde_json::json!({"event":"landed","seconds":time.elapsed_secs_f64(),"height":height,"impact_height":impact_height,"diving":diving,"position":feet.to_array(),"recovery":mobility.recovery,"landing_recovery":mobility.landing_recovery,"impact_scale":impact_scale,"percent":percent,"surface_flags":surface_flags}));
    }
    if launch_dive {
        mobility.dive = Some(crate::movement::Dive {
            launch_height: feet.y,
            elapsed: 0.,
            slide_remaining: None,
        });
        mobility.airborne = true;
        mobility.peak = feet.y;
        mobility.fall_speed = 0.;
        evidence.record(serde_json::json!({"event":"dive_started","seconds":time.elapsed_secs_f64(),"feet":feet.to_array(),"sprint_seconds":sprint.continuous_seconds(),"speed":player.velocity.xz().length(),"since_dive":mobility.since_dive}));
    }
    if mobility.busy() {
        *actions = TickActions::default();
        input.use_held = false;
        input.grenade_held = false;
        input.mine_held = false;
        input.tactical_held = false;
        input.buildable = false;
        input.drone = false;
        input.staff_secondary = false;
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
        && input.movement.y >= 105. / 127.;
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
    let movement_scale = terrain.scale
        * inventory
            .definition(&definition.content.weapons)
            .map_or(1., |weapon| {
                weapon.mechanics.movement[0]
                    + (weapon.mechanics.movement[1] - weapon.mechanics.movement[0]) * inventory.ads
            });
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
    } * movement_scale;
    let jump_started = jump
        && player.grounded
        && !downed
        && !player.prone
        && !mobility.busy()
        && mobility.jump_recovery == 0.
        && !slick
        && drinking.is_none();
    let motion_grounded = player.grounded && !launch_dive && !jump_started;
    let desired = tuning.physics.wish_velocity(
        input.movement,
        player.yaw,
        if motion_grounded {
            speed
        } else {
            tuning.walk_speed
                * speed_scale
                * if player.prone || input.ads || inventory.ads > 0. {
                    0.4
                } else {
                    1.
                }
        },
        sprint.sprinting && motion_grounded,
        motion_grounded,
    );
    let velocity_before = player.velocity;
    let mut motion_wish = desired;
    let mut apex_clamped = false;
    if launch_dive {
        player.velocity.y =
            (2. * m.dive_jump_height * tuning.gravity).sqrt() * m.dive_launch_multiplier;
    } else if let Some(dive) = mobility.dive
        && feet.y >= dive.launch_height + m.dive_jump_height
    {
        player.velocity.y = if dive.slide_remaining.is_none() && dive.elapsed < m.dive_apex_seconds
        {
            0.
        } else {
            player.velocity.y.min(0.)
        };
        apex_clamped = true;
    } else if jump_started {
        player.velocity.y = tuning.jump_speed;
    }
    if let Some(dive) = mobility.dive {
        let dive_grounded = motion_grounded;
        let friction_speed = if dive_grounded {
            player.velocity.xz().length()
        } else {
            player.velocity.length()
        };
        if friction_speed < 0.0254 {
            player.velocity = Vec3::ZERO;
        } else if dive_grounded && let Some(remaining) = dive.slide_remaining {
            let fraction = 1. - remaining / m.dive_slide;
            let retained = (friction_speed
                - friction_speed.max(tuning.physics.stop_speed)
                    * if slick { 1.5 } else { tuning.physics.friction }
                    * fraction
                    * dt)
                .max(0.)
                / friction_speed;
            player.velocity *= retained;
        }
        let wish = tuning.physics.wish_velocity(
            input.movement,
            player.yaw,
            tuning.walk_speed * speed_scale * 0.4,
            false,
            false,
        );
        motion_wish = wish;
        let direction = wish.normalize_or_zero();
        let wish_speed = wish.length();
        let add = wish_speed - player.velocity.dot(direction);
        if add > 0. {
            let acceleration = tuning.physics.air_acceleration;
            player.velocity +=
                direction * add.min(dt * acceleration * wish_speed.max(tuning.physics.stop_speed));
        }
    } else {
        let desired = if mobility.recovery > 0. {
            Vec3::ZERO
        } else {
            desired
        };
        player.velocity = tuning.physics.velocity(
            player.velocity,
            desired,
            ground_normal.filter(|_| motion_grounded),
            player.prone,
            player.crouched,
            slick,
            mobility.landing_recovery > 0.,
            dt,
        );
    }
    if motion_grounded && let Some(normal) = ground_normal {
        if mobility.dive.is_some() {
            let normal_speed = player.velocity.dot(normal);
            player.velocity -= normal * (normal_speed - normal_speed.abs() * 0.001);
        } else {
            player.velocity = crate::movement::physics::ground_velocity(player.velocity, normal);
        }
    }
    if launch_dive || jump_started {
        player.grounded = false;
    }
    let gravity_step = if !player.grounded || mobility.dive.is_some() || slick {
        tuning.gravity * dt
    } else {
        0.
    };
    mobility.fall_origin = feet.y;
    mobility.fall_velocity = player.velocity.y;
    let move_velocity = player.velocity - Vec3::Y * gravity_step * 0.5;
    if !player.grounded {
        if !mobility.airborne {
            mobility.airborne = true;
            mobility.peak = feet.y;
            mobility.fall_speed = 0.;
        }
        mobility.peak = mobility.peak.max(feet.y);
        mobility.fall_speed = mobility.fall_speed.max(gravity_step - player.velocity.y);
    }
    let motion = crate::movement::physics::move_and_slide(
        &slide,
        &slide.spatial_query,
        MotionShape {
            collider: &collider,
            radius: tuning.radius,
            step_height,
            grounded: player.grounded,
        },
        transform.translation,
        move_velocity,
        time.delta(),
        &filter,
        &tuning.physics,
    );
    let result = motion.slide;
    let step_scale = if player.grounded {
        tuning.physics.step_scale(
            motion.step_adjustment,
            result.position.y - transform.translation.y,
            step_height,
        )
    } else {
        1.
    };
    transform.translation = result.position;
    player.velocity = result.projected_velocity;
    if (player.velocity.y - move_velocity.y).abs() < 0.00001 {
        player.velocity.y -= gravity_step * 0.5;
    }
    player.velocity *= step_scale;
    if cfg!(debug_assertions)
        && options.validation_remote.is_some()
        && physics_evidence.samples.len() < 512
    {
        physics_evidence.samples.push(serde_json::json!({
            "seconds":time.elapsed_secs_f64(),"dt":dt,"input":input.movement.to_array(),
            "grounded":player.grounded,"ground_normal":ground_normal.map(|n| n.to_array()),
            "surface_flags":surface_flags,"landing_recovery":mobility.landing_recovery,
            "prone":player.prone,"crouched":player.crouched,"sprinting":sprint.sprinting,
            "diving":mobility.dive.is_some(),"wish":motion_wish.to_array(),
            "position_before":feet.to_array(),"position_after":(result.position-Vec3::Y*player.height*0.5).to_array(),
            "velocity_before":velocity_before.to_array(),"move_velocity":move_velocity.to_array(),
            "collision_velocity":result.projected_velocity.to_array(),"step_scale":step_scale,
            "step_adjustment":motion.step_adjustment,
            "step_rise":motion.step_rise,"step_normal":motion.step_normal.map(|n| n.to_array()),
            "step_surface_normal":motion.step_surface.map(|(normal, _)| normal.to_array()),
            "step_surface_height":motion.step_surface.map(|(_, height)| height),
            "velocity_after":player.velocity.to_array()
        }));
    }
    if cfg!(debug_assertions)
        && options.validation_remote.is_some()
        && (mobility.airborne || mobility.dive.is_some())
        && evidence.motion.len() < 512
    {
        evidence.motion.push(serde_json::json!({"seconds":time.elapsed_secs_f64(),"dt":dt,"diving":mobility.dive.is_some(),"elapsed":mobility.dive.map(|d|d.elapsed),"slide_remaining":mobility.dive.and_then(|d|d.slide_remaining),"launch_height":mobility.dive.map(|d|d.launch_height),"apex_clamped":apex_clamped,"feet_before":feet.to_array(),"feet_after":(result.position-Vec3::Y*player.height*0.5).to_array(),"velocity_before":velocity_before.to_array(),"vertical_before_gravity":mobility.fall_velocity,"gravity_step":gravity_step,"move_velocity":move_velocity.to_array(),"velocity_after":player.velocity.to_array()}));
    }
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
    crate::movement::physics::move_and_slide(
        slide,
        query,
        motor,
        start,
        velocity,
        delta,
        filter,
        &default(),
    )
    .slide
}
