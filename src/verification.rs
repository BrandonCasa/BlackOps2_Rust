//! Opt-in bounded verification of the actual application, never a second viewer.
use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions, LoadFailure},
    gameplay::{Session, Zombie},
    input::GameInput,
    player::Player,
};
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::{path::PathBuf, time::Instant};
mod arsenal;
mod claymores;
mod equipment_refills;
mod monkeys;
mod performance;
mod pipelines;
mod semtex;

#[derive(Resource)]
pub(crate) struct Verification {
    output: PathBuf,
    benchmark: bool,
    combat: bool,
    only_view: bool,
    moving_view: bool,
    started: Instant,
    playing: f32,
    stage: usize,
    initial: Option<Vec3>,
    moved: bool,
    survey: usize,
    round_two: bool,
    water_times: Vec<f32>,
    camera_views: Vec<serde_json::Value>,
    reported: u32,
    ads_seen: bool,
    recoil_seen: bool,
    wall_weapons: bool,
    rounds: bool,
    health: bool,
    hit_regions: bool,
    grenades: bool,
    semtex: bool,
    semtex_exercised: bool,
    claymores: bool,
    claymores_exercised: bool,
    monkeys: bool,
    monkeys_exercised: bool,
    equipment_refills: bool,
    equipment_refills_exercised: bool,
    grenade_flash_frame: bool,
    accepted_regions: std::collections::BTreeSet<String>,
    damage_frame: bool,
    healed_frame: bool,
    death_frame: bool,
    melee_frame: bool,
    enemy_attack_frame: bool,
    enemy_death_frame: bool,
    combat_receipt: serde_json::Value,
}
pub struct VerificationPlugin;
impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        if app.world().resource::<LaunchOptions>().verify.is_some() {
            pipelines::install(app);
            if app
                .world()
                .resource::<LaunchOptions>()
                .verify_equipment_refills
            {
                equipment_refills::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_wall_weapons {
                arsenal::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_monkeys {
                monkeys::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_claymores {
                claymores::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_semtex {
                semtex::install(app);
            }
        }
        app.add_systems(Startup, setup)
            .add_systems(
                PreUpdate,
                drive
                    .after(bevy::input::InputSystems)
                    .after(crate::input::collect),
            )
            .add_systems(Update, check)
            .add_systems(Last, performance::record);
    }
}
fn setup(mut commands: Commands, options: Res<LaunchOptions>) {
    if let Some(output) = &options.verify {
        if output.starts_with(env!("CARGO_MANIFEST_DIR")) {
            error!("Verification output must be external");
            commands.write_message(AppExit::error());
            return;
        }
        if let Err(error) = std::fs::create_dir_all(output) {
            error!("Verification directory: {error}");
            commands.write_message(AppExit::error());
            return;
        }
        commands.insert_resource(Verification {
            output: output.clone(),
            benchmark: options.benchmark,
            combat: options.benchmark_combat,
            only_view: options.benchmark_view.is_some(),
            moving_view: options.benchmark_motion,
            started: Instant::now(),
            playing: 0.,
            stage: 0,
            initial: None,
            moved: false,
            survey: options.benchmark_view.unwrap_or(0),
            round_two: false,
            water_times: Vec::new(),
            camera_views: Vec::new(),
            reported: 0,
            ads_seen: false,
            recoil_seen: false,
            wall_weapons: options.verify_wall_weapons
                || options.verify_semtex
                || options.verify_claymores
                || options.verify_monkeys
                || options.verify_equipment_refills,
            equipment_refills: options.verify_equipment_refills,
            equipment_refills_exercised: false,
            rounds: options.verify_rounds,
            health: options.verify_health,
            hit_regions: options.verify_hit_regions,
            grenades: options.verify_grenades,
            semtex: options.verify_semtex,
            semtex_exercised: false,
            claymores: options.verify_claymores,
            claymores_exercised: false,
            monkeys: options.verify_monkeys,
            monkeys_exercised: false,
            grenade_flash_frame: false,
            accepted_regions: Default::default(),
            damage_frame: false,
            healed_frame: false,
            death_frame: false,
            melee_frame: false,
            enemy_attack_frame: false,
            enemy_death_frame: false,
            combat_receipt: serde_json::Value::Null,
        });
    }
}
pub(crate) fn drive(
    verification: Option<Res<Verification>>,
    mut input: ResMut<GameInput>,
    players: Query<(Entity, &Player, &Transform)>,
    enemies: crate::hit_regions::RayTargets,
    inventories: Query<&crate::weapons::WeaponInventory, With<Player>>,
    history: Res<crate::damage::CombatHistory>,
    enemy_evidence: Res<crate::enemies::EnemyEvidence>,
    health_evidence: Res<crate::player_health::HealthEvidence>,
    vitality: Query<&crate::life::Vitality, With<Player>>,
    definition: Option<Res<GameDefinition>>,
    query: avian3d::prelude::SpatialQuery,
    equipment: Res<crate::equipment::EquipmentEvidence>,
    grenades: Query<(&crate::equipment::Grenade, &Transform), Without<Player>>,
    grenade_inventory: Query<&crate::equipment::GrenadeInventory, With<Player>>,
    mut grenade_escape: Local<Option<Vec3>>,
) {
    let (Some(verification), Some(definition), Ok((entity, player, transform))) =
        (verification, definition, players.single())
    else {
        return;
    };
    if verification.stage != 0
        && !((verification.health || verification.grenades) && verification.stage == 2)
    {
        if verification.stage != 4 {
            input.clear();
        }
        return;
    }
    input.movement = if verification.playing < 1. {
        Vec2::new(0.25, 0.35)
    } else {
        Vec2::ZERO
    };
    if verification.grenades && (equipment.detonations == 0 || verification.stage == 2) {
        input.fire = false;
        input.fire_held = false;
        input.ads = false;
        input.melee = false;
        input.reload = false;
        input.grenade_held = verification.stage == 2
            || (equipment.thrown == 0
                && grenade_inventory.single().is_ok_and(|i| {
                    matches!(
                        i.state,
                        crate::equipment::ThrowState::Ready
                            | crate::equipment::ThrowState::Priming { .. }
                    )
                }));
        input.look = Vec2::new(0., player.pitch + 1.5) / definition.tuning.mouse_sensitivity;
        if let Some((_, pose)) = grenades.iter().next() {
            let delta = pose.translation
                - (transform.translation
                    + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5));
            let yaw = (-delta.x).atan2(-delta.z);
            input.look = Vec2::new(
                -((yaw - player.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI),
                player.pitch - delta.y.atan2(delta.xz().length()),
            ) / definition.tuning.mouse_sensitivity;
        }
        input.movement = Vec2::ZERO;
        if verification.stage == 0 && grenades.iter().any(|(g, _)| g.remaining < 2.5) {
            let escape = grenade_escape.get_or_insert_with(|| {
                [Vec3::X, Vec3::NEG_X, Vec3::Z, Vec3::NEG_Z]
                    .into_iter()
                    .max_by(|a, b| {
                        let clear = |dir: Vec3| {
                            query
                                .cast_shape(
                                    &crate::player::capsule(
                                        definition.tuning.radius,
                                        player.height,
                                    ),
                                    transform.translation,
                                    Quat::IDENTITY,
                                    Dir3::new(dir).unwrap(),
                                    &avian3d::prelude::ShapeCastConfig {
                                        ignore_origin_penetration: true,
                                        ..avian3d::prelude::ShapeCastConfig::from_max_distance(8.)
                                    },
                                    &avian3d::prelude::SpatialQueryFilter::from_excluded_entities(
                                        [entity],
                                    ),
                                )
                                .map_or(8., |hit| hit.distance)
                        };
                        clear(*a).total_cmp(&clear(*b))
                    })
                    .unwrap_or(Vec3::X)
            });
            let local = Quat::from_rotation_y(-player.yaw) * *escape;
            input.movement = Vec2::new(local.x, -local.z);
        }
        input.sprint = input.movement.length_squared() > 0.;
        return;
    }
    input.grenade_held = false;
    *grenade_escape = None;
    input.sprint = false;
    let origin =
        transform.translation + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
    let wanted = [
        crate::weapons::HitRegion::Leg,
        crate::weapons::HitRegion::Arm,
        crate::weapons::HitRegion::Torso,
        crate::weapons::HitRegion::UpperTorso,
        crate::weapons::HitRegion::Head,
    ]
    .into_iter()
    .find(|region| {
        !history.recent.iter().any(|r| {
            matches!(
                r.impact.category,
                crate::origins::content::DamageCategory::Bullet
            ) && r.impact.region == *region
        })
    });
    let aim = verification
        .hit_regions
        .then(|| wanted)
        .flatten()
        .and_then(|region| {
            enemies
                .iter()
                .filter(|(_, health, _, _)| health.state() == crate::life::LifeState::ActorAlive)
                .flat_map(|(_, _, t, boxes)| boxes.aim_points(t, region))
                .filter(|point| {
                    Dir3::new(*point - origin)
                        .ok()
                        .and_then(|dir| {
                            crate::hit_regions::cast(&enemies, &query, entity, origin, dir, 200.)
                        })
                        .is_some_and(|hit| hit.hit.region == region)
                })
                .min_by(|a, b| {
                    a.distance_squared(origin)
                        .total_cmp(&b.distance_squared(origin))
                })
        });
    if let Some((_, _, target, boxes)) = enemies
        .iter()
        .filter(|(_, health, _, _)| health.state() == crate::life::LifeState::ActorAlive)
        .min_by(|(_, _, a, _), (_, _, b, _)| {
            a.translation
                .distance_squared(transform.translation)
                .total_cmp(&b.translation.distance_squared(transform.translation))
        })
    {
        let delta = aim
            .or_else(|| {
                boxes
                    .aim_points(target, crate::weapons::HitRegion::Head)
                    .next()
            })
            .unwrap_or(target.translation + Vec3::Y * 0.65)
            - origin;
        let yaw = (-delta.x).atan2(-delta.z);
        if verification.health && (verification.stage == 2 || health_evidence.nonterminal_hits < 2)
        {
            input.look = Vec2::new(
                -((yaw - player.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                    - std::f32::consts::PI),
                player.pitch - delta.y.atan2(Vec2::new(delta.x, delta.z).length()),
            ) / definition.tuning.mouse_sensitivity;
            let wounded = vitality
                .get(entity)
                .is_ok_and(|v| v.current() != Some(v.maximum()));
            input.movement = Vec2::new(
                0.,
                if wounded && verification.stage == 0 {
                    if delta.length() < 4. { -0.7 } else { 0. }
                } else if delta.length() > 1.25 {
                    0.4
                } else {
                    0.
                },
            );
            input.fire = false;
            input.fire_held = false;
            input.ads = false;
            input.melee = false;
            input.reload = false;
            return;
        }
        if history.melee_kills == 0 && verification.playing >= 1. {
            input.movement = Vec2::new(0., if delta.length() > 1.25 { 0.4 } else { 0. });
        } else if verification.playing >= 1. && delta.length() < 3. {
            input.movement = Vec2::new(0., -0.7);
        }
        let pitch = delta.y.atan2(Vec2::new(delta.x, delta.z).length());
        let yaw_delta = (yaw - player.yaw + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
            - std::f32::consts::PI;
        input.look =
            Vec2::new(-yaw_delta, player.pitch - pitch) / definition.tuning.mouse_sensitivity;
        if let Ok(inventory) = inventories.single() {
            input.look += inventory.recoil / definition.tuning.mouse_sensitivity;
            let sensitivity = definition.tuning.mouse_sensitivity
                + (definition.tuning.ads_mouse_sensitivity - definition.tuning.mouse_sensitivity)
                    * inventory.ads;
            input.look *= definition.tuning.mouse_sensitivity / sensitivity;
            input.ads = verification.playing >= 1.;
        }
        let origin =
            transform.translation + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
        input.fire = Dir3::new(delta)
            .ok()
            .and_then(|dir| {
                crate::hit_regions::cast(
                    &enemies,
                    &query,
                    entity,
                    origin,
                    dir,
                    delta.length() + 0.3,
                )
            })
            .is_some();
        input.fire &= !verification.hit_regions || wanted.is_none() || aim.is_some();
        input.fire &= history.melee_kills > 0;
        input.ads &= history.melee_kills > 0;
        let struck = history.melee_kills > 0
            || players.get(entity).is_ok() && enemy_evidence.attack_hits > 0;
        input.melee |= delta.length() < 1.9 && struck;
        input.reload = inventories
            .single()
            .ok()
            .and_then(|i| i.active())
            .is_some_and(|w| w.magazine == 0);
    }
}
fn check(
    mut commands: Commands,
    time: Res<Time>,
    real_time: Res<Time<Real>>,
    verification: Option<ResMut<Verification>>,
    state: Res<State<GameState>>,
    failure: Res<LoadFailure>,
    definition: Option<Res<GameDefinition>>,
    combat: (
        Res<Session>,
        Res<crate::damage::CombatHistory>,
        Res<crate::damage::DamageQueue>,
        Res<crate::enemies::EnemyEvidence>,
        Res<crate::rounds::RoundEvidence>,
        Res<crate::player_health::HealthEvidence>,
        Res<crate::hit_regions::HitEvidence>,
        Res<crate::equipment::EquipmentEvidence>,
    ),
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &crate::weapons::WeaponInventory,
            &crate::life::Vitality,
            &crate::player_health::Recovery,
        ),
        Without<Zombie>,
    >,
    mut next: ResMut<NextState<GameState>>,
    enemies: Query<(&Transform, &Zombie), Without<Player>>,
    query: avian3d::prelude::SpatialQuery,
    surfaces: Res<crate::rendering::OpaqueSurfaceStatus>,
    profile: Res<crate::rendering::profile::VisualProfile>,
    mut cameras: Query<
        (&Transform, &mut Projection),
        (
            With<crate::rendering::GameCamera>,
            Without<Player>,
            Without<Zombie>,
        ),
    >,
    mut overlays: Query<
        (&mut Visibility, Option<&crate::hud::Crosshair>),
        With<crate::hud::PlayerOverlay>,
    >,
) {
    let (
        session,
        history,
        impacts,
        enemy_evidence,
        round_evidence,
        health_evidence,
        hit_evidence,
        equipment,
    ) = combat;
    let Some(mut verification) = verification else {
        return;
    };
    for receipt in &history.recent {
        if matches!(
            receipt.impact.category,
            crate::origins::content::DamageCategory::Bullet
        ) {
            verification
                .accepted_regions
                .insert(format!("{:?}", receipt.impact.region));
        }
    }
    let ammo = players
        .iter()
        .next()
        .and_then(|(_, _, _, i, _, _)| i.active())
        .map_or(0, |w| w.magazine);
    if verification.stage == 0 {
        if verification.grenades && equipment.detonations > 0 && !verification.grenade_flash_frame {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(verification.output.join("grenade-blast.png")));
            verification.grenade_flash_frame = true;
        }
        if !verification.enemy_attack_frame
            && enemies.iter().any(|(_, z)| {
                z.action == crate::origins::content::EnemyAnimation::Attack
                    && z.elapsed > 0.2
                    && z.elapsed < 0.75
            })
        {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(verification.output.join("zombie-attack.png")));
            verification.enemy_attack_frame = true;
        }
        if !verification.enemy_death_frame
            && session.kills == 6
            && enemies.iter().any(|(_, z)| {
                z.action == crate::origins::content::EnemyAnimation::Death
                    && z.elapsed > 0.25
                    && z.elapsed < 0.7
            })
        {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(verification.output.join("zombie-death.png")));
            verification.enemy_death_frame = true;
        }
        for (_, _, _, inventory, _, _) in &players {
            if !verification.melee_frame
                && matches!(inventory.action,
                crate::weapons::WeaponAction::Melee { remaining, .. } if remaining < 0.42 && remaining > 0.30)
            {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("knife-melee.png")));
                verification.melee_frame = true;
            }
            verification.recoil_seen |= inventory.recoil.length_squared() > 0.000001;
            if inventory.ads >= 0.99 && !verification.ads_seen {
                verification.ads_seen = true;
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(verification.output.join("ads.png")));
            }
        }
    }
    if verification.benchmark
        && !verification.combat
        && verification.stage == 0
        && **state == GameState::Playing
    {
        // Rendering measurements use an isolated paused survey. Combat remains
        // exercised separately by --verify, at the optimized frame rate.
        verification.stage = 1;
        verification.playing = 0.;
        next.set(GameState::Paused);
    }
    for (mut visibility, crosshair) in &mut overlays {
        if crosshair.is_some() && verification.stage != 1 {
            continue;
        }
        *visibility = if verification.stage == 1 {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
    if verification.started.elapsed().as_secs() > if verification.benchmark { 360 } else { 180 }
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && verification.stage == 0)
    {
        let evidence = serde_json::json!({"passed": false, "state": format!("{:?}", **state),
            "round": session.round, "pending": session.pending, "kills": session.kills,
            "shots": session.shots, "ammo": ammo, "load_failure": failure.0,
            "equipment_events":equipment.events,"player_hits":health_evidence.hits,
            "player_position":players.iter().next().map(|p|p.2.translation.to_array()),
            "elapsed_seconds": verification.started.elapsed().as_secs()});
        let _ = std::fs::write(
            verification.output.join("failure.json"),
            evidence.to_string(),
        );
        error!(
            "Game verification failed: {:?}, round {}, pending {}, kills {}, shots {}, ammo {}, {}",
            **state, session.round, session.pending, session.kills, session.shots, ammo, failure.0
        );
        commands.write_message(AppExit::error());
        return;
    }
    if verification.stage == 2 && **state == GameState::GameOver {
        if (verification.health || verification.grenades) && !verification.death_frame {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(
                    verification.output.join("player-game-over.png"),
                ));
            verification.death_frame = true;
            return;
        }
        if (verification.health || verification.grenades)
            && !verification.output.join("player-game-over.png").is_file()
        {
            return;
        }
        commands.write_message(crate::input::RestartGame);
        verification.stage = 3;
        return;
    }
    if **state != GameState::Playing && !(verification.stage == 1 && **state == GameState::Paused) {
        return;
    }
    verification.playing += time.delta_secs();
    let (Some(definition), Ok((_, mut player, mut transform, _, health, recovery))) =
        (definition, players.single_mut())
    else {
        return;
    };
    if verification.health && verification.stage == 0 {
        if !verification.damage_frame
            && health.current() != Some(health.maximum())
            && recovery.age < 0.3
        {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(verification.output.join("player-hurt.png")));
            verification.damage_frame = true;
        }
        if !verification.healed_frame
            && !health_evidence.heals.is_empty()
            && health.current() == Some(health.maximum())
        {
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(
                    verification.output.join("player-recovered.png"),
                ));
            verification.healed_frame = true;
        }
    }
    if verification.initial.is_none() {
        verification.initial = Some(transform.translation);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("spawn.png")));
    }
    verification.moved |= verification
        .initial
        .is_some_and(|p| p.distance(transform.translation) > 0.1);
    if verification.stage == 0 && verification.playing as u32 / 5 > verification.reported {
        verification.reported = verification.playing as u32 / 5;
        info!(
            "Verification position {:?}, grounded {}, pending {}, living {}, kills {}, shots {}, ammo {}",
            transform.translation,
            player.grounded,
            session.pending,
            enemies.iter().count(),
            session.kills,
            session.shots,
            ammo
        );
        for point in definition.spawn_points.iter().take(3) {
            let position = Vec3::from_array(*point);
            let floor = query.cast_ray(
                position + Vec3::Y,
                Dir3::NEG_Y,
                4.,
                false,
                &avian3d::prelude::SpatialQueryFilter::default(),
            );
            info!(
                "Entry {:?}, floor {:?}",
                point,
                floor.map(|f| (f.distance, f.normal))
            );
        }
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("playing.png")));
    }
    if verification.stage == 0 && session.round >= if verification.rounds { 3 } else { 2 } {
        verification.combat_receipt = serde_json::json!({
            "melee_hits":history.melee_hits,"melee_kills":history.melee_kills,
            "headshot_kills":history.headshot_kills,"rejected_terminal":history.rejected_terminal,
            "awarded_points":history.awarded_points,"balance":session.points,"kills":session.kills,
            "impacts":history.recent.iter().map(|r| serde_json::json!({
                "attacker":r.impact.attacker.to_bits(),"target":r.impact.target.to_bits(),
                "weapon":r.impact.source.id(),"category":format!("{:?}",r.impact.category),
                "region":format!("{:?}",r.impact.region),"damage":r.impact.amount,
                "lethal":r.lethal,"points":r.points,
            })).collect::<Vec<_>>()
        });
        verification.round_two = true;
        verification.stage = 1;
        if verification.wall_weapons {
            verification.stage = 4;
            return;
        }
        verification.playing = 0.;
        next.set(GameState::Paused);
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(verification.output.join("round-two.png")));
    }
    if verification.stage == 1 && verification.playing > 1. {
        let view = visual_view(verification.survey, &definition);
        if let Some((position, yaw, pitch)) = view {
            if let Ok((_, mut projection)) = cameras.single_mut()
                && let Projection::Perspective(perspective) = &mut *projection
            {
                // Comparison-only projection; ordinary player projection is unchanged.
                perspective.fov = profile.comparison_fov_degrees.to_radians();
            }
            let sweep = if verification.moving_view {
                ((verification.playing - 1.) * 0.6).sin()
            } else {
                (verification.playing - 1.).clamp(0., 1.)
            };
            let moving =
                verification.moving_view || verification.survey < definition.survey_views.len();
            transform.translation =
                position + Vec3::new(if moving { sweep * 0.3 } else { 0. }, 0., 0.);
            player.velocity = Vec3::ZERO;
            player.yaw = yaw
                + if moving {
                    if verification.moving_view {
                        if verification.playing >= 60. {
                            ((verification.playing - 60.) / 0.5).floor().min(11.)
                                * std::f32::consts::TAU
                                / 12.
                        } else {
                            (verification.playing - 1.) * 0.5
                        }
                    } else {
                        sweep * 0.2
                    }
                } else {
                    0.
                };
            player.pitch = pitch;
            if verification.moving_view && verification.playing >= 60. {
                transform.translation = position;
                if verification.reported < 12
                    && verification.playing > 60.4 + verification.reported as f32 * 0.5
                {
                    commands
                        .spawn(Screenshot::primary_window())
                        .observe(save_to_disk(
                            verification
                                .output
                                .join(format!("rotation-{}.png", verification.reported)),
                        ));
                    verification.reported += 1;
                }
            }
            let water_view = verification.survey >= definition.survey_views.len() + 5;
            // Allow exposure to settle before judging atmosphere/animation.
            if verification.playing
                > if verification.moving_view {
                    66.
                } else if verification.benchmark {
                    14.
                } else if water_view {
                    8.
                } else {
                    4.
                }
            {
                if let Ok((camera, projection)) = cameras.single_mut()
                    && let Projection::Perspective(perspective) = &*projection
                {
                    let survey = verification.survey;
                    verification.camera_views.push(serde_json::json!({
                        "view": survey, "position": camera.translation.to_array(),
                        "yaw": player.yaw, "pitch": player.pitch, "vertical_fov_degrees": perspective.fov.to_degrees(),
                        "body_position": transform.translation.to_array(), "eye_height": definition.tuning.eye_height,
                        "aspect": perspective.aspect_ratio,
                    }));
                }
                if water_view {
                    verification.water_times.push(real_time.elapsed_secs());
                }
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        verification
                            .output
                            .join(format!("map-{}.png", verification.survey)),
                    ));
                verification.survey += 1;
                verification.playing = 0.;
                if verification.only_view {
                    verification.stage = 2;
                    next.set(if verification.health || verification.grenades {
                        GameState::Playing
                    } else {
                        GameState::GameOver
                    });
                }
            }
        } else {
            verification.stage = 2;
            verification.playing = 0.;
            next.set(if verification.health || verification.grenades {
                GameState::Playing
            } else {
                GameState::GameOver
            });
        }
    }
    if verification.stage == 3 && verification.playing > 0.2 {
        let restart_damage_clear =
            history.recent.is_empty() && history.awarded_points == 0 && impacts.0.is_empty();
        let rounds_exercised = !verification.rounds
            || (round_evidence.transitions >= 2
                && round_evidence
                    .admitted
                    .iter()
                    .filter(|v| v["round"] == 1)
                    .count()
                    == 6
                && round_evidence
                    .admitted
                    .iter()
                    .filter(|v| v["round"] == 2)
                    .count()
                    == 8
                && round_evidence.budget_consumed_on_failure == 0);
        let health_exercised = !verification.health
            || (health_evidence.heals.len() >= 2
                && health_evidence.nonterminal_hits >= 3
                && health_evidence.death_events == 1
                && verification.damage_frame
                && verification.healed_frame
                && verification.death_frame);
        let restart_health_clear = recovery.immunity == 0.
            && !recovery.critical
            && recovery.feedback == 0.
            && health.current() == Some(definition.player_health.maximum);
        let passed = session.round == 1
            && (!verification.semtex || verification.semtex_exercised)
            && (!verification.claymores || verification.claymores_exercised)
            && (!verification.monkeys || verification.monkeys_exercised)
            && (!verification.equipment_refills || verification.equipment_refills_exercised)
            && (!verification.grenades
                || (equipment.thrown >= 1
                    && equipment.cooked_off >= 1
                    && equipment.bounces > 0
                    && equipment.enemy_impacts > 0
                    && equipment.self_impacts > 0
                    && verification.death_frame))
            && (!verification.hit_regions || verification.accepted_regions.len() == 5)
            && health_exercised
            && restart_health_clear
            && rounds_exercised
            && restart_damage_clear
            && surfaces.pending == 0
            && if verification.benchmark {
                (!verification.combat || verification.round_two)
                    && verification.camera_views.len()
                        == if verification.only_view {
                            1
                        } else {
                            definition.survey_views.len() + 7
                        }
            } else {
                verification.round_two && verification.moved
            };
        let mut receipt = serde_json::json!({"passed":passed,
            "scope": if verification.benchmark && !verification.combat { "paused render survey and restart" } else { "combat movement round advance render survey and restart" },
            "continuous_camera_motion": verification.moving_view,
            "round_two":verification.round_two, "movement":verification.moved, "restart_round":session.round,
            "hit_regions_exercised":verification.accepted_regions,
            "semtex_exercised":verification.semtex_exercised,
            "claymores_exercised":verification.claymores_exercised,
            "equipment":{"primed":equipment.primed,"thrown":equipment.thrown,"cooked_off":equipment.cooked_off,
                "flash_frame":verification.grenade_flash_frame,
                "bounces":equipment.bounces,"detonations":equipment.detonations,"obstructed":equipment.obstructed,
                "enemy_impacts":equipment.enemy_impacts,"self_impacts":equipment.self_impacts,
                "surface_attachments":equipment.surface_attachments,"actor_attachments":equipment.actor_attachments,"attached_motion":equipment.attached_motion,"events":equipment.events},
            "hit_volumes":{"bound":hit_evidence.bound_volumes,"pose_updates":hit_evidence.pose_updates,"regions":hit_evidence.regions,"hits":hit_evidence.hits},
            "enemy_attack_frame":verification.enemy_attack_frame,"enemy_death_frame":verification.enemy_death_frame,
            "enemy_presentation": {"variants":enemy_evidence.variants,"animations":enemy_evidence.animations,
                "attached_heads":enemy_evidence.attached_heads,"attacks_started":enemy_evidence.attacks_started,
                "attack_hits":enemy_evidence.attack_hits,"deaths":enemy_evidence.deaths,
                "corpses_removed":enemy_evidence.corpses_removed,"bounds_updates":enemy_evidence.bounds_updates},
            "health_exercised":health_exercised,"restart_health_clear":restart_health_clear,
            "player_damage_frame":verification.damage_frame,"player_healed_frame":verification.healed_frame,"player_death_frame":verification.death_frame,
            "player_health":{"hits":health_evidence.hits,"heals":health_evidence.heals,
                "nonterminal_hits":health_evidence.nonterminal_hits,"feedback_events":health_evidence.feedback_events,
                "immunity_rejections":health_evidence.immunity_rejections,"terminal_rejections":health_evidence.terminal_rejections,
                "death_events":health_evidence.death_events},
            "rounds_exercised":rounds_exercised,
            "round_profile":(1..=255).map(|r| serde_json::json!({"round":r,"health":definition.rounds.health(r),
                "population":definition.rounds.population(r),"spawn_delay":definition.rounds.spawn_delay(r)})).collect::<Vec<_>>(),
            "round_admissions":round_evidence.admitted,"round_attempts":round_evidence.attempts,
            "failed_admissions":round_evidence.failed_admissions,"budget_consumed_on_failure":round_evidence.budget_consumed_on_failure,
            "peak_ordinary":round_evidence.peak_ordinary,"cap_waits":round_evidence.cap_waits,
            "round_transitions":round_evidence.transitions,"between_round_seconds":round_evidence.intermission_seconds,
            "combat_rewards":verification.combat_receipt,"melee_frame_captured":verification.melee_frame,
            "restart_damage_clear":restart_damage_clear,
            "ads_reached": verification.ads_seen, "recoil_observed": verification.recoil_seen,
            "source_surfaces":definition.source_surfaces, "static_instances":definition.static_model_instances,
            "survey_views":verification.camera_views.len(), "water_animation_seconds":verification.water_times,
            "camera_views":verification.camera_views, "opaque_materials": &*surfaces});
        receipt["monkeys_exercised"] = verification.monkeys_exercised.into();
        receipt["equipment_refills_exercised"] = verification.equipment_refills_exercised.into();
        let _ = std::fs::write(
            verification.output.join("verification.json"),
            receipt.to_string(),
        );
        info!("Game verification: {receipt}");
        commands.write_message(if passed {
            AppExit::Success
        } else {
            AppExit::error()
        });
    }
}

// Additional actual-map views target the supplied spawn-room references and a
// retained puddle. The repeated water view checks animation at a stationary camera.
fn visual_view(index: usize, definition: &GameDefinition) -> Option<(Vec3, f32, f32)> {
    if let Some(position) = definition.survey_views.get(index) {
        return Some((
            Vec3::from_array(*position) + Vec3::Y * 1.8,
            index as f32,
            0.,
        ));
    }
    let local = index - definition.survey_views.len();
    let spawn = Vec3::from_array(definition.spawn) + Vec3::new(0., 0.2, 1.);
    match local {
        0..=3 => Some((
            spawn,
            definition.yaw + local as f32 * std::f32::consts::FRAC_PI_2,
            0.05,
        )),
        4 => Some((spawn, definition.yaw, 0.7)),
        5..=6 => Some((Vec3::new(-38.7, 4.5, -13.), 0., -0.6)),
        _ => None,
    }
}
