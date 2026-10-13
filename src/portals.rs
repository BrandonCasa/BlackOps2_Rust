use crate::{
    GameState,
    access::ZoneAccess,
    assets::GameDefinition,
    crafting::Crafting,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    player::Player,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;
const OPEN_SECONDS: f32 = 12.;
const CLOSE_SECONDS: f32 = 5.9666667;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PortalPhase {
    #[default]
    Closed,
    Opening(f32),
    Open,
    Closing(f32),
}
#[derive(Resource, Default)]
pub struct Portals {
    pub gramophone: Option<usize>,
    pub entrances: [PortalPhase; 4],
    pub returns: [f32; 4],
    return_closing: [f32; 4],
    pub excavation_open: bool,
    pub trips: u32,
    pub visited: [bool; 4],
    pub return_unlocked: [bool; 4],
    pub crystals_ready: [bool; 4],
    crystals: [CrystalStage; 4],
    opening_stairs: f32,
    latched: bool,
    cooldown: f32,
    audio: Option<Entity>,
}
#[derive(Clone, Copy, Default)]
enum CrystalStage {
    #[default]
    Waiting,
    Plinth(f32),
    Looking(f32),
    Rising(f32),
    Ready,
}
#[derive(Component)]
pub(crate) struct Transit {
    remaining: f32,
    destination: source::SourcePose,
    effect: Entity,
}
#[derive(Component)]
struct CrystalPlinth {
    element: usize,
    origin: Vec3,
}
#[derive(Component)]
struct PortalFrame {
    element: usize,
    exit: bool,
    source: Handle<Gltf>,
    effect: Option<Entity>,
    sound: Option<Entity>,
    phase: u8,
}
#[derive(Component)]
pub(crate) enum Target {
    Gramophone(usize),
    Entrance,
    Return(usize),
}
pub struct PortalPlugin;
impl Plugin for PortalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Portals>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (tick, interact, travel, crystals, presentation)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::perks::vendors_interaction)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut portals: ResMut<Portals>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    extras: Query<(Entity, &bevy::gltf::GltfExtras)>,
    children: Query<&Children>,
) {
    *portals = default();
    for (entity, metadata) in &extras {
        if serde_json::from_str::<serde_json::Value>(&metadata.value)
            .ok()
            .is_some_and(|d| {
                d["static_instance"] == true
                    && d["source_model"].as_str().is_some_and(|m| {
                        m == "fxanim_zom_tomb_portal_mod"
                            || m.starts_with("p6_zm_tm_elm_plinth_top_")
                    })
            })
        {
            commands
                .entity(entity)
                .insert((Visibility::Hidden, ColliderDisabled));
            for child in children.iter_descendants(entity) {
                commands.entity(child).insert(ColliderDisabled);
            }
        }
    }
    for (index, position) in source::GRAMOPHONES.iter().enumerate() {
        commands.spawn((
            SessionEntity,
            Target::Gramophone(index),
            Transform::from_translation(Vec3::from_array(*position)),
        ));
    }
    for i in 0..4 {
        commands.spawn((
            SessionEntity,
            Target::Entrance,
            Transform::from_translation(Vec3::from_array(source::ENTRANCES[i])),
        ));
        commands.spawn((
            SessionEntity,
            Target::Return(i),
            Transform::from_translation(Vec3::from_array(source::EXITS[i])),
        ));
        if let Some(scene) = crate::assets::native_scene(&game, "fxanim_zom_tomb_portal_mod") {
            for (exit, pose) in [
                (false, source::ENTRANCE_FRAMES[i]),
                (true, source::RETURN_FRAMES[i]),
            ] {
                commands.spawn((
                    SessionEntity,
                    PortalFrame {
                        element: i,
                        exit,
                        effect: None,
                        sound: None,
                        phase: 0,
                        source: server.load(scene.split('#').next().unwrap_or(scene).to_owned()),
                    },
                    WorldAssetRoot(server.load(scene.clone())),
                    Transform::from_translation(Vec3::from_array(pose.feet)).with_rotation(
                        Quat::from_rotation_y(pose.yaw + std::f32::consts::FRAC_PI_2),
                    ),
                    Visibility::Hidden,
                ));
            }
        }
        let (model, origin, yaw) = [
            (
                "p6_zm_tm_elm_plinth_top_fire",
                Vec3::new(256.6416, -11.4808, 206.8068),
                135_f32,
            ),
            (
                "p6_zm_tm_elm_plinth_top_wind",
                Vec3::new(268.6304, -11.4808, 206.8068),
                45.,
            ),
            (
                "p6_zm_tm_elm_plinth_top_lightning",
                Vec3::new(256.6416, -11.4808, 194.818),
                315.,
            ),
            (
                "p6_zm_tm_elm_plinth_top_ice",
                Vec3::new(268.6304, -11.4808, 194.818),
                315.,
            ),
        ][i];
        if let Some(scene) = crate::assets::native_scene(&game, model) {
            commands.spawn((
                SessionEntity,
                CrystalPlinth { element: i, origin },
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(origin - Vec3::Y * 1.9812)
                    .with_rotation(Quat::from_rotation_y(yaw.to_radians())),
            ));
        }
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut portals: ResMut<Portals>,
    mut access: ResMut<ZoneAccess>,
    game: Res<GameDefinition>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
) {
    let dt = time.delta_secs();
    portals.cooldown = (portals.cooldown - dt).max(0.);
    for index in 0..4 {
        let phase = &mut portals.entrances[index];
        *phase = match *phase {
            PortalPhase::Opening(t) if t <= dt => PortalPhase::Open,
            PortalPhase::Opening(t) => PortalPhase::Opening(t - dt),
            PortalPhase::Closing(t) if t <= dt => PortalPhase::Closed,
            PortalPhase::Closing(t) => PortalPhase::Closing(t - dt),
            p => p,
        };
        if *phase == PortalPhase::Open {
            portals.return_unlocked[index] = true;
        }
    }
    for index in 0..4 {
        portals.return_closing[index] = (portals.return_closing[index] - dt).max(0.);
        let previous = portals.returns[index];
        portals.returns[index] = (previous - dt).max(0.);
        if previous > 0. && portals.returns[index] == 0. {
            portals.return_closing[index] = CLOSE_SECONDS;
        }
    }
    if portals.opening_stairs > 0. {
        portals.opening_stairs -= dt;
        if portals.opening_stairs <= 0. {
            portals.excavation_open = true;
            access.open.extend(
                game.content
                    .zone_connections
                    .iter()
                    .filter(|c| c.id.0.starts_with("connection.activate_zone_crypt."))
                    .map(|c| c.id.clone()),
            );
            access.refresh(&game.content);
            for (entity, brush) in &brushes {
                if game
                    .environment
                    .brush_entities
                    .iter()
                    .find(|b| b.id == brush.id)
                    .and_then(|b| b.source.get("targetname"))
                    .is_some_and(|n| {
                        matches!(
                            n.as_str(),
                            "chamber_entrance" | "junk_nml_chamber_clip" | "junk_nml_chamber"
                        )
                    })
                {
                    commands
                        .entity(entity)
                        .insert((ColliderDisabled, Visibility::Hidden));
                }
            }
        }
    }
}
pub fn relocate(
    commands: &mut Commands,
    entity: Entity,
    player: &mut Player,
    pose: &mut Transform,
    mobility: &mut crate::movement::Mobility,
    feet: Vec3,
) {
    pose.translation = feet + Vec3::Y * player.height * 0.5;
    player.velocity = Vec3::ZERO;
    player.grounded = false;
    mobility.airborne = false;
    mobility.fall_speed = 0.;
    mobility.peak = feet.y;
    mobility.dive = None;
    commands
        .entity(entity)
        .insert(crate::movement::platforms::PlatformSupport::default());
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    mut portals: ResMut<Portals>,
    mut crafting: ResMut<Crafting>,
    mut access: ResMut<ZoneAccess>,
    mut focus: ResMut<InteractionFocus>,
    targets: Query<(Entity, &Target, &Transform), Without<Player>>,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &Vitality,
            &WeaponInventory,
            &mut crate::movement::Mobility,
            Has<crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
) {
    if !input.use_held {
        portals.latched = false;
    }
    let Ok((owner, player, pose, health, inventory, mobility, drinking)) = players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing || focus.target.is_some() {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let candidate = targets
        .iter()
        .filter_map(|(entity, target, t)| {
            if matches!(target, Target::Entrance)
                || matches!(target, Target::Return(i) if !portals.return_unlocked[*i])
            {
                return None;
            }
            let delta = t.translation - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            (distance < 2.54
                && forward.dot(*direction) > 0.7
                && spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.15).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((entity, target, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, target, _)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    let key = settings.label(crate::input::Action::Interact);
    let pressed = input.use_held && !portals.latched;
    if pressed {
        portals.latched = true;
    }
    let busy = drinking || inventory.action != WeaponAction::Ready || mobility.busy();
    match target {
        Target::Gramophone(index) => {
            if portals.gramophone == Some(*index) {
                focus.prompt = format!("Press {key} to retrieve Gramophone");
                if *index == 0 && !portals.excavation_open
                    || *index > 0
                        && matches!(portals.entrances[*index - 1], PortalPhase::Opening(_))
                {
                    focus.prompt = "Gramophone playing".into();
                    return;
                }
                if pressed && !busy {
                    portals.gramophone = None;
                    crafting.gramophone = true;
                    if *index > 0 {
                        portals.entrances[*index - 1] = PortalPhase::Closing(CLOSE_SECONDS);
                    }
                    if let Some(sound) = portals.audio.take() {
                        commands.entity(sound).despawn();
                    }
                    crate::audio::emit(
                        &mut commands,
                        "zmb_craftable_pickup",
                        crate::audio::SoundSource::attached(owner),
                    );
                    focus.accepted += 1;
                }
                return;
            }
            let record = ["master", "fire", "air", "elec", "ice"][*index];
            if !crafting.records.contains(record)
                || !crafting.gramophone
                || portals.gramophone.is_some()
            {
                focus.prompt = format!("Requires Gramophone and {record} record");
                focus.rejection = Some(InteractionRejection::Unavailable);
                return;
            }
            focus.prompt = format!("Press {key} to place Gramophone");
            if !pressed || busy {
                return;
            }
            crafting.gramophone = false;
            portals.gramophone = Some(*index);
            let song = [
                "mus_gramophone_electric",
                "mus_gramophone_fire",
                "mus_gramophone_air",
                "mus_gramophone_electric",
                "mus_gramophone_ice",
            ][*index];
            portals.audio = Some(crate::audio::emit_loop(
                &mut commands,
                song,
                crate::audio::SoundSource::attached(entity),
            ));
            if *index == 0 {
                if !portals.excavation_open {
                    portals.opening_stairs = 16.;
                    crate::audio::emit(
                        &mut commands,
                        "zmb_crypt_stairs",
                        crate::audio::SoundSource::attached(entity),
                    );
                }
            } else {
                portals.entrances[*index - 1] = PortalPhase::Opening(OPEN_SECONDS);
                access.activated.extend(
                    game.content
                        .zones
                        .iter()
                        .filter(|z| z.id.0.starts_with("zone.chamber_"))
                        .map(|z| z.id.clone()),
                );
                access.refresh(&game.content);
            }
            focus.accepted += 1;
        }
        Target::Entrance => {}
        Target::Return(index) => {
            focus.prompt = format!("Press {key} to open the return portal");
            if portals.returns[*index] > 0. || portals.return_closing[*index] > 0. {
                focus.prompt = "Return portal active".into();
                return;
            }
            if pressed && !busy {
                portals.returns[*index] = OPEN_SECONDS + 20.;
                focus.accepted += 1;
                crate::audio::emit(
                    &mut commands,
                    "zmb_teleporter_loop_pre",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
    }
}

fn travel(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut portals: ResMut<Portals>,
    mut players: Query<(
        Entity,
        &mut Player,
        &mut Transform,
        &Vitality,
        &mut WeaponInventory,
        &mut crate::movement::Mobility,
        &mut crate::equipment::GrenadeInventory,
        &mut crate::equipment::TacticalInventory,
        &mut crate::mines::MineInventory,
        Option<&mut Transit>,
    )>,
    mut frames: Query<(&PortalFrame, &mut Visibility), Without<Player>>,
) {
    for (frame, mut visibility) in &mut frames {
        *visibility = if !frame.exit || portals.return_unlocked[frame.element] {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    for (
        entity,
        mut player,
        mut pose,
        health,
        mut inventory,
        mut mobility,
        mut grenades,
        mut tactical,
        mut mines,
        transit,
    ) in &mut players
    {
        if let Some(mut transit) = transit {
            transit.remaining -= time.delta_secs();
            if transit.remaining <= 0. || health.state() != LifeState::Standing {
                relocate(
                    &mut commands,
                    entity,
                    &mut player,
                    &mut pose,
                    &mut mobility,
                    Vec3::from_array(transit.destination.feet),
                );
                player.yaw = transit.destination.yaw;
                player.pitch = transit.destination.pitch;
                commands.entity(transit.effect).try_despawn();
                commands
                    .entity(entity)
                    .remove::<(Transit, crate::actions::HandReservation, ColliderDisabled)>();
            }
            continue;
        }
        if health.state() != LifeState::Standing
            || mobility.stance == crate::movement::Stance::Prone
            || portals.cooldown > 0.
        {
            continue;
        }
        let feet = pose.translation - Vec3::Y * player.height * 0.5;
        let destination = (0..4).find_map(|i| {
            if portals.entrances[i] == PortalPhase::Open
                && feet.distance_squared(Vec3::from_array(source::ENTRANCES[i]) - Vec3::Y * 0.762)
                    < 1.27_f32.powi(2)
            {
                portals.visited[i] = true;
                if matches!(portals.crystals[i], CrystalStage::Waiting) {
                    portals.crystals[i] = CrystalStage::Plinth(6.);
                }
                Some(source::ARRIVAL_POSES[i][0])
            } else if portals.return_unlocked[i]
                && portals.returns[i] > 0.
                && portals.returns[i] <= 20.
                && feet.distance_squared(Vec3::from_array(source::EXITS[i])) < 3.048_f32.powi(2)
            {
                Some(source::RETURN_POSES[i][0])
            } else {
                None
            }
        });
        let Some(destination) = destination else {
            continue;
        };
        inventory.cancel_action();
        grenades.cancel();
        if let Some(g) = tactical.0.as_mut() {
            g.cancel();
        }
        mines.cancel();
        crate::native_fx::emit(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_teleport_3p",
            Transform::from_translation(pose.translation),
        );
        crate::audio::emit(
            &mut commands,
            "zmb_teleporter_tele_3d",
            crate::audio::SoundSource::World(pose.translation),
        );
        relocate(
            &mut commands,
            entity,
            &mut player,
            &mut pose,
            &mut mobility,
            Vec3::from_array(source::IMAGE_ROOM.feet),
        );
        player.yaw = source::IMAGE_ROOM.yaw;
        player.pitch = source::IMAGE_ROOM.pitch;
        let effect = crate::native_fx::attach(
            &mut commands,
            "maps/zombie_tomb/fx_tomb_teleport_1p",
            entity,
            Transform::from_translation(Vec3::Y * 1.6256),
        );
        commands.entity(entity).insert((
            Transit {
                remaining: 2.,
                destination,
                effect,
            },
            crate::actions::HandReservation::held(crate::actions::HandAction::Portal),
            ColliderDisabled,
        ));
        crate::audio::emit(
            &mut commands,
            "zmb_teleporter_tele_2d",
            crate::audio::SoundSource::attached(entity),
        );

        portals.trips += 1;
        portals.cooldown = 2.;
    }
}
fn crystals(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut portals: ResMut<Portals>,
    players: Query<(&Player, &Transform, &Vitality), Without<CrystalPlinth>>,
    mut parts: Query<
        (
            Entity,
            &crate::crafting::CraftTarget,
            &mut Transform,
            &mut Visibility,
        ),
        Without<Player>,
    >,
    mut plinths: Query<(&CrystalPlinth, &mut Transform), Without<crate::crafting::CraftTarget>>,
) {
    let dt = time.delta_secs();
    for (entity, target, mut pose, mut visibility) in &mut parts {
        let crate::crafting::CraftTarget::Part(id) = target else {
            continue;
        };
        if !id.ends_with(".gem") {
            continue;
        }
        let index = if id.contains("_fire.") {
            0
        } else if id.contains("_air.") {
            1
        } else if id.contains("_lightning.") {
            2
        } else {
            3
        };
        let stage = portals.crystals[index];
        *visibility = if matches!(stage, CrystalStage::Rising(_) | CrystalStage::Ready) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        let looking = players.iter().any(|(player, body, health)| {
            let eye = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
            let delta = pose.translation - eye;
            health.state() == LifeState::Standing
                && delta.length_squared() < 161.29
                && delta.normalize_or_zero().dot(
                    Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z,
                ) > 0.
        });
        portals.crystals[index] = match stage {
            CrystalStage::Plinth(t) if t <= dt => CrystalStage::Looking(0.),
            CrystalStage::Plinth(t) => CrystalStage::Plinth(t - dt),
            CrystalStage::Looking(t) if looking && t + dt >= 1. => {
                pose.translation.y -= 0.381;
                crate::audio::emit_loop(
                    &mut commands,
                    "zmb_squest_crystal_loop",
                    crate::audio::SoundSource::attached(entity),
                );
                portals.crystals_ready[index] = true;
                CrystalStage::Rising(2.)
            }
            CrystalStage::Looking(t) => CrystalStage::Looking(if looking { t + dt } else { 0. }),
            CrystalStage::Rising(t) => {
                pose.translation.y += 0.1905 * dt.min(t);
                pose.rotate_y(dt);
                if t <= dt {
                    CrystalStage::Ready
                } else {
                    CrystalStage::Rising(t - dt)
                }
            }
            CrystalStage::Ready => {
                pose.rotate_y(dt);
                CrystalStage::Ready
            }
            other => other,
        };
    }
    for (plinth, mut pose) in &mut plinths {
        let offset = match portals.crystals[plinth.element] {
            CrystalStage::Waiting => -1.9812,
            CrystalStage::Plinth(t) => -1.016 - t / 6. * 0.9652,
            _ => -1.016,
        };
        pose.translation = plinth.origin + Vec3::Y * offset;
    }
}

fn presentation(
    mut commands: Commands,
    portals: Res<Portals>,
    mut frames: Query<(Entity, &mut PortalFrame)>,
) {
    for (entity, mut frame) in &mut frames {
        let phase = if frame.exit {
            if portals.returns[frame.element] > 20. {
                PortalPhase::Opening(portals.returns[frame.element] - 20.)
            } else if portals.returns[frame.element] > 0. {
                PortalPhase::Open
            } else if portals.return_closing[frame.element] > 0. {
                PortalPhase::Closing(portals.return_closing[frame.element])
            } else {
                PortalPhase::Closed
            }
        } else {
            portals.entrances[frame.element]
        };
        let stage = match phase {
            PortalPhase::Closed => 0,
            PortalPhase::Opening(_) => 1,
            PortalPhase::Open => 2,
            PortalPhase::Closing(_) => 3,
        };
        if stage != frame.phase {
            frame.phase = stage;
            if let Some(sound) = frame.sound.take() {
                commands.entity(sound).try_despawn();
            }
            if stage != 2
                && let Some(effect) = frame.effect.take()
            {
                commands.entity(effect).try_despawn();
            }
            if stage == 1 || stage == 2 {
                frame.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    if stage == 1 {
                        "zmb_teleporter_loop_pre"
                    } else {
                        "zmb_teleporter_loop_post"
                    },
                    crate::audio::SoundSource::attached(entity),
                ));
            }
            if stage == 2 {
                frame.effect = Some(crate::native_fx::emit(
                    &mut commands,
                    format!(
                        "maps/zombie_tomb/fx_tomb_portal_{}",
                        ["fire", "air", "elec", "ice"][frame.element]
                    ),
                    Transform::from_translation(Vec3::from_array(if frame.exit {
                        source::EXITS[frame.element]
                    } else {
                        source::ENTRANCES[frame.element]
                    })),
                ));
            } else if stage == 3 {
                crate::audio::emit(
                    &mut commands,
                    "zmb_teleporter_anim_collapse_pew",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
        let (clip, seconds) = match phase {
            PortalPhase::Opening(t) => ("fxanim_zom_tomb_portal_open_anim", OPEN_SECONDS - t),
            PortalPhase::Open => ("fxanim_zom_tomb_portal_open_1frame_anim", 0.),
            PortalPhase::Closing(t) => ("fxanim_zom_tomb_portal_collapse_anim", CLOSE_SECONDS - t),
            PortalPhase::Closed => ("fxanim_zom_tomb_portal_collapse_anim", CLOSE_SECONDS),
        };
        commands
            .entity(entity)
            .insert(crate::animation::NativeAnimation {
                source: frame.source.clone(),
                clip: clip.into(),
                seconds,
                looping: false,
                restart: 0,
            });
    }
}
