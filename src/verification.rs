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
mod ads;
mod arsenal;
mod audio;
mod barriers;
mod brushes;
mod claymores;
mod controls;
mod defensive_perks;
mod doors;
mod equipment_presentation;
mod equipment_refills;
mod firing_perks;
mod generators;
mod health_recovery;
mod locomotion;
mod mobility;
mod monkeys;
mod mud;
mod mule_kick;
mod performance;
mod perk_health;
mod pipelines;
mod placements;
mod release_performance;
pub(crate) use pipelines::error_layer;
mod semtex;
mod solo_revive;
mod speed_cola;
mod stamin_up;
mod texture_streaming;
mod triggers;
mod viewmodel_shadows;
mod weapon_assemblies;

#[derive(Resource)]
pub(crate) struct Verification {
    output: PathBuf,
    benchmark: bool,
    combat: bool,
    only_view: bool,
    moving_view: bool,
    hands_view: bool,
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
    weapon_assemblies: bool,
    ads: bool,
    rounds: bool,
    health: bool,
    solo_revive: bool,
    solo_bleedout: bool,
    perk_health: bool,
    speed_cola: bool,
    stamin_up: bool,
    mule_kick: bool,
    firing_perks: bool,
    defensive_perks: bool,
    mobility: bool,
    mud: bool,
    doors: bool,
    navigation: bool,
    traversal: bool,
    audio: bool,
    barriers: bool,
    generators: bool,
    admission_snapshot: bool,
    health_recovery_exercised: bool,
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
    placement_snapshot: Option<placements::Snapshot>,
    trigger_observation: Option<serde_json::Value>,
    brush_observation: Option<serde_json::Value>,
    negotiation_observation: Option<serde_json::Value>,
}
pub struct VerificationPlugin;
impl Plugin for VerificationPlugin {
    fn build(&self, app: &mut App) {
        if app.world().resource::<LaunchOptions>().verify_locomotion {
            pipelines::install(app);
            locomotion::install(app);
            return;
        }
        if app
            .world()
            .resource::<LaunchOptions>()
            .verify_viewmodel_shadows
            || app.world().resource::<LaunchOptions>().verify_actor_shadows
        {
            pipelines::install(app);
            viewmodel_shadows::install(app);
            return;
        }
        if app
            .world()
            .resource::<LaunchOptions>()
            .verify_texture_streaming
        {
            pipelines::install(app);
            texture_streaming::install(app);
            return;
        }
        if app.world().resource::<LaunchOptions>().release_benchmark {
            pipelines::install(app);
            release_performance::install(app);
            return;
        }
        if app.world().resource::<LaunchOptions>().verify_controls {
            pipelines::install(app);
            controls::install(app);
            return;
        }
        if app.world().resource::<LaunchOptions>().verify.is_some() {
            if app
                .world()
                .resource::<LaunchOptions>()
                .verify_weapon_assemblies
            {
                weapon_assemblies::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_ads {
                ads::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_generators {
                generators::install(app);
            }
            audio::install(app);
            pipelines::install(app);
            if app.world().resource::<LaunchOptions>().verify_barriers {
                barriers::install(app);
            }
            if app
                .world()
                .resource::<LaunchOptions>()
                .verify_defensive_perks
            {
                defensive_perks::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_firing_perks {
                firing_perks::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_mule_kick {
                mule_kick::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_stamin_up {
                stamin_up::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_speed_cola {
                speed_cola::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_perk_health {
                perk_health::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_mobility {
                mobility::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_mud {
                mud::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_doors {
                doors::install(app);
            }
            equipment_presentation::install(app);
            if app.world().resource::<LaunchOptions>().verify_solo_revive {
                solo_revive::install(app);
            }
            if app.world().resource::<LaunchOptions>().verify_health {
                health_recovery::install(app);
            }
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
            .add_systems(
                Last,
                performance::record.after(bevy::diagnostic::update_frame_count),
            );
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
            hands_view: options.benchmark_hands,
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
            weapon_assemblies: options.verify_weapon_assemblies,
            ads: options.verify_ads,
            equipment_refills: options.verify_equipment_refills,
            equipment_refills_exercised: false,
            rounds: options.verify_rounds,
            health: options.verify_health,
            solo_revive: options.verify_solo_revive,
            solo_bleedout: options.verify_solo_bleedout,
            perk_health: options.verify_perk_health,
            speed_cola: options.verify_speed_cola,
            stamin_up: options.verify_stamin_up,
            mule_kick: options.verify_mule_kick,
            firing_perks: options.verify_firing_perks,
            defensive_perks: options.verify_defensive_perks,
            mobility: options.verify_mobility,
            mud: options.verify_mud,
            doors: options.verify_doors,
            navigation: options.verify_navigation,
            traversal: options.verify_traversal,
            audio: options.verify_audio,
            barriers: options.verify_barriers,
            generators: options.verify_generators,
            admission_snapshot: false,
            health_recovery_exercised: false,
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
            placement_snapshot: None,
            trigger_observation: None,
            brush_observation: None,
            negotiation_observation: None,
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
    if verification.weapon_assemblies {
        return;
    }
    if verification.ads
        || verification.solo_revive
        || verification.perk_health
        || verification.speed_cola
        || verification.stamin_up
        || verification.mule_kick
        || verification.firing_perks
        || verification.defensive_perks
        || verification.mobility
        || verification.mud
        || verification.doors
        || verification.audio
        || verification.barriers
        || verification.generators
    {
        return;
    }
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
        if verification.stage == 0 && equipment.primed == 0 && verification.playing < 2. {
            input.clear();
            return;
        }
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
                                    &crate::environment::solids([entity]),
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
        let empty = inventories.single().is_ok_and(|inventory| {
            inventory.active().is_some_and(|w| w.magazine == 0)
                && inventory
                    .definition(&definition.content.weapons)
                    .is_some_and(|w| inventory.reserve(w) == 0)
        });
        if (history.melee_kills == 0 || empty) && verification.playing >= 1. {
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
            input.ads = !verification.ads_seen && verification.playing >= 1.;
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
    } else if verification.stage == 0
        && let Some(initial) = verification.initial
    {
        // Leave a cleared entrance so its minimum spawn clearance can recover.
        let delta = initial - transform.translation;
        if delta.xz().length_squared() > 1. {
            let local = Quat::from_rotation_y(-player.yaw) * delta.normalize_or_zero();
            input.movement = Vec2::new(local.x, -local.z).clamp_length_max(0.5);
        }
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
        Res<crate::presentation::EquipmentPresentationEvidence>,
        Res<crate::navigation::Navigation>,
        Res<crate::access::ZoneAccess>,
        Res<crate::barriers::Barriers>,
    ),
    map_sites: (
        placements::Sites,
        crate::environment::triggers::TriggerSites,
        crate::environment::brushes::BrushSites,
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
    render_status: (
        Res<crate::rendering::OpaqueSurfaceStatus>,
        Res<crate::rendering::TextureStreamingStats>,
        Res<crate::rendering::profile::VisualProfile>,
    ),
    mut cameras: Query<
        (&Transform, &mut Projection),
        (
            With<crate::rendering::GameCamera>,
            Without<Player>,
            Without<Zombie>,
        ),
    >,
    mut overlays: Query<
        (
            &mut Visibility,
            Option<&crate::hud::Crosshair>,
            Option<&crate::presentation::ViewModelGroup>,
        ),
        With<crate::hud::PlayerOverlay>,
    >,
) {
    let mut screenshot = None;
    let (sites, trigger_sites, brush_sites) = map_sites;
    let (surfaces, streaming, profile) = render_status;
    let (
        session,
        history,
        impacts,
        enemy_evidence,
        round_evidence,
        health_evidence,
        hit_evidence,
        equipment,
        equipment_presentation,
        navigation,
        access,
        barriers,
    ) = combat;
    let Some(mut verification) = verification else {
        return;
    };
    if verification.weapon_assemblies
        || verification.ads
        || verification.solo_revive
        || verification.perk_health
        || verification.speed_cola
        || verification.stamin_up
        || verification.mule_kick
        || verification.firing_perks
        || verification.defensive_perks
        || verification.mobility
        || verification.mud
        || verification.doors
        || verification.audio
        || verification.barriers
        || verification.generators
    {
        return;
    }
    if verification.placement_snapshot.is_none()
        && **state == GameState::Playing
        && let Some(game) = &definition
    {
        match placements::snapshot(game, &sites) {
            Ok(snapshot) => verification.placement_snapshot = Some(snapshot),
            Err(error) => {
                let _ = std::fs::write(
                    verification.output.join("placement-failure.json"),
                    serde_json::json!({"passed":false,"error":error}).to_string(),
                );
                error!("Game verification failed: {error}");
                commands.write_message(AppExit::error());
                return;
            }
        }
    }
    if verification.negotiation_observation.is_none()
        && navigation.link_count() > 0
        && let Some(game) = &definition
    {
        let observed = navigation.negotiation_observation(game, &access);
        let _ = std::fs::write(
            verification.output.join("traversal-links.json"),
            observed.to_string(),
        );
        if observed["passed"] != true {
            error!("Game verification failed: negotiation closure: {observed}");
            commands.write_message(AppExit::error());
            return;
        }
        verification.negotiation_observation = Some(observed);
    }
    if verification.trigger_observation.is_none()
        && matches!(**state, GameState::Playing | GameState::Paused)
        && verification.playing > 0.5
        && let Some(game) = &definition
    {
        let observed = triggers::observe(game, &trigger_sites, &query);
        let _ = std::fs::write(
            verification.output.join("trigger-volumes.json"),
            observed.to_string(),
        );
        if observed["passed"] != true {
            error!("Game verification failed: trigger volume queries: {observed}");
            commands.write_message(AppExit::error());
            return;
        }
        verification.trigger_observation = Some(observed);
    }
    if verification.brush_observation.is_none()
        && matches!(**state, GameState::Playing | GameState::Paused)
        && verification.playing > 0.5
        && let Some(game) = &definition
    {
        let observed = brushes::observe(game, &brush_sites, &query);
        let _ = std::fs::write(
            verification.output.join("brush-entities.json"),
            observed.to_string(),
        );
        if observed["passed"] != true {
            error!("Game verification failed: brush entities: {observed}");
            commands.write_message(AppExit::error());
            return;
        }
        verification.brush_observation = Some(observed);
    }
    if !verification.benchmark
        && !verification.admission_snapshot
        && verification.started.elapsed().as_secs() > 30
    {
        verification.admission_snapshot = true;
        let _=std::fs::write(verification.output.join("spawn-admission.json"),serde_json::json!({
            "pending":session.pending,"attempts":round_evidence.attempts,"failed":round_evidence.failed_admissions,"nodes":navigation.node_count(),"links":navigation.link_count(),
            "reachable":access.reachable,"purchases":access.purchases,"admissions":navigation.admissions.iter().map(|a|serde_json::json!({"position":a.position.to_array(),"source":a.source,"zone":a.zone})).collect::<Vec<_>>(),
            "barriers":{"windows":barriers.windows.len(),"destroyed":barriers.destroyed,"entries":barriers.entries},"actors":enemies.iter().map(|(t,z)|serde_json::json!({"position":t.translation.to_array(),"action":format!("{:?}",z.action)})).collect::<Vec<_>>()
        }).to_string());
    }
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
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("grenade-blast.png"),
            );
            verification.grenade_flash_frame = true;
        }
        if !verification.enemy_attack_frame
            && enemies.iter().any(|(_, z)| {
                z.action == crate::origins::content::EnemyAnimation::Attack
                    && z.elapsed > 0.2
                    && z.elapsed < 0.75
            })
        {
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("zombie-attack.png"),
            );
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
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("zombie-death.png"),
            );
            verification.enemy_death_frame = true;
        }
        for (_, _, _, inventory, _, _) in &players {
            if !verification.melee_frame
                && matches!(inventory.action,
                crate::weapons::WeaponAction::Melee { remaining, .. } if remaining < 0.42 && remaining > 0.30)
            {
                capture(
                    &mut commands,
                    &mut screenshot,
                    verification.output.join("knife-melee.png"),
                );
                verification.melee_frame = true;
            }
            verification.recoil_seen |= inventory.recoil.length_squared() > 0.000001;
            if inventory.ads >= 0.99 && !verification.ads_seen {
                verification.ads_seen = true;
                capture(
                    &mut commands,
                    &mut screenshot,
                    verification.output.join("ads.png"),
                );
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
    for (mut visibility, crosshair, viewmodel) in &mut overlays {
        if crosshair.is_some() && verification.stage != 1 {
            continue;
        }
        *visibility =
            if verification.stage == 1 && !(verification.hands_view && viewmodel.is_some()) {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            };
    }
    // Allocation-instrumented captures include loading and full combat before
    // their sampled window. Keep a bounded deadline without discarding a slow
    // but advancing workload before the round transition and restart receipts.
    if verification.started.elapsed().as_secs() > if verification.benchmark { 900 } else { 180 }
        || **state == GameState::LoadError
        || (**state == GameState::GameOver && verification.stage == 0)
    {
        let evidence = serde_json::json!({"passed": false, "state": format!("{:?}", **state),
            "round": session.round, "pending": session.pending, "kills": session.kills,
            "shots": session.shots, "ammo": ammo, "load_failure": failure.0,
            "equipment_events":equipment.events,"player_hits":health_evidence.hits,
            "player_position":players.iter().next().map(|p|p.2.translation.to_array()),
            "elapsed_seconds": verification.started.elapsed().as_secs(),"navigation":{"nodes":navigation.node_count(),"links":navigation.link_count(),"admissions":navigation.admissions.len()},"failed_admissions":round_evidence.failed_admissions});
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
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("player-game-over.png"),
            );
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
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("player-hurt.png"),
            );
            verification.damage_frame = true;
        }
        if !verification.healed_frame
            && !health_evidence.heals.is_empty()
            && health.current() == Some(health.maximum())
        {
            capture(
                &mut commands,
                &mut screenshot,
                verification.output.join("player-recovered.png"),
            );
            verification.healed_frame = true;
        }
    }
    if verification.initial.is_none() {
        verification.initial = Some(transform.translation);
        capture(
            &mut commands,
            &mut screenshot,
            verification.output.join("spawn.png"),
        );
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
                &crate::environment::solids([]),
            );
            info!(
                "Entry {:?}, floor {:?}",
                point,
                floor.map(|f| (f.distance, f.normal))
            );
        }
        capture(
            &mut commands,
            &mut screenshot,
            verification.output.join("playing.png"),
        );
    }
    if verification.stage == 0 && session.round >= if verification.rounds { 3 } else { 2 } {
        verification.combat_receipt = serde_json::json!({
            "melee_hits":history.melee_hits,"melee_kills":history.melee_kills,
            "headshot_kills":history.headshot_kills,"rejected_terminal":history.rejected_terminal,
            "awarded_points":history.awarded_points,"balance":session.points,"kills":session.kills,
            "barriers":{"destroyed":barriers.destroyed,"entries":barriers.entries,"sounds":barriers.playing_sounds},
            "impacts":history.recent.iter().map(|r| serde_json::json!({
                "attacker":r.impact.attacker.to_bits(),"target":r.impact.target.to_bits(),
                "weapon":r.impact.source.id(),"category":format!("{:?}",r.impact.category),
                "region":format!("{:?}",r.impact.region),"damage":r.impact.amount,
                "lethal":r.lethal,"points":r.points,
            })).collect::<Vec<_>>()
        });
        verification.round_two = true;
        verification.stage = 1;
        if verification.wall_weapons
            || (verification.health && !verification.health_recovery_exercised)
        {
            verification.stage = 4;
            return;
        }
        verification.playing = 0.;
        next.set(GameState::Paused);
        capture(
            &mut commands,
            &mut screenshot,
            verification.output.join("round-two.png"),
        );
    }
    if verification.stage == 1 && verification.playing > 1. {
        let view = visual_view(
            verification.survey,
            &definition,
            verification.only_view,
            !verification.benchmark,
        );
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
                    capture(
                        &mut commands,
                        &mut screenshot,
                        verification
                            .output
                            .join(format!("rotation-{}.png", verification.reported)),
                    );
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
                capture(
                    &mut commands,
                    &mut screenshot,
                    verification
                        .output
                        .join(format!("map-{}.png", verification.survey)),
                );
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
            || (verification.health_recovery_exercised
                && health_evidence.heals.len() >= 2
                && health_evidence.nonterminal_hits >= 3
                && health_evidence.death_events == 1
                && verification.damage_frame
                && verification.healed_frame
                && verification.death_frame);
        let restart_health_clear = recovery.immunity == 0.
            && !recovery.critical
            && recovery.feedback == 0.
            && health.current() == Some(definition.player_health.maximum);
        let restarted_sites = placements::snapshot(&definition, &sites);
        let placement_ids_preserved =
            verification
                .placement_snapshot
                .as_ref()
                .is_some_and(|before| {
                    restarted_sites
                        .as_ref()
                        .is_ok_and(|after| placements::restart_matches(before, after))
                });
        let restarted_triggers = triggers::observe(&definition, &trigger_sites, &query);
        let restarted_brushes = brushes::observe(&definition, &brush_sites, &query);
        let restarted_negotiations = navigation.negotiation_observation(&definition, &access);
        let passed = session.round == 1
            && restarted_negotiations["passed"] == true
            && verification
                .negotiation_observation
                .as_ref()
                .is_some_and(|o| o["passed"] == true)
            && placement_ids_preserved
            && restarted_triggers["passed"] == true
            && restarted_brushes["passed"] == true
            && verification
                .brush_observation
                .as_ref()
                .is_some_and(|o| o["passed"] == true)
            && verification
                .trigger_observation
                .as_ref()
                .is_some_and(|o| o["passed"] == true)
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
            && surfaces.straight_alpha_mismatches == 0
            && streaming.errors == 0
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
            "enemy_presentation": {"variants":enemy_evidence.variants,"animations":enemy_evidence.animations,"locomotion_clips":enemy_evidence.locomotion_clips,
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
        receipt["enemy_bounds"] = serde_json::json!({
            "native_frames": enemy_evidence.native_bounds_frames,
            "fallback_frames": enemy_evidence.fallback_bounds_frames,
        });
        receipt["navigation"] = serde_json::json!({"nodes":navigation.node_count(),"links":navigation.link_count(),"rejected_nodes":navigation.rejected_nodes,"rejected_links":navigation.rejected_links,"requests":navigation.requests,"cache_hits":navigation.cache_hits,"fallbacks":navigation.fallbacks,"admissions":navigation.admissions.len()});
        receipt["placement_identities"] = serde_json::json!({
            "preserved_on_restart":placement_ids_preserved,
            "initial":verification.placement_snapshot.as_ref().map(placements::receipt),
            "restarted":restarted_sites.as_ref().ok().map(placements::receipt),
            "restart_error":restarted_sites.err(),
        });
        receipt["trigger_volumes"] = serde_json::json!({"initial": verification.trigger_observation,
            "restarted":restarted_triggers});
        receipt["brush_entities"] = serde_json::json!({"initial": verification.brush_observation,
            "restarted":restarted_brushes});
        receipt["traversal_links"] = serde_json::json!({"initial": verification.negotiation_observation,
            "restarted":restarted_negotiations});
        receipt["texture_streaming"] =
            serde_json::to_value(&*streaming).expect("Serializable streaming counters");
        receipt["shared_lighting"] = serde_json::json!({
            "method": "native_raster", "ray_tracing": false,
            "indirect": "atmosphere_environment_and_ssao",
        });
        receipt["enemy_traversal"] = serde_json::json!({"started":enemy_evidence.traversals_started,"finished":enemy_evidence.traversals_finished,"rising_finished":enemy_evidence.rising_finished,"blocked_landings":enemy_evidence.traversal_blocked_landings,"adjusted_landings":enemy_evidence.adjusted_landings});
        receipt["enemy_navigation"] = serde_json::json!({"replans":enemy_evidence.replans,"target_replans":enemy_evidence.target_replans,"blocker_replans":enemy_evidence.blocker_replans,"direct_pursuits":enemy_evidence.direct_pursuits,"stuck_recoveries":enemy_evidence.stuck_recoveries,"separations":enemy_evidence.separations});
        receipt["monkeys_exercised"] = verification.monkeys_exercised.into();
        receipt["equipment_refills_exercised"] = verification.equipment_refills_exercised.into();
        receipt["equipment_presentation"] = serde_json::json!({"phases":equipment_presentation.phases,
            "motion":equipment_presentation.motion,"frames":equipment_presentation.frames,"phase_frames":equipment_presentation.phase_frames,"views":equipment_presentation.views,"clocks":equipment_presentation.clocks,"sampled_clocks":equipment_presentation.sampled_clocks});
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
fn capture(commands: &mut Commands, frame: &mut Option<Entity>, path: PathBuf) {
    // All labels requested during this check observe the same rendered frame.
    // Share its native capture instead of submitting duplicate window targets,
    // which Bevy drops before either observer can save the skipped request.
    let entity = *frame.get_or_insert_with(|| commands.spawn(Screenshot::primary_window()).id());
    commands.entity(entity).observe(save_to_disk(path));
}

fn visual_view(
    index: usize,
    definition: &GameDefinition,
    only_view: bool,
    brush_views: bool,
) -> Option<(Vec3, f32, f32)> {
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
        7..=8 if brush_views => brush_view(definition, local == 8),
        10..=11 if only_view => brush_view(definition, local == 11),
        // Opt-in close views of probe-only radios beside authored lanterns.
        // Additional ordinary views above qualify authored brush placements.
        7 if only_view => Some((Vec3::new(72.0, -9.0, -130.0), 3.75, 0.1)),
        8 if only_view => Some((Vec3::new(-7.5, 1.8, -4.0), 1.92, -0.28)),
        // The reported thin-wall leak is beside the original Quick Revive
        // placement. Reuse the survey's settling/capture path at its real pose.
        9 if only_view => {
            let placement = definition
                .content
                .placements
                .iter()
                .find(|placement| placement.id.0 == "placement.quick_revive")?;
            let machine = Vec3::from_array(placement.position);
            let rotation = Quat::from_rotation_y(placement.yaw);
            let position = machine
                + rotation * Vec3::new(0., definition.tuning.standing_height * 0.5 + 0.02, 2.2);
            let target = machine + Vec3::Y + rotation * Vec3::new(0., 0., 0.8);
            let delta = target
                - (position
                    + Vec3::Y
                        * (definition.tuning.eye_height - definition.tuning.standing_height * 0.5));
            Some((
                position,
                (-delta.x).atan2(-delta.z),
                (delta.y / delta.length()).asin(),
            ))
        }
        _ => None,
    }
}

fn brush_view(definition: &GameDefinition, ice: bool) -> Option<(Vec3, f32, f32)> {
    let source = definition.environment.brush_entities.iter().find(|b| {
        if ice {
            b.source
                .get("script_noteworthy")
                .is_some_and(|n| n == "ice_ceiling_tile")
        } else {
            b.source
                .get("targetname")
                .is_some_and(|n| n == "chamber_entrance")
        }
    })?;
    let origin = Vec3::from_array(source.position);
    let (position, target) = if ice {
        (origin + Vec3::new(0., -2.5, 3.), origin)
    } else {
        // Native chamber_entrance_position is (-339, 110.5, 69.5) inches/Z-up.
        // Stand beside the authored gramophone site, outside the entrance brush.
        (Vec3::new(-8.6106, 2.3, -2.8067), origin + Vec3::Y * 3.1)
    };
    let delta = target
        - (position
            + Vec3::Y * (definition.tuning.eye_height - definition.tuning.standing_height * 0.5));
    Some((
        position,
        (-delta.x).atan2(-delta.z),
        (delta.y / delta.length()).asin(),
    ))
}
