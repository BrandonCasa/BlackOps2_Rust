use crate::{
    GameState,
    environment::support::{EnvironmentOwner, EnvironmentalHit, ShootableEnvironment},
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    interactions::InteractionFocus,
    life::{LifeState, Vitality},
    player::Player,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
pub(super) mod source;
#[derive(Resource, Default)]
pub struct StaffPuzzles {
    pub first: [bool; 4],
    pub second: [bool; 4],
    pub chords: usize,
    pub rejected: u32,
    pub relays: BTreeMap<String, u8>,
    pub wind_rings: [u8; 4],
    pub smoke: [bool; 3],
    pub ice_value: Option<u8>,
    pub ice_remaining: BTreeSet<u8>,
    pub ice_tiles: BTreeSet<u8>,
    pub frozen_stones: [bool; 3],
    pub broken_stones: [bool; 3],
    pub sacrifices: u32,
    pub fire_torches: BTreeSet<u8>,
    pub lit_torches: BTreeMap<u8, f32>,
    fire_corpses: Vec<FireCorpse>,
    ignition: f32,
    ice_lock: f32,
    notes: BTreeSet<String>,
    ringing: f32,
    latched: bool,
    note_sounds: Vec<Entity>,
    note_fx: Vec<Entity>,
    smoke_cleanup: f32,
    corpse_poll: f32,
    torch_cooldown: BTreeMap<u8, f32>,
    torch_sounds: BTreeMap<u8, (Entity, Entity, Entity)>,
    floor_fx: Option<(Entity, Entity)>,
    plinths: BTreeMap<usize, (Entity, Entity, Entity)>,
    plinth_cleanup: f32,
    celebration: f32,
    celebration_next: usize,
    celebration_order: Vec<u8>,
    torch_out: BTreeSet<u8>,
    torches_lit: u32,
    ice_pending: bool,
    snow: f32,
    wind_cleanup: f32,
}
#[derive(Component)]
struct PianoKey(String);
#[derive(Component)]
pub(crate) struct Relay(String);
#[derive(Component)]
pub(crate) struct RelayState {
    base: Quat,
    from: Quat,
    to: Quat,
    remaining: f32,
    power: u8,
    sound: Option<Entity>,
    fx: Option<Entity>,
}
#[derive(Component)]
pub(crate) struct PianoPlaying;
#[derive(Component)]
struct DelayedCue {
    alias: String,
    remaining: f32,
    position: Vec3,
}
struct FireCorpse {
    entity: Entity,
    pose: Transform,
    remaining: f32,
}
#[derive(Component)]
struct PuzzleProgress {
    player: Entity,
    remaining: f32,
}
fn puzzle_good(commands: &mut Commands, player: Entity) {
    commands.spawn((
        SessionEntity,
        PuzzleProgress {
            player,
            remaining: 1.,
        },
    ));
}
fn stage_feedback(commands: &mut Commands, element: usize, stage: usize, position: Vec3) {
    crate::audio::emit(
        commands,
        if stage == 0 {
            "zmb_squest_step1_finished"
        } else {
            "zmb_squest_step2_finished"
        },
        crate::audio::SoundSource::World(position),
    );
    let family = ["fire", "ice", "wind", "lightning"][element];
    let line = if element == 2 { [1, 0][stage] } else { stage };
    commands.spawn((
        SessionEntity,
        DelayedCue {
            alias: format!("vox_sam_{family}_puz_solve_{line}"),
            remaining: 4.,
            position,
        },
    ));
}
fn delayed_cues(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut cues: Query<(Entity, &mut DelayedCue)>,
    mut progress: Query<(Entity, &mut PuzzleProgress)>,
    mut narrative: MessageWriter<crate::audio::NarrativeProgress>,
) {
    for (entity, mut cue) in &mut cues {
        cue.remaining -= time.delta_secs();
        if cue.remaining <= 0. {
            crate::audio::emit(
                &mut commands,
                cue.alias.clone(),
                crate::audio::SoundSource::World(cue.position),
            );
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut event) in &mut progress {
        event.remaining -= time.delta_secs();
        if event.remaining <= 0. {
            narrative.write(crate::audio::NarrativeProgress {
                player: event.player,
            });
            commands.entity(entity).despawn();
        }
    }
}
fn native_rotation(pitch: f32, yaw: f32, roll: f32) -> Quat {
    Quat::from_rotation_y(yaw.to_radians())
        * Quat::from_rotation_z(-pitch.to_radians())
        * Quat::from_rotation_x(roll.to_radians())
}

pub struct StaffPuzzlePlugin;
impl Plugin for StaffPuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaffPuzzles>()
            .add_systems(
                FixedUpdate,
                delayed_cues
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                OnEnter(GameState::StartingSession),
                (setup, setup_ice, setup_fire)
                    .chain()
                    .in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                fire.in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                ice.in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                wind.in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                piano
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                relay
                    .in_set(GameStep::Interactions)
                    .after(crate::rituals::claim)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut puzzles: ResMut<StaffPuzzles>,
    mut session: ResMut<crate::gameplay::Session>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    props: Query<(Entity, &Name), With<crate::assets::StaticPropRoot>>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
) {
    *puzzles = default();
    for (entity, name) in &props {
        if name.as_str().contains("p6_zm_tm_wind_ceiling_ring") {
            commands.entity(entity).insert(Visibility::Hidden);
        }
    }
    for (index, target) in [0_u8, 3, 2, 3].into_iter().enumerate() {
        puzzles.wind_rings[index] = (target + 1 + session.random() as u8 % 3) % 4;
        let radius = 2.286 + index as f32 * 1.524;
        let shape = Collider::compound(
            (0..32)
                .map(|segment| {
                    let angle = segment as f32 * std::f32::consts::TAU / 32.;
                    (
                        Vec3::new(angle.sin() * radius, 12.5, angle.cos() * radius),
                        Quat::from_rotation_y(angle),
                        Collider::cuboid(
                            2. * (std::f32::consts::PI / 32.).tan() * (radius + 0.725),
                            1.05,
                            1.45,
                        ),
                    )
                })
                .collect(),
        );
        let model = format!("p6_zm_tm_wind_ceiling_ring_{}_lod0", index + 1);
        let mut entity = commands.spawn((
            SessionEntity,
            WindRing {
                index,
                remaining: 0.,
                from: 0.,
                to: puzzles.wind_rings[index] as f32 * std::f32::consts::FRAC_PI_2,
            },
            Transform::from_translation(Vec3::new(285.877, -14.2748, 220.0402)).with_rotation(
                Quat::from_rotation_y(
                    puzzles.wind_rings[index] as f32 * std::f32::consts::FRAC_PI_2,
                ),
            ),
            shape,
            Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
            EnvironmentOwner::Quest,
            ShootableEnvironment,
        ));
        if let Some(scene) = game
            .content
            .presentations
            .iter()
            .filter_map(|p| p.scene.as_ref())
            .find(|s| s.contains(&model))
        {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    for (index, target) in ["pf2264_auto2", "pf2265_auto2", "pf2266_auto2"]
        .into_iter()
        .enumerate()
    {
        for (entity, brush) in &brushes {
            if game
                .environment
                .brush_entities
                .iter()
                .find(|b| b.id == brush.id)
                .is_some_and(|b| b.source.get("targetname").is_some_and(|n| n == target))
            {
                commands
                    .entity(entity)
                    .insert((
                        Smoke {
                            index,
                            remaining: 0.,
                            direction: Vec3::ZERO,
                            attacker: None,
                        },
                        EnvironmentOwner::Quest,
                        ShootableEnvironment,
                    ))
                    .remove::<ColliderDisabled>();
            }
        }
    }
    for (index, point) in [
        [-48.1711, 4.826, -0.2032],
        [61.1886, 4.9149, -13.4366],
        [-22.9997, 4.6482, 29.6291],
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            SessionEntity,
            SmokeEmitter {
                index,
                fx: None,
                sound: None,
            },
            Transform::from_translation(Vec3::from_array(point))
                .with_rotation(native_rotation(0., 240., 0.)),
            Visibility::Hidden,
        ));
    }
    for (name, value) in [
        ("bunker", 2),
        ("tank_platform", 1),
        ("start", 3),
        ("elec", 1),
        ("ruins", 3),
        ("air", 0),
        ("ice", 1),
        ("village", 1),
    ] {
        puzzles.relays.insert(name.into(), value);
    }
    for (note, point) in source::KEYS {
        commands.spawn((
            SessionEntity,
            PianoKey((*note).into()),
            Transform::from_translation(Vec3::from_array(*point)),
        ));
    }
    for ((name, point), yaw) in source::RELAYS
        .iter()
        .zip([0_f32, 0., 282.6, 90., 270., 0., 14.5, 180.])
    {
        let base = native_rotation(0., yaw, 0.);
        let rotation = base
            * Quat::from_rotation_z(-(puzzles.relays[*name] as f32) * std::f32::consts::FRAC_PI_2);
        let mut entity = commands.spawn((
            SessionEntity,
            Relay((*name).into()),
            RelayState {
                base,
                from: rotation,
                to: rotation,
                remaining: 0.,
                power: 0,
                sound: None,
                fx: None,
            },
            Transform::from_translation(Vec3::from_array(*point)).with_rotation(rotation),
        ));
        if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_rotary_switch") {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
}

fn piano(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut puzzles: ResMut<StaffPuzzles>,
    mut impacts: MessageReader<crate::projectiles::ProjectileImpact>,
    keys: Query<(Entity, &PianoKey, &Transform), Without<Relay>>,
    players: Query<(Entity, &Transform, &WeaponInventory), (With<Player>, Without<Relay>)>,
    mut relays: Query<(Entity, &Relay, &mut RelayState, &mut Transform), Without<Player>>,
) {
    let previous = puzzles.ringing;
    puzzles.ringing = (previous - time.delta_secs()).max(0.);
    if previous > 0. && puzzles.ringing == 0. {
        for sound in puzzles.note_sounds.drain(..) {
            commands.entity(sound).try_despawn();
        }
        for fx in puzzles.note_fx.drain(..) {
            commands.entity(fx).try_despawn();
        }
        for (entity, _, _) in &keys {
            commands.entity(entity).remove::<PianoPlaying>();
        }
        puzzles.notes.clear();
        if puzzles.chords == 3 {
            puzzles.first[3] = true;
            stage_feedback(&mut commands, 3, 0, Vec3::ZERO);
        }
    }
    let occupied = players.iter().any(|(_, p, _)| {
        (p.translation - Vec3::new(262.636, -7.9756, 200.8124))
            .xz()
            .length_squared()
            < 45.72_f32.powi(2)
    });
    for hit in impacts.read() {
        if !occupied
            || puzzles.first[3]
            || puzzles.ringing > 0.
            || hit.weapon.0 != "weapon.staff_lightning_zm"
        {
            continue;
        }
        let Some((entity, key, _)) = keys
            .iter()
            .filter(|(_, _, p)| p.translation.distance_squared(hit.point) <= 0.508_f32.powi(2))
            .min_by(|a, b| {
                a.2.translation
                    .distance_squared(hit.point)
                    .total_cmp(&b.2.translation.distance_squared(hit.point))
            })
        else {
            continue;
        };
        if puzzles.notes.contains(&key.0) {
            continue;
        }
        if puzzles.notes.len() >= 3 {
            for sound in puzzles.note_sounds.drain(..) {
                commands.entity(sound).try_despawn();
            }
            for fx in puzzles.note_fx.drain(..) {
                commands.entity(fx).try_despawn();
            }
            for (entity, _, _) in &keys {
                commands.entity(entity).remove::<PianoPlaying>();
            }
            puzzles.notes.clear();
        }
        puzzles.notes.insert(key.0.clone());
        commands.entity(entity).insert(PianoPlaying);
        let sound = crate::audio::emit_loop(
            &mut commands,
            format!("zmb_kbd_{}", key.0),
            crate::audio::SoundSource::attached(entity),
        );
        puzzles.note_sounds.push(sound);
        let fx = crate::native_fx::attach(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_puzzle_elec_sparks",
            entity,
            Transform::IDENTITY,
        );
        puzzles.note_fx.push(fx);
        if puzzles.notes.len() != 3 {
            continue;
        }
        let chords = [["a", "c", "e"], ["e", "g", "b"], ["d", "f", "a"]];
        if chords[puzzles.chords]
            .iter()
            .all(|n| puzzles.notes.contains(*n))
        {
            puzzles.chords += 1;
            puzzles.ringing = 4.;
            for (player, _, inventory) in &players {
                if inventory
                    .owned()
                    .any(|(_, weapon)| weapon.definition.0 == "weapon.staff_lightning_zm")
                {
                    puzzle_good(&mut commands, player);
                }
            }
        } else {
            puzzles.rejected += 1;
        }
    }
    let chain = [
        ("elec", 0),
        ("ice", 3),
        ("village", 2),
        ("air", 2),
        ("start", 1),
        ("tank_platform", 0),
        ("ruins", 2),
    ];
    let mut powered = BTreeMap::new();
    if puzzles.first[3] && !puzzles.second[3] {
        for (name, value) in chain {
            let sending = puzzles.relays.get(name) == Some(&value);
            powered.insert(name, if sending { 2 } else { 1 });
            if !sending {
                break;
            }
        }
        if powered.get("ruins") == Some(&2) {
            puzzles.second[3] = true;
            stage_feedback(&mut commands, 3, 1, Vec3::ZERO);
        }
    }
    for (entity, relay, mut state, mut pose) in &mut relays {
        if state.remaining > 0. {
            state.remaining = (state.remaining - time.delta_secs()).max(0.);
            pose.rotation = state.from.slerp(state.to, 1. - state.remaining / 0.1);
        }
        let power = powered.get(relay.0.as_str()).copied().unwrap_or(0);
        if power != state.power {
            if let Some(sound) = state.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if let Some(fx) = state.fx.take() {
                commands.entity(fx).try_despawn();
            }
            if power == 1 {
                state.fx = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_sparks",
                    entity,
                    Transform::from_xyz(0., 0., -0.0254)
                        .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
                ));
            }
            if power > 0 {
                state.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    if power == 2 {
                        "zmb_squest_elec_switch_hum"
                    } else {
                        "zmb_squest_elec_switch_spark"
                    },
                    crate::audio::SoundSource::attached(entity),
                ));
            }
            state.power = power;
        }
    }
}
pub(crate) fn relay(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut puzzles: ResMut<StaffPuzzles>,
    mut focus: ResMut<InteractionFocus>,
    spatial: SpatialQuery,
    mut targets: Query<(Entity, &Relay, &Transform, &mut RelayState), Without<Player>>,
    players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &WeaponInventory,
        Has<crate::perks::PerkDrinking>,
        Option<&crate::actions::HandReservation>,
    )>,
) {
    if !input.use_held {
        puzzles.latched = false;
    }
    let Ok((owner, player, pose, health, inventory, drinking, reservation)) = players.single()
    else {
        return;
    };
    if focus.target.is_some() || health.state() != LifeState::Standing || puzzles.second[3] {
        return;
    }
    let player_origin = pose.translation - Vec3::Y * (player.height * 0.5);
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let target = targets
        .iter()
        .filter_map(|(entity, relay, pose, state)| {
            let delta = pose.translation - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            ((pose.translation - (player_origin)).xz().length_squared() < 1.27_f32.powi(2)
                && state.remaining <= 0.
                && forward.dot(*direction) > 0.7
                && spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.1).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((entity, relay.0.clone(), distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, relay, _)) = target else {
        return;
    };
    focus.target = Some(entity);
    focus.hold_seconds = 0.;
    focus.progress = 0.;
    focus.prompt = format!(
        "Press {} to rotate electrical panel ({})",
        settings.label(crate::input::Action::Interact),
        puzzles.relays[&relay]
    );
    if !input.use_held
        || puzzles.latched
        || drinking
        || reservation.is_some()
        || inventory.action != WeaponAction::Ready
    {
        return;
    }
    puzzles.latched = true;
    let position = puzzles.relays.entry(relay.clone()).or_default();
    *position = (*position + 1) % 4;
    let orientation = *position;
    if (relay == "tank_platform" && orientation == 0) || (relay == "ice" && orientation == 3) {
        puzzle_good(&mut commands, owner);
    }
    if let Ok((_, _, pose, mut state)) = targets.get_mut(entity) {
        state.from = pose.rotation;
        state.to =
            state.base * Quat::from_rotation_z(-(orientation as f32) * std::f32::consts::FRAC_PI_2);
        state.remaining = 0.1;
    }
    crate::audio::emit(
        &mut commands,
        "zmb_squest_elec_switch",
        crate::audio::SoundSource::attached(entity),
    );
    focus.accepted += 1;
}

#[derive(Component)]
struct WindRing {
    index: usize,
    remaining: f32,
    from: f32,
    to: f32,
}
#[derive(Component)]
struct Smoke {
    index: usize,
    remaining: f32,
    direction: Vec3,
    attacker: Option<Entity>,
}
#[derive(Component)]
pub(crate) struct SmokeEmitter {
    index: usize,
    fx: Option<Entity>,
    sound: Option<Entity>,
}
fn wind(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut puzzles: ResMut<StaffPuzzles>,
    mut session: ResMut<crate::gameplay::Session>,
    mut hits: MessageReader<EnvironmentalHit>,
    mut rings: Query<
        (Entity, &mut WindRing, &mut Transform),
        (Without<Player>, Without<SmokeEmitter>),
    >,
    mut smoke: Query<(Entity, &mut Smoke)>,
    mut emitters: Query<
        (Entity, &mut SmokeEmitter, &mut Transform, &mut Visibility),
        Without<WindRing>,
    >,
) {
    let dt = time.delta_secs();
    let mut arrived = false;
    for (_, mut ring, mut pose) in &mut rings {
        if ring.remaining > 0. {
            ring.remaining = (ring.remaining - dt).max(0.);
            pose.rotation = Quat::from_rotation_y(
                ring.from + (ring.to - ring.from) * (1. - ring.remaining / 0.5),
            );
            if ring.remaining == 0. {
                arrived = true;
            }
        }
    }
    for (_, mut target) in &mut smoke {
        if target.remaining > 0. {
            target.remaining = (target.remaining - dt).max(0.);
            if target.remaining == 0. {
                let origins = [
                    Vec3::new(-48.1711, 4.826, -0.2032),
                    Vec3::new(61.1886, 4.9149, -13.4366),
                    Vec3::new(-22.9997, 4.6482, 29.6291),
                ];
                puzzles.smoke[target.index] = target.direction.dot(
                    (Vec3::new(0.1016, 14.8336, 0.) - origins[target.index]).normalize_or_zero(),
                ) > 25_f32.to_radians().cos();
                if puzzles.smoke[target.index]
                    && session.random() % 100 < 10
                    && let Some(player) = target.attacker
                {
                    puzzle_good(&mut commands, player);
                }
            }
        }
    }
    if arrived
        && !puzzles.first[2]
        && puzzles.wind_rings == [0, 3, 2, 3]
        && rings.iter().all(|(_, r, _)| r.remaining == 0.)
    {
        puzzles.first[2] = true;
        puzzles.wind_cleanup = 4.5;
        stage_feedback(&mut commands, 2, 0, Vec3::ZERO);
        for (entity, mut emitter, _, mut visibility) in &mut emitters {
            *visibility = Visibility::Inherited;
            emitter.sound = Some(crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_wind_incense_loop",
                crate::audio::SoundSource::attached(entity),
            ));
            emitter.fx = Some(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_air_smoke",
                entity,
                Transform::IDENTITY,
            ));
        }
    }
    if puzzles.first[2] && !puzzles.second[2] && puzzles.smoke.into_iter().all(|v| v) {
        puzzles.second[2] = true;
        puzzles.smoke_cleanup = 6.;
        stage_feedback(&mut commands, 2, 1, Vec3::ZERO);
    }
    for hit in hits.read() {
        if hit.weapon.0 != "weapon.staff_air_zm" {
            continue;
        }
        if !puzzles.first[2]
            && let Ok((_, mut ring, pose)) = rings.get_mut(hit.target)
        {
            if ring.remaining > 0. {
                continue;
            }
            puzzles.wind_rings[ring.index] = (puzzles.wind_rings[ring.index] + 1) % 4;
            ring.from = ring.to;
            ring.to += std::f32::consts::FRAC_PI_2;
            ring.remaining = 0.5;
            let _ = pose;
            crate::audio::emit(
                &mut commands,
                "zmb_squest_wind_ring_turn",
                crate::audio::SoundSource::attached(hit.target),
            );
        }
        if puzzles.first[2]
            && !puzzles.second[2]
            && let Ok((_, mut target)) = smoke.get_mut(hit.target)
        {
            if target.remaining > 0. {
                continue;
            }
            target.remaining = 1.;
            target.direction = hit.direction.normalize_or_zero();
            target.attacker = Some(hit.attacker);
        }
    }
    if puzzles.wind_cleanup > 0. {
        puzzles.wind_cleanup = (puzzles.wind_cleanup - dt).max(0.);
        let elapsed = 4.5 - puzzles.wind_cleanup;
        for (entity, ring, mut pose) in &mut rings {
            if ring.index < 3 {
                let amount = ((elapsed - ring.index as f32 * 1.5) / 1.5).clamp(0., 1.);
                pose.translation.y = -14.2748 + (3 - ring.index) as f32 * 0.508 * amount;
                pose.rotation = Quat::from_rotation_y(ring.to + std::f32::consts::TAU * amount);
                if amount == 1. {
                    commands
                        .entity(entity)
                        .remove::<ShootableEnvironment>()
                        .insert(ColliderDisabled);
                }
            }
        }
        if puzzles.wind_cleanup == 0. {
            crate::audio::emit(
                &mut commands,
                "zmb_squest_wind_ring_disappear",
                crate::audio::SoundSource::World(Vec3::new(285.877, -14.2748, 220.0402)),
            );
        }
    }
    if puzzles.smoke_cleanup > 0. {
        puzzles.smoke_cleanup = (puzzles.smoke_cleanup - dt).max(0.);
    }
    for (entity, mut emitter, mut pose, mut visibility) in &mut emitters {
        if puzzles.second[2] {
            if puzzles.smoke_cleanup >= 5. {
                pose.translation.y -= 25.4 * dt;
            }
            if puzzles.smoke_cleanup == 0. {
                if let Some(fx) = emitter.fx.take() {
                    commands.entity(fx).try_despawn();
                }
                if let Some(sound) = emitter.sound.take() {
                    commands.entity(sound).try_despawn();
                }
                *visibility = Visibility::Hidden;
                for (target, _) in &mut smoke {
                    commands
                        .entity(target)
                        .remove::<ShootableEnvironment>()
                        .insert(ColliderDisabled);
                }
            }
        } else if let Some((_, target)) = smoke.iter().find(|(_, s)| s.index == emitter.index)
            && target.direction.length_squared() > 0.
        {
            let yaw = (-target.direction.z).atan2(target.direction.x);
            let destination = Quat::from_rotation_y(yaw);
            pose.rotation = pose
                .rotation
                .slerp(destination, (dt / (target.remaining + dt)).clamp(0., 1.));
        }
        let _ = entity;
    }
}

#[derive(Component)]
pub(crate) struct IceTile(pub(crate) u8);
#[derive(Component)]
struct TileMotion {
    base: Quat,
    from: f32,
    to: f32,
    remaining: f32,
}
#[derive(Component)]
struct IceDigit {
    slot: usize,
    value: i8,
}
#[derive(Component)]
struct IceGem;
#[derive(Component)]
pub(crate) struct IceStone {
    index: usize,
    base: Vec3,
    rise: f32,
}
#[derive(Component)]
pub(crate) struct FrozenStone {
    pub amount: f32,
    next: f32,
    wait: f64,
}
impl FrozenStone {
    fn new(inputs: &mut crate::presentation::cues::CueParameters) -> Self {
        inputs.luminous_channel = Some((3, 0));
        inputs.script_vectors[3] = [0.; 4];
        inputs.script_available |= 1 << 3;
        Self {
            amount: 0.,
            next: 0.01,
            wait: 0.0166_f32 as f64,
        }
    }

    fn advance(&mut self, seconds: f64, inputs: &mut crate::presentation::cues::CueParameters) {
        self.wait -= seconds;
        while self.wait <= 0. && self.next <= 1. {
            self.amount = self.next;
            inputs.luminous_channel = Some((3, 0));
            inputs.script_vectors[3] = [self.amount; 4];
            self.next += 0.01;
            self.wait += 0.0166_f32 as f64;
        }
    }
}
#[cfg(test)]
mod frozen_stone_tests {
    use super::*;

    #[test]
    fn freezing_maps_all_components_and_preserves_source_wait_steps_and_terminal_float() {
        let mut inputs = crate::presentation::cues::CueParameters {
            script_available: 2,
            script_vectors: [[0.5; 4]; 4],
            ..default()
        };
        let mut stone = FrozenStone::new(&mut inputs);
        assert_eq!(inputs.script_available, 10);
        assert_eq!(inputs.script_vectors[3], [0.; 4]);
        assert_eq!(inputs.script_vectors[1], [0.5; 4]);
        let interval = 0.0166_f32 as f64;
        stone.advance(interval * 0.5, &mut inputs);
        assert_eq!(stone.amount, 0.);
        stone.advance(interval * 0.5, &mut inputs);
        assert_eq!(stone.amount, 0.01);
        stone.advance(interval * 9., &mut inputs);
        assert!((stone.amount - 0.1).abs() < 0.000001);
        stone.advance(100., &mut inputs);
        assert_eq!(stone.amount.to_bits(), 0x3f7ffff5);
        assert_eq!(inputs.script_vectors[3], [stone.amount; 4]);
        let end = stone.amount;
        stone.advance(10., &mut inputs);
        assert_eq!(stone.amount, end);
    }
}
fn setup_ice(
    mut commands: Commands,
    mut puzzles: ResMut<StaffPuzzles>,
    mut session: ResMut<crate::gameplay::Session>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    mut brushes: Query<(
        Entity,
        &crate::environment::brushes::MapBrush,
        &mut Transform,
    )>,
) {
    let positions: Vec<_> = game
        .environment
        .brush_entities
        .iter()
        .filter(|b| {
            b.source
                .get("targetname")
                .is_some_and(|name| name == "ice_tile_original")
        })
        .map(|b| Vec3::from_array(b.position))
        .collect();
    let mut available: Vec<_> = (0_u8..12).collect();
    let mut selected = BTreeMap::new();
    for position in positions {
        let index = session.random() as usize % available.len();
        selected.insert(available.remove(index), position);
    }
    puzzles.ice_tiles = selected.keys().copied().collect();
    puzzles.ice_remaining = puzzles.ice_tiles.clone();
    choose_ice_value(&mut puzzles, &mut session);
    for (entity, brush, mut pose) in &mut brushes {
        let Some(source) = game
            .environment
            .brush_entities
            .iter()
            .find(|b| b.id == brush.id)
        else {
            continue;
        };
        if source
            .source
            .get("targetname")
            .is_some_and(|name| name == "ice_chamber_digit")
        {
            if let (Some(slot), Some(value)) = (
                source
                    .source
                    .get("script_noteworthy")
                    .and_then(|s| s.parse::<usize>().ok()),
                source
                    .source
                    .get("script_string")
                    .and_then(|s| s.parse::<i8>().ok()),
            ) {
                commands.entity(entity).insert(IceDigit { slot, value });
            }
        }
        if source
            .source
            .get("script_noteworthy")
            .is_some_and(|name| name == "ice_ceiling_tile")
        {
            let Some(value) = source
                .source
                .get("script_string")
                .and_then(|s| s.parse::<u8>().ok())
            else {
                continue;
            };
            if let Some(position) = selected.get(&value) {
                *pose = source.transform();
                pose.translation = *position;
                commands
                    .entity(entity)
                    .insert((
                        IceTile(value),
                        TileMotion {
                            base: pose.rotation,
                            from: 0.,
                            to: 0.,
                            remaining: 0.,
                        },
                        EnvironmentOwner::Quest,
                        ShootableEnvironment,
                        Visibility::Inherited,
                    ))
                    .remove::<ColliderDisabled>();
            } else {
                *pose = source.transform();
                commands
                    .entity(entity)
                    .remove::<(IceTile, TileMotion, ShootableEnvironment)>()
                    .insert(Visibility::Hidden)
                    .insert_if_new(ColliderDisabled);
            }
        }
    }
    commands.spawn((
        SessionEntity,
        IceGem,
        Transform::from_translation(Vec3::new(290.4998, -6.7056, 187.1218)),
        Collider::sphere(0.55),
        Sensor,
        CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
        EnvironmentOwner::Quest,
        ShootableEnvironment,
    ));
    for (index, point) in [
        [74.273156, 4.66725, 12.209018],
        [17.470882, 4.778756, 7.70636],
        [-29.95295, -3.182366, -123.754642],
    ]
    .into_iter()
    .enumerate()
    {
        let mut stone = commands.spawn((
            SessionEntity,
            IceStone {
                index,
                base: Vec3::from_array(point),
                rise: 0.,
            },
            crate::presentation::cues::CueParameters::default(),
            Transform::from_translation(Vec3::from_array(point)).with_rotation(native_rotation(
                [354.459, 347.137, 349.084][index],
                [26.6952, 88.9387, 211.357][index],
                [5.2057, 15.2013, -7.12971][index],
            )),
            Visibility::Hidden,
            ColliderDisabled,
            Collider::cuboid(0.75, 1.5, 0.4),
            Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE),
            EnvironmentOwner::Quest,
        ));
        if let Some(scene) = game
            .content
            .presentations
            .iter()
            .filter_map(|p| p.scene.as_ref())
            .find(|scene| scene.contains("p6_zm_tm_note_rock_01_anim_lod0"))
        {
            stone.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
}
fn choose_ice_value(puzzles: &mut StaffPuzzles, session: &mut crate::gameplay::Session) {
    puzzles.ice_value = if puzzles.ice_remaining.is_empty() {
        None
    } else {
        puzzles
            .ice_remaining
            .iter()
            .nth(session.random() as usize % puzzles.ice_remaining.len())
            .copied()
    };
}
fn ice(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut puzzles: ResMut<StaffPuzzles>,
    mut session: ResMut<crate::gameplay::Session>,
    game: Res<crate::assets::GameDefinition>,
    mut hits: MessageReader<EnvironmentalHit>,
    mut tiles: Query<(&IceTile, &mut TileMotion, &mut Transform), Without<IceStone>>,
    gems: Query<(), With<IceGem>>,
    mut stones: Query<
        (
            Entity,
            &mut IceStone,
            &mut Transform,
            &mut Visibility,
            Option<&mut FrozenStone>,
            &mut crate::presentation::cues::CueParameters,
        ),
        Without<IceTile>,
    >,
    mut digits: Query<(&IceDigit, &mut Visibility), Without<IceStone>>,
    mut weather: ResMut<crate::environment::weather::Weather>,
) {
    let dt = time.delta_secs();
    let ternary = [
        [-1, 0, -1],
        [-1, 1, -1],
        [-1, 2, -1],
        [1, -1, 0],
        [1, -1, 1],
        [1, -1, 2],
        [2, -1, 0],
        [2, -1, 1],
        [2, -1, 2],
        [1, 0, 0],
        [1, 0, 1],
        [1, 0, 2],
    ];
    for (digit, mut visibility) in &mut digits {
        *visibility = if puzzles
            .ice_value
            .is_some_and(|v| digit.slot < 3 && ternary[v as usize][digit.slot] == digit.value)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    let previous = puzzles.ice_lock;
    puzzles.ice_lock = (previous - dt).max(0.);
    if previous > 0. && puzzles.ice_lock == 0. && puzzles.ice_pending {
        puzzles.ice_pending = false;
        choose_ice_value(&mut puzzles, &mut session);
    }
    for (_, mut motion, mut pose) in &mut tiles {
        if motion.remaining > 0. {
            motion.remaining = (motion.remaining - dt).max(0.);
            let angle = motion.from + (motion.to - motion.from) * (1. - motion.remaining / 0.5);
            pose.rotation = motion.base * Quat::from_rotation_x(angle);
        }
    }
    for (entity, mut stone, mut pose, mut visibility, frozen, mut inputs) in &mut stones {
        if puzzles.first[1] && !puzzles.broken_stones[stone.index] {
            if stone.rise == 0. {
                *visibility = Visibility::Inherited;
                commands
                    .entity(entity)
                    .remove::<ColliderDisabled>()
                    .insert(ShootableEnvironment);
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_shovel_dig",
                    Transform::from_translation(stone.base),
                );
            }
            stone.rise = (stone.rise + dt).min(1.);
            pose.translation = stone.base - pose.rotation * Vec3::Y * (1.6256 * (1. - stone.rise));
        }
        if let Some(mut frozen) = frozen {
            frozen.advance(time.delta_secs_f64(), &mut inputs);
        }
    }
    if puzzles.snow > 0. {
        puzzles.snow = (puzzles.snow - dt).max(0.);
        if puzzles.snow == 0. {
            weather.puzzle_precipitation(crate::environment::weather::WeatherKind::Clear, 0);
        }
    }
    for hit in hits.read() {
        let water = hit.weapon.0.contains("staff_water");
        if !puzzles.first[1] && water && puzzles.ice_lock <= 0. {
            if gems.contains(hit.target) {
                choose_ice_value(&mut puzzles, &mut session);
            }
            let value = tiles.get(hit.target).ok().map(|(t, _, _)| t.0);
            if let Some(value) = value.filter(|v| puzzles.ice_remaining.contains(v)) {
                if puzzles.ice_value == Some(value) {
                    puzzle_good(&mut commands, hit.attacker);
                    puzzles.ice_remaining.remove(&value);
                    puzzles.ice_lock = 0.7;
                    if let Ok((_, mut motion, _)) = tiles.get_mut(hit.target) {
                        motion.from = motion.to;
                        motion.to = std::f32::consts::PI;
                        motion.remaining = 0.5;
                    }
                    crate::audio::emit(
                        &mut commands,
                        "zmb_squest_ice_tile_flip",
                        crate::audio::SoundSource::attached(hit.target),
                    );
                } else {
                    for (tile, mut motion, _) in &mut tiles {
                        if !puzzles.ice_remaining.contains(&tile.0) {
                            motion.from = motion.to;
                            motion.to = 0.;
                            motion.remaining = 0.5;
                        }
                    }
                    puzzles.ice_remaining = puzzles.ice_tiles.clone();
                    puzzles.ice_lock = 2.;
                    puzzles.rejected += 1;
                }
                puzzles.ice_pending = true;
                if puzzles.ice_remaining.is_empty() {
                    puzzles.first[1] = true;
                    stage_feedback(&mut commands, 1, 0, Vec3::ZERO);
                }
            }
        }
        if puzzles.first[1]
            && !puzzles.second[1]
            && let Ok((_, stone, _, _, _, mut inputs)) = stones.get_mut(hit.target)
        {
            let index = stone.index;
            if !puzzles.frozen_stones[index] && water {
                puzzle_good(&mut commands, hit.attacker);
                puzzles.frozen_stones[index] = true;
                commands
                    .entity(hit.target)
                    .insert(FrozenStone::new(&mut inputs));
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_ice_stone_freeze",
                    crate::audio::SoundSource::attached(hit.target),
                );
            } else if puzzles.frozen_stones[index]
                && !hit.weapon.0.contains("staff")
                && game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id == hit.weapon)
                    .is_some_and(|w| matches!(w.category, crate::origins::DamageCategory::Bullet))
            {
                puzzles.broken_stones[index] = true;
                puzzle_good(&mut commands, hit.attacker);
                commands
                    .entity(hit.target)
                    .insert((Visibility::Hidden, ColliderDisabled))
                    .remove::<ShootableEnvironment>();
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_ice_stone_shatter",
                    crate::audio::SoundSource::World(hit.point),
                );
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_puzzle_ice_pipe_burst",
                    Transform::from_translation(stone.base),
                );
                if puzzles.broken_stones.into_iter().all(|b| b) {
                    puzzles.second[1] = true;
                    stage_feedback(&mut commands, 1, 1, hit.point);
                    puzzles.snow = 5.;
                    weather.puzzle_precipitation(crate::environment::weather::WeatherKind::Snow, 5);
                }
            }
        }
    }
}

#[derive(Component)]
struct FireSymbol(u8);
fn setup_fire(
    mut commands: Commands,
    mut puzzles: ResMut<StaffPuzzles>,
    mut session: ResMut<crate::gameplay::Session>,
    game: Res<crate::assets::GameDefinition>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
) {
    let mut groups: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for brush in &game.environment.brush_entities {
        if brush
            .source
            .get("script_noteworthy")
            .is_some_and(|name| name == "fire_torch_ternary")
        {
            if let (Some(group), Some(value)) = (
                brush.source.get("targetname"),
                brush
                    .source
                    .get("script_int")
                    .and_then(|s| s.parse::<u8>().ok()),
            ) {
                groups.entry(group.clone()).or_default().push(value);
            }
        }
    }
    for values in groups.values() {
        puzzles
            .fire_torches
            .insert(values[session.random() as usize % values.len()]);
    }
    for (entity, brush) in &brushes {
        if let Some(source) = game
            .environment
            .brush_entities
            .iter()
            .find(|b| b.id == brush.id)
            && source
                .source
                .get("script_noteworthy")
                .is_some_and(|name| name == "fire_torch_ternary")
        {
            if let Some(value) = source
                .source
                .get("script_int")
                .and_then(|s| s.parse::<u8>().ok())
            {
                commands
                    .entity(entity)
                    .insert((FireSymbol(value), Visibility::Hidden));
            }
        }
    }
}
fn fire(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut puzzles: ResMut<StaffPuzzles>,
    mut impacts: MessageReader<crate::projectiles::ProjectileImpact>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    crafting: Res<crate::crafting::Crafting>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    spatial: SpatialQuery,
    volumes: Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    players: Query<(Entity, &Transform, &WeaponInventory), With<Player>>,
    enemies: Query<(
        &Transform,
        &crate::enemies::Zombie,
        &crate::combat_effects::Wounds,
    )>,
    mut symbols: Query<(&FireSymbol, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    let occupied = players.iter().any(|(_, p, _)| {
        (p.translation - Vec3::new(262.636, -7.9756, 200.8124))
            .xz()
            .length_squared()
            < 45.72_f32.powi(2)
    });
    for corpse in &mut puzzles.fire_corpses {
        corpse.remaining = (corpse.remaining - dt).max(0.);
    }
    puzzles.corpse_poll -= dt;
    if puzzles.corpse_poll <= 0. {
        puzzles.corpse_poll += 1.;
        if !occupied {
            for corpse in puzzles.fire_corpses.drain(..) {
                commands.entity(corpse.entity).try_despawn();
            }
        }
    }
    let ignition = puzzles.ignition;
    puzzles.ignition = (ignition - dt).max(0.);
    if ignition > 0. && puzzles.ignition == 0. {
        if let Some((owner, fx)) = puzzles.floor_fx.take() {
            commands.entity(fx).try_despawn();
            commands.entity(owner).try_despawn();
        }
        if puzzles.sacrifices >= 32 && !puzzles.first[0] {
            puzzles.first[0] = true;
            for corpse in puzzles.fire_corpses.drain(..) {
                commands.entity(corpse.entity).try_despawn();
            }
            puzzles.plinth_cleanup = 30.;
            stage_feedback(&mut commands, 0, 0, Vec3::ZERO);
        }
    }
    for timer in puzzles.torch_cooldown.values_mut() {
        *timer = (*timer - dt).max(0.);
    }
    for duration in puzzles.lit_torches.values_mut() {
        *duration -= dt;
    }
    if !puzzles.second[0] {
        puzzles.lit_torches.retain(|_, t| *t > 0.);
    }
    if puzzles.second[0] && puzzles.celebration > 0. {
        puzzles.celebration = (puzzles.celebration - dt).max(0.);
        let lighting = puzzles.celebration_order.len() as f32 * 0.25;
        let elapsed = lighting + 30. + source::TORCHES.len() as f32 * 0.25 - puzzles.celebration;
        while puzzles.celebration_next < puzzles.celebration_order.len()
            && elapsed >= puzzles.celebration_next as f32 * 0.25
        {
            let number = puzzles.celebration_order[puzzles.celebration_next];
            puzzles.lit_torches.insert(number, 15.);
            puzzles.celebration_next += 1;
        }
        if elapsed >= lighting + 30. {
            let removed = ((elapsed - lighting - 30.) / 0.25).floor() as usize + 1;
            for (number, _) in source::TORCHES.iter().take(removed) {
                puzzles.lit_torches.remove(number);
            }
        }
    }
    let inside = |point| {
        spatial
            .shape_intersections(
                &Collider::sphere(0.1),
                point,
                Quat::IDENTITY,
                &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
            )
            .iter()
            .filter_map(|e| volumes.get(*e).ok())
            .filter(|(_, state)| state.enabled)
            .map(|(volume, _)| volume)
            .any(|v| {
                game.environment
                    .triggers
                    .iter()
                    .find(|t| t.id == v.0)
                    .is_some_and(|t| {
                        t.source
                            .get("targetname")
                            .is_some_and(|n| n == "fire_sacrifice_volume")
                    })
            })
    };
    for impact in impacts.read() {
        if !impact.weapon.0.contains("staff_fire") {
            continue;
        }
        if !puzzles.first[0] && occupied && puzzles.ignition == 0. && inside(impact.point) {
            puzzles.ignition = 6.;
            let owner = commands
                .spawn((
                    SessionEntity,
                    Transform::from_translation(Vec3::new(9881.73, -466., 8557.79) * 0.0254),
                ))
                .id();
            let fx = crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_fire_sacrifice",
                owner,
                Transform::IDENTITY.with_rotation(native_rotation(-90., 0., 0.)),
            );
            puzzles.floor_fx = Some((owner, fx));
        }
        if puzzles.first[0] && !puzzles.second[0] {
            for (number, point) in source::TORCHES {
                if impact.point.distance_squared(Vec3::from_array(*point)) <= 1.6256_f32.powi(2)
                    && puzzles.torch_cooldown.get(number).copied().unwrap_or(0.) <= 0.
                {
                    puzzles.torch_cooldown.insert(*number, 2.);
                    if let Some((owner, fx, sound)) = puzzles.torch_sounds.remove(number) {
                        commands.entity(sound).try_despawn();
                        commands.entity(fx).try_despawn();
                        commands.entity(owner).try_despawn();
                    }
                    puzzles.torch_out.remove(number);
                    puzzles.lit_torches.insert(*number, 15.);
                    puzzles.torches_lit += 1;
                    if !puzzles.fire_torches.contains(number) {
                        puzzles.rejected += 1;
                    }
                    if puzzles.torches_lit % 12 != 0
                        && puzzles
                            .lit_torches
                            .keys()
                            .all(|n| puzzles.fire_torches.contains(n))
                        && puzzles
                            .fire_torches
                            .iter()
                            .any(|n| !puzzles.lit_torches.contains_key(n))
                    {
                        for (player, _, inventory) in &players {
                            if inventory
                                .owned()
                                .any(|(_, weapon)| weapon.definition.0 == "weapon.staff_fire_zm")
                            {
                                puzzle_good(&mut commands, player);
                            }
                        }
                    }
                }
            }
            if puzzles.lit_torches.keys().copied().collect::<BTreeSet<_>>() == puzzles.fire_torches
            {
                puzzles.second[0] = true;
                puzzles.celebration_order = source::TORCHES
                    .iter()
                    .map(|(n, _)| *n)
                    .filter(|n| !puzzles.lit_torches.contains_key(n))
                    .collect();
                puzzles.celebration = puzzles.celebration_order.len() as f32 * 0.25
                    + 30.
                    + source::TORCHES.len() as f32 * 0.25;
                puzzles.celebration_next = 0;
                stage_feedback(&mut commands, 0, 1, impact.point);
            }
        }
    }
    for death in deaths.read() {
        if puzzles.first[0]
            || !occupied
            || !crafting.crafted.contains("elemental_staff_fire")
            || !death.0.can_supply_soul()
        {
            continue;
        }
        if let Ok((pose, zombie, wounds)) = enemies.get(death.0.impact.target)
            && inside(pose.translation)
            && let Some(entity) = crate::enemies::spawn_visual_clone(
                &mut commands,
                &server,
                &game,
                &zombie.definition,
                *pose,
                if wounds.crawler {
                    "ai_zombie_crawl_death_v1"
                } else {
                    "ch_dazed_a_death"
                },
            )
        {
            commands
                .entity(death.0.impact.target)
                .insert(Visibility::Hidden);
            let mut appearance = crate::combat_effects::Wounds::default();
            appearance.crawler = wounds.crawler;
            appearance.headless = wounds.headless;
            appearance.left_arm = wounds.left_arm;
            appearance.right_arm = wounds.right_arm;
            commands.entity(entity).insert(appearance);
            if puzzles.fire_corpses.len() >= 15 {
                let oldest = puzzles.fire_corpses.remove(0);
                commands.entity(oldest.entity).try_despawn();
            }
            puzzles.fire_corpses.push(FireCorpse {
                entity,
                pose: *pose,
                remaining: 2.1333333333,
            });
        }
    }
    if puzzles.ignition > 0. {
        let ready: Vec<_> = puzzles
            .fire_corpses
            .iter()
            .filter(|corpse| corpse.remaining == 0.)
            .map(|corpse| (corpse.entity, corpse.pose))
            .collect();
        puzzles.fire_corpses.retain(|corpse| corpse.remaining > 0.);
        puzzles.sacrifices += ready.len() as u32;
        for (entity, pose) in ready {
            for (player, _, inventory) in &players {
                if inventory
                    .owned()
                    .any(|(_, weapon)| weapon.definition.0 == "weapon.staff_fire_zm")
                {
                    puzzle_good(&mut commands, player);
                }
            }
            crate::native_fx::emit(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_fire_exp_ash",
                pose,
            );
            commands.entity(entity).try_despawn();
        }
    }
    let out: Vec<_> = puzzles
        .torch_sounds
        .keys()
        .copied()
        .filter(|n| {
            !puzzles.torch_out.contains(n) && puzzles.lit_torches.get(n).is_none_or(|t| *t <= 0.)
        })
        .collect();
    for number in out {
        puzzles.torch_out.insert(number);
        if let Some((_, _, sound)) = puzzles.torch_sounds.get(&number) {
            commands.entity(*sound).try_despawn();
        }
        if let Some((_, point)) = source::TORCHES.iter().find(|(n, _)| *n == number) {
            crate::audio::emit(
                &mut commands,
                "zmb_squest_fire_torch_out",
                crate::audio::SoundSource::World(Vec3::from_array(*point)),
            );
        }
    }
    let expired: Vec<_> = puzzles
        .torch_sounds
        .keys()
        .copied()
        .filter(|n| !puzzles.lit_torches.contains_key(n))
        .collect();
    for number in expired {
        if let Some((owner, fx, sound)) = puzzles.torch_sounds.remove(&number) {
            commands.entity(sound).try_despawn();
            commands.entity(fx).try_despawn();
            commands.entity(owner).try_despawn();
            puzzles.torch_out.remove(&number);
        }
    }
    let newly_lit: Vec<_> = puzzles
        .lit_torches
        .keys()
        .copied()
        .filter(|n| !puzzles.torch_sounds.contains_key(n))
        .collect();
    for number in newly_lit {
        if let Some((_, point)) = source::TORCHES.iter().find(|(n, _)| *n == number) {
            let owner = commands
                .spawn((
                    SessionEntity,
                    Transform::from_translation(Vec3::from_array(*point)),
                ))
                .id();
            let fx = crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_fire_torch",
                owner,
                Transform::IDENTITY.with_rotation(native_rotation(-90., 0., 0.)),
            );
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_fire_torch_loop",
                crate::audio::SoundSource::attached(owner),
            );
            puzzles.torch_sounds.insert(number, (owner, fx, sound));
            crate::audio::emit(
                &mut commands,
                "zmb_squest_fire_torch_ignite",
                crate::audio::SoundSource::World(Vec3::from_array(*point)),
            );
        }
    }
    let plinths = [
        ([9909.78, -8828.47, -428.], 0.5),
        ([9822.28, -8774.97, -435.], 1.),
        ([9953.78, -8849.97, -432.5], 0.001),
        ([9866.28, -8801.97, -428.], 0.75),
    ];
    for (index, (point, threshold)) in plinths.into_iter().enumerate() {
        if !puzzles.plinths.contains_key(&index)
            && (puzzles.sacrifices as f32 / 32. > threshold || puzzles.sacrifices >= 32)
            && (!puzzles.first[0] || puzzles.plinth_cleanup > 0.)
        {
            let point = Vec3::new(point[0], point[2], -point[1]) * 0.0254;
            let owner = commands
                .spawn((SessionEntity, Transform::from_translation(point)))
                .id();
            let fx = crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_puzzle_fire_torch",
                owner,
                Transform::IDENTITY.with_rotation(native_rotation(-90., 0., 0.)),
            );
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_squest_fire_torch_loop",
                crate::audio::SoundSource::attached(owner),
            );
            puzzles.plinths.insert(index, (owner, fx, sound));
            crate::audio::emit(
                &mut commands,
                "zmb_squest_fire_torch_ignite",
                crate::audio::SoundSource::World(point),
            );
        }
    }
    if puzzles.plinth_cleanup > 0. {
        puzzles.plinth_cleanup = (puzzles.plinth_cleanup - dt).max(0.);
        if puzzles.plinth_cleanup == 0. {
            for (_, (owner, fx, sound)) in std::mem::take(&mut puzzles.plinths) {
                commands.entity(sound).try_despawn();
                commands.entity(fx).try_despawn();
                commands.entity(owner).try_despawn();
            }
        }
    }
    for (symbol, mut visibility) in &mut symbols {
        *visibility = if puzzles.first[0] && puzzles.fire_torches.contains(&symbol.0) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}
