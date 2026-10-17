use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    origins::content::WeaponId,
    player::Player,
    weapons::{AmmoSnapshot, WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;

const BOX_POSITIONS: [[f32; 3]; 6] = [
    [9.144, -3.2512, -64.6176],
    [73.66, -9.3472, -140.208],
    [-13.4112, -8.9408, -97.3328],
    [52.2224, 2.9972, -11.5824],
    [-72.5424, 5.8928, -4.4704],
    [28.448, 7.6708, 69.088],
];
const BOX_GENERATORS: [u32; 6] = [3, 1, 2, 4, 5, 6];
const BOX_YAWS: [f32; 6] = [0., 0., 90., 0., 90., 285.];

#[derive(Clone, Copy, PartialEq)]
enum BoxPose {
    Closed,
    Open,
    Leaving,
}

#[derive(Component)]
#[require(crate::animation::SceneClips)]
pub(crate) struct BoxVisual {
    index: usize,
    pose: BoxPose,
    elapsed: f32,
    open_seconds: Option<f32>,
}
impl BoxVisual {
    pub(crate) fn new(index: usize) -> Self {
        Self {
            index,
            pose: BoxPose::Closed,
            elapsed: f32::MAX,
            open_seconds: None,
        }
    }
}

pub(crate) fn present_boxes(
    time: Res<Time>,
    game: Res<GameDefinition>,
    stations: Res<Stations>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut assets: crate::animation::ClipAssets,
    mut boxes: Query<(
        &mut BoxVisual,
        &WorldAssetRoot,
        &mut Visibility,
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
) {
    for (mut visual, source, mut visibility, mut clips, mut player, mut graph) in &mut boxes {
        let Some(phase) = stations.boxes.get(visual.index) else {
            continue;
        };
        let desired = match phase {
            BoxPhase::Rolling { .. } | BoxPhase::Reward { .. } => BoxPose::Open,
            BoxPhase::Relocating { .. } => BoxPose::Leaving,
            BoxPhase::Idle => BoxPose::Closed,
        };
        if visual.pose != desired {
            visual.pose = desired;
            visual.elapsed = 0.;
        }
        let visible = visual.index >= BOX_POSITIONS.len()
            || stations.normal_box == visual.index
            || effects.fire_sale > 0.
            || desired != BoxPose::Closed;
        if !visible || !clips.ready {
            *visibility = Visibility::Hidden;
            continue;
        }
        let name = match desired {
            BoxPose::Open => "o_zombie_dlc4_magic_box_open",
            BoxPose::Closed => "o_zombie_dlc4_magic_box_close",
            BoxPose::Leaving => "o_zombie_dlc4_magic_box_leave",
        };
        let scene = source.0.path().expect("Mystery Box scene");
        let paths = &game.scene_animations[scene.path().to_str().expect("Mystery Box scene path")];
        let path = paths[name]
            .as_deref()
            .expect("Mystery Box source animation");
        let duration = if desired == BoxPose::Open {
            visual.open_seconds
        } else {
            None
        };
        let Some(duration) = duration.or_else(|| {
            clips
                .node(path, &mut graph, &mut assets)
                .map(|(_, duration)| duration)
        }) else {
            *visibility = Visibility::Hidden;
            continue;
        };
        if desired == BoxPose::Open {
            visual.open_seconds = Some(duration);
        }
        let (path, elapsed, looping) = if desired == BoxPose::Open && visual.elapsed >= duration {
            (
                paths["o_zombie_dlc4_magic_box_open_idle"]
                    .as_deref()
                    .expect("Mystery Box open idle"),
                visual.elapsed - duration,
                true,
            )
        } else {
            (path, visual.elapsed.min(duration), false)
        };
        *visibility = if clips.sample(path, elapsed, looping, &mut player, &mut graph, &mut assets)
        {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        visual.elapsed += time.delta_secs();
    }
}
const BOX_WEAPONS: &[&str] = &[
    "hamr_zm",
    "mg08_zm",
    "type95_zm",
    "galil_zm",
    "fnfal_zm",
    "scar_zm",
    "ksg_zm",
    "srm1216_zm",
    "ak74u_extclip_zm",
    "pdw57_zm",
    "thompson_zm",
    "qcw05_zm",
    "mp40_stalker_zm",
    "evoskorpion_zm",
    "dsr50_zm",
    "beretta93r_extclip_zm",
    "kard_zm",
    "python_zm",
    "fivesevendw_zm",
    "m32_zm",
    "cymbal_monkey_zm",
    "ray_gun_zm",
    "raygun_mark2_zm",
];
#[derive(Component, Clone, Copy)]
pub(crate) enum Station {
    Box(usize),
    PackAPunch,
}
#[derive(Clone)]
enum BoxPhase {
    Idle,
    Rolling {
        owner: Entity,
        weapon: WeaponId,
        remaining: f32,
        teddy: bool,
        normal: bool,
        refund: u32,
    },
    Reward {
        owner: Entity,
        weapon: WeaponId,
        remaining: f32,
        normal: bool,
    },
    Relocating {
        remaining: f32,
    },
}
#[derive(Clone)]
enum UpgradePhase {
    Idle,
    Processing {
        owner: Entity,
        weapon: WeaponId,
        remaining: f32,
    },
    Ready {
        owner: Entity,
        weapon: WeaponId,
        remaining: f32,
    },
}
#[derive(Resource)]
pub struct Stations {
    pub normal_box: usize,
    pub moves: u32,
    pub uses: u32,
    pub purchases: u32,
    pub collections: u32,
    pub refunds: u32,
    boxes: Vec<BoxPhase>,
    free_boxes: Vec<usize>,
    upgrade: UpgradePhase,
    attachment: Option<String>,
    deposited: Option<AmmoSnapshot>,
    latched: bool,
}
impl Default for Stations {
    fn default() -> Self {
        Self {
            normal_box: 0,
            moves: 0,
            uses: 0,
            purchases: 0,
            collections: 0,
            refunds: 0,
            boxes: vec![BoxPhase::Idle; 6],
            free_boxes: Vec::new(),
            upgrade: UpgradePhase::Idle,
            attachment: None,
            deposited: None,
            latched: false,
        }
    }
}
impl Stations {
    pub(crate) fn add_sandbox_box(&mut self) -> usize {
        if let Some(index) = self.free_boxes.pop() {
            return index;
        }
        let index = self.boxes.len();
        self.boxes.push(BoxPhase::Idle);
        index
    }
    pub(crate) fn remove_sandbox_box(&mut self, index: usize) {
        if index >= 6 && index < self.boxes.len() && !self.free_boxes.contains(&index) {
            self.boxes[index] = BoxPhase::Idle;
            self.free_boxes.push(index);
        }
    }
}
fn attachments(id: &WeaponId) -> (&'static str, &'static [&'static str]) {
    match id.0.strip_prefix("weapon.").unwrap_or(&id.0) {
        "ak74u_upgraded_zm" => ("reflex", &[]),
        "fnfal_upgraded_zm" => ("reflex", &[]),
        "galil_upgraded_zm" => ("reflex", &[]),
        "barretm82_upgraded_zm" => ("vzoom", &[]),
        "judge_upgraded_zm" => ("none", &["extbarrel", "dualclip_judge", "dw"]),
        "kard_upgraded_zm" => ("none", &["extbarrel", "dualclip"]),
        "fiveseven_upgraded_zm" => ("none", &["extbarrel", "dualclip"]),
        "beretta93r_upgraded_zm" => ("none", &["extbarrel", "dualclip"]),
        "qcw05_upgraded_zm" => ("none", &["reflex", "grip", "dualclip", "sf"]),
        "870mcs_upgraded_zm" => ("none", &["reflex", "extbarrel"]),
        "saiga12_upgraded_zm" => ("none", &["reflex", "extbarrel"]),
        "srm1216_upgraded_zm" => ("none", &["reflex", "extbarrel"]),
        "saritch_upgraded_zm" => ("dualoptic", &["reflex", "rangefinder", "mms"]),
        "xm8_upgraded_zm" => ("reflex", &["longbreath", "rangefinder", "gl", "mms"]),
        "type95_upgraded_zm" => ("reflex", &["longbreath", "rangefinder", "gl", "mms"]),
        "tar21_upgraded_zm" => ("reflex", &["longbreath", "rangefinder", "gl", "mms"]),
        "hamr_upgraded_zm" => ("none", &["reflex", "grip", "acog"]),
        "dsr50_upgraded_zm" => ("vzoom", &["is", "silencer", "vzoom"]),
        "hk416_upgraded_zm" => ("reflex", &["longbreath", "rangefinder", "mms"]),
        "lsat_upgraded_zm" => ("rangefinder", &["reflex", "grip", "acog"]),
        "an94_upgraded_zm" => ("reflex", &["grip", "rangefinder", "mms"]),
        "pdw57_upgraded_zm" => ("none", &["reflex", "rangefinder", "mms"]),
        "svu_upgraded_zm" => ("vzoom", &[]),
        "ballista_upgraded_zm" => ("acog", &["is", "acog"]),
        "ksg_upgraded_zm" => ("reflex", &[]),
        "evoskorpion_upgraded_zm" => ("reflex", &["rangefinder", "dualclip", "reflex"]),
        "scar_upgraded_zm" => ("reflex", &["rangefinder", "dualclip", "acog"]),
        "ak74u_extclip_upgraded_zm" => ("reflex", &[]),
        "beretta93r_extclip_upgraded_zm" => ("none", &["extbarrel", "dualclip"]),
        _ => ("none", &[]),
    }
}
pub struct StationPlugin;
impl Plugin for StationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Stations>()
            .add_systems(Update, present_boxes.run_if(in_state(GameState::Playing)))
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (tick, interact)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut stations: ResMut<Stations>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    *stations = Stations::default();
    stations.normal_box = [0, 2][session.random() as usize % 2];
    for (index, position) in BOX_POSITIONS.iter().enumerate() {
        let scene = crate::assets::native_scene(&game, "p6_anim_zm_tm_magic_box")
            .expect("Origins Mystery Box model");
        commands
            .spawn((
                Station::Box(index),
                SessionEntity,
                Transform::from_translation(Vec3::from_array(*position))
                    .with_rotation(Quat::from_rotation_y(BOX_YAWS[index].to_radians())),
                Visibility::Inherited,
            ))
            .with_children(|parent| {
                parent.spawn((
                    BoxVisual::new(index),
                    WorldAssetRoot(server.load(scene.clone())),
                    Transform::IDENTITY,
                    Visibility::Hidden,
                ));
            });
    }
    commands.spawn((
        Station::PackAPunch,
        SessionEntity,
        Transform::from_xyz(-0.1397, 8.5217, 0.2159),
    ));
}
fn tick(
    time: Res<Time<Fixed>>,
    mut stations: ResMut<Stations>,
    mut session: ResMut<Session>,
    mut digging: ResMut<crate::environment::digging::Digging>,
) {
    let dt = time.delta_secs();
    for index in 0..stations.boxes.len() {
        stations.boxes[index] = match std::mem::replace(&mut stations.boxes[index], BoxPhase::Idle)
        {
            BoxPhase::Rolling {
                owner,
                weapon,
                mut remaining,
                teddy,
                normal,
                refund,
            } => {
                remaining -= dt;
                if remaining > 0. {
                    BoxPhase::Rolling {
                        owner,
                        weapon,
                        remaining,
                        teddy,
                        normal,
                        refund,
                    }
                } else if teddy {
                    if refund > 0 {
                        session.points = session.points.saturating_add(refund);
                        stations.refunds += 1;
                    }
                    BoxPhase::Relocating { remaining: 7. }
                } else {
                    BoxPhase::Reward {
                        owner,
                        weapon,
                        remaining: 12.,
                        normal,
                    }
                }
            }
            BoxPhase::Reward {
                owner,
                weapon,
                remaining,
                normal,
            } if remaining > dt => BoxPhase::Reward {
                owner,
                weapon,
                remaining: remaining - dt,
                normal,
            },
            BoxPhase::Reward { normal, .. } => {
                stations.uses += u32::from(normal);
                BoxPhase::Idle
            }
            BoxPhase::Relocating { remaining } if remaining > dt => BoxPhase::Relocating {
                remaining: remaining - dt,
            },
            BoxPhase::Relocating { .. } => {
                stations.normal_box = (index + 1 + session.random() as usize % 5) % 6;
                stations.moves += 1;
                stations.uses = 0;
                digging.box_moved = true;
                BoxPhase::Idle
            }
            _ => BoxPhase::Idle,
        };
    }
    stations.upgrade = match std::mem::replace(&mut stations.upgrade, UpgradePhase::Idle) {
        UpgradePhase::Processing {
            owner,
            weapon,
            remaining,
        } if remaining > dt => UpgradePhase::Processing {
            owner,
            weapon,
            remaining: remaining - dt,
        },
        UpgradePhase::Processing { owner, weapon, .. } => UpgradePhase::Ready {
            owner,
            weapon,
            remaining: 15.,
        },
        UpgradePhase::Ready {
            owner,
            weapon,
            remaining,
        } if remaining > dt => UpgradePhase::Ready {
            owner,
            weapon,
            remaining: remaining - dt,
        },
        _ => UpgradePhase::Idle,
    };
}

fn box_candidates(
    definition: &GameDefinition,
    inventory: &WeaponInventory,
    tactical: &crate::equipment::TacticalInventory,
    round: u32,
    beacon_unlocked: bool,
) -> Vec<(WeaponId, u32)> {
    let ray_owned = inventory.owned().any(|(_, owned)| {
        owned.definition.0.starts_with("weapon.ray_gun")
            || owned.definition.0.starts_with("weapon.raygun_mark2")
    });
    BOX_WEAPONS
        .iter()
        .copied()
        .chain(beacon_unlocked.then_some("beacon_zm"))
        .filter_map(|id| {
            let weapon = definition
                .content
                .weapons
                .iter()
                .find(|w| w.id.0.strip_prefix("weapon.").unwrap_or(&w.id.0) == id)?;
            if inventory.owned().any(|(_, owned)| {
                owned.definition == weapon.id || weapon.upgraded.as_ref() == Some(&owned.definition)
            }) {
                return None;
            }
            if ray_owned && matches!(id, "ray_gun_zm" | "raygun_mark2_zm") {
                return None;
            }
            let weight = if id == "cymbal_monkey_zm" {
                if tactical
                    .0
                    .as_ref()
                    .is_some_and(|t| t.definition == weapon.id)
                {
                    1
                } else if round < 10 {
                    3
                } else {
                    5
                }
            } else {
                1
            };
            Some((weapon.id.clone(), weight))
        })
        .collect()
}
fn roll(candidates: &[(WeaponId, u32)], session: &mut Session) -> Option<WeaponId> {
    let total: u32 = candidates.iter().map(|(_, weight)| weight).sum();
    if total == 0 {
        return None;
    }
    loop {
        let mut pick = session.random() % total;
        for (weapon, weight) in candidates {
            if pick < *weight {
                if weapon.0 == "weapon.raygun_mark2_zm" && session.random() % 100 >= 33 {
                    break;
                }
                return Some(weapon.clone());
            }
            pick -= weight;
        }
    }
}
fn can_take(action: WeaponAction) -> bool {
    matches!(
        action,
        WeaponAction::Ready | WeaponAction::Reloading { .. } | WeaponAction::ShellReload { .. }
    )
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    definition: Res<GameDefinition>,
    beacons: Res<crate::tablets::TabletProgress>,
    reserved: Query<(), With<crate::actions::HandReservation>>,
    effects: Res<crate::powerups::PowerupEffects>,
    generators: Res<crate::generators::Generators>,
    query: SpatialQuery,
    targets: Query<(Entity, &Station, &Transform), Without<Player>>,
    mut players: Query<
        (
            Entity,
            &Player,
            &Transform,
            &Vitality,
            &mut WeaponInventory,
            &mut crate::equipment::TacticalInventory,
            &crate::equipment::GrenadeInventory,
            &crate::mines::MineInventory,
            Has<crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
    mut stations: ResMut<Stations>,
    mut focus: ResMut<InteractionFocus>,
    mut session: ResMut<Session>,
) {
    if !input.use_held {
        stations.latched = false;
    }
    let Ok((
        player_entity,
        player,
        body,
        vitality,
        mut inventory,
        mut tactical,
        grenades,
        mines,
        drinking,
    )) = players.single_mut()
    else {
        return;
    };
    if vitality.state() != LifeState::Standing {
        stations.latched = input.use_held;
        return;
    }
    if focus.target.is_some() {
        return;
    }
    let origin = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x(player.pitch + inventory.recoil.y)
        * Vec3::NEG_Z;
    let sale = effects.fire_sale > 0.;
    let candidate = targets
        .iter()
        .filter_map(|(entity, station, transform)| {
            if let Station::Box(index) = station {
                if *index < 6
                    && *index != stations.normal_box
                    && !sale
                    && matches!(stations.boxes[*index], BoxPhase::Idle)
                {
                    return None;
                }
            }
            let delta = transform.translation + Vec3::Y * 0.8 - origin;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            if distance > 2.2
                || forward.dot(*direction) < 0.85
                || query
                    .cast_ray(
                        origin,
                        direction,
                        (distance - 0.12).max(0.),
                        true,
                        &crate::environment::solids([player_entity]),
                    )
                    .is_some()
            {
                return None;
            }
            Some((entity, *station, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, station, _)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    let key = settings.label(crate::input::Action::Interact);
    let offhand_busy = tactical.busy()
        || grenades.busy()
        || mines.busy()
        || drinking
        || reserved.contains(player_entity);
    let busy = inventory.action != WeaponAction::Ready || offhand_busy;
    let take_busy = !can_take(inventory.action) || offhand_busy;
    let pressed = input.use_held && !stations.latched;
    if pressed {
        stations.latched = true;
    }
    match station {
        Station::Box(index) => {
            let phase = stations.boxes[index].clone();
            match phase {
                BoxPhase::Idle => {
                    let powered = index >= 6
                        || definition
                            .content
                            .generators
                            .iter()
                            .find(|g| {
                                BOX_GENERATORS
                                    .get(index)
                                    .is_some_and(|number| g.number as u32 == *number)
                            })
                            .and_then(|g| generators.sites.get(&g.id))
                            .is_some_and(|site| {
                                site.state == crate::generators::GeneratorState::Active
                            });
                    if !powered {
                        focus.prompt = "Mystery Box requires its local generator".into();
                        focus.rejection = Some(InteractionRejection::NoPower);
                        return;
                    }
                    let price = if sale { 10 } else { 950 };
                    focus.prompt = format!("Press {key} for Mystery Box [Cost: {price}]");
                    if !pressed {
                        return;
                    }
                    if busy {
                        focus.rejection = Some(InteractionRejection::Busy);
                        return;
                    }
                    let candidates = box_candidates(
                        &definition,
                        &inventory,
                        &tactical,
                        session.round,
                        beacons.beacon_unlocked,
                    );
                    if candidates.is_empty() {
                        focus.rejection = Some(InteractionRejection::Unavailable);
                        return;
                    }

                    if let Err(error) =
                        session.purchase(price, || Ok::<_, InteractionRejection>(()))
                    {
                        focus.rejection = Some(error);
                        return;
                    }
                    let weapon = roll(&candidates, &mut session).expect("nonempty box candidates");
                    let uses = stations.uses;
                    let chance = if uses < 4 {
                        0
                    } else if stations.moves == 0 && uses >= 8 {
                        100
                    } else if uses < 8 {
                        15
                    } else if uses < 13 {
                        30
                    } else {
                        50
                    };
                    let teddy = index < 6 && !sale && session.random() % 100 < chance;
                    crate::audio::emit(
                        &mut commands,
                        "music_chest",
                        crate::audio::SoundSource::attached(entity),
                    );
                    stations.boxes[index] = BoxPhase::Rolling {
                        owner: player_entity,
                        weapon,
                        remaining: 4.,
                        teddy,
                        normal: index < 6 && !sale,
                        refund: price,
                    };
                    stations.purchases += 1;
                    crate::audio::voice(&mut commands, "use_box", player_entity);
                    focus.accepted += 1;
                }
                BoxPhase::Reward {
                    owner,
                    weapon,
                    normal,
                    ..
                } => {
                    let Some(d) = definition.content.weapons.iter().find(|d| d.id == weapon) else {
                        return;
                    };
                    focus.prompt = format!("Press {key} to take {}", d.name);
                    if pressed && !take_busy && owner == player_entity {
                        let accepted = if d.throwable.is_some() {
                            tactical.receive(d).is_ok()
                        } else {
                            inventory.receive(d).is_ok()
                        };
                        if accepted {
                            stations.uses += u32::from(normal);
                            stations.boxes[index] = BoxPhase::Idle;
                            stations.collections += 1;
                            crate::audio::voice(&mut commands, "take_box_weapon", player_entity);
                            focus.accepted += 1;
                        } else {
                            focus.rejection = Some(InteractionRejection::Unavailable);
                        }
                    }
                }
                BoxPhase::Rolling { .. } => focus.prompt = "Mystery Box rolling".into(),
                BoxPhase::Relocating { .. } => focus.prompt = "Mystery Box moving".into(),
            }
        }
        Station::PackAPunch => match stations.upgrade.clone() {
            UpgradePhase::Idle => {
                if generators.active_count() != 6 {
                    focus.prompt = "Pack-a-Punch requires all six generators".into();
                    focus.rejection = Some(InteractionRejection::NoPower);
                    return;
                }
                let current = inventory.definition(&definition.content.weapons);
                let repeat =
                    current.is_some_and(|d| attachments(&d.id).1.len() > 1 && d.upgraded.is_none());
                let price = if repeat { 2000 } else { 5000 };
                focus.prompt = format!("Press {key} to Pack-a-Punch [Cost: {price}]");
                if !pressed {
                    return;
                }
                if busy {
                    focus.rejection = Some(InteractionRejection::Busy);
                    return;
                }
                let upgraded = current.and_then(|d| {
                    if repeat {
                        Some(d.id.clone())
                    } else {
                        d.upgraded.clone()
                    }
                });
                let Some(weapon) = upgraded else {
                    focus.rejection = Some(InteractionRejection::Unavailable);
                    return;
                };
                let old_ammo = current.and_then(|d| inventory.ammo_snapshot(d));
                let previous = inventory.attachment().to_string();
                let result = session
                    .purchase(price, || {
                        inventory
                            .deposit_active()
                            .map_err(|_| InteractionRejection::Busy)
                    })
                    .map(|_| {
                        let (default, addons) = attachments(&weapon);
                        let selected = if repeat {
                            loop {
                                let selected = addons[session.random() as usize % addons.len()];
                                if selected != previous {
                                    break selected;
                                }
                            }
                        } else {
                            default
                        };
                        (selected != "none").then(|| selected.to_string())
                    });
                match result {
                    Ok(attachment) => {
                        stations.attachment = attachment;
                        stations.deposited = if repeat { old_ammo } else { None };
                        inventory.cancel_action();
                        let seconds = definition
                            .content
                            .weapons
                            .iter()
                            .find(|d| d.id.0 == "weapon.zombie_knuckle_crack")
                            .map_or(2.1, |d| d.mechanics.equip_seconds);
                        commands.entity(player_entity).insert(
                            crate::actions::HandReservation::timed(
                                crate::actions::HandAction::Station,
                                seconds,
                            ),
                        );
                        if inventory.active().is_none() {
                            if let Some(fists) = definition
                                .content
                                .weapons
                                .iter()
                                .find(|d| d.id.0 == "weapon.zombie_fists_zm")
                            {
                                let _ = inventory.grant(fists);
                            }
                        }
                        stations.upgrade = UpgradePhase::Processing {
                            owner: player_entity,
                            weapon,
                            remaining: 3.35,
                        };
                        stations.purchases += 1;
                        focus.accepted += 1;
                    }
                    Err(error) => focus.rejection = Some(error),
                }
            }
            UpgradePhase::Processing { .. } => focus.prompt = "Pack-a-Punch processing".into(),
            UpgradePhase::Ready {
                owner,
                weapon,
                remaining,
            } => {
                let Some(d) = definition.content.weapons.iter().find(|d| d.id == weapon) else {
                    return;
                };
                focus.prompt = format!("Press {key} to collect {} [{remaining:.0}s]", d.name);
                if pressed && !take_busy && owner == player_entity && inventory.receive(d).is_ok() {
                    if let Some(snapshot) = stations.deposited.take() {
                        inventory.restore_received(d, &snapshot);
                    } else {
                        inventory.fill_received(d);
                    }
                    inventory.set_attachment(&d.id, stations.attachment.take());
                    stations.upgrade = UpgradePhase::Idle;
                    stations.collections += 1;
                    focus.accepted += 1;
                }
            }
        },
    }
}

#[cfg(test)]
#[test]
fn native_box_focus_preserves_roll_reward_and_relocation() {
    use bevy::ecs::system::RunSystemOnce;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        TransformPlugin,
        AssetPlugin::default(),
        PhysicsPlugins::new(Update),
    ))
    .init_asset::<Mesh>()
    .init_resource::<GameInput>()
    .init_resource::<crate::input::ControlSettings>()
    .init_resource::<crate::tablets::TabletProgress>()
    .init_resource::<crate::powerups::PowerupEffects>()
    .init_resource::<crate::generators::Generators>()
    .init_resource::<Stations>()
    .init_resource::<InteractionFocus>()
    .init_resource::<Session>();
    app.insert_resource(
        serde_json::from_str::<GameDefinition>(include_str!("../gameplay/origins/game.json"))
            .unwrap(),
    );
    let grenade = crate::equipment::ThrowableDefinition {
        slot: crate::equipment::ThrowableSlot::Lethal,
        initial: 4,
        capacity: 4,
        round_award: 2,
        prime_seconds: 0.3,
        throw_seconds: 0.5,
        release_delay: 0.1,
        fuse_seconds: 4.,
        speed: 10.,
        up_speed: 0.,
        relative_up_speed: 0.,
        rotating: true,
        radius: 5.,
        inner_radius: 0.,
        inner_damage: 100,
        outer_damage: 10,
        tangent_bounce: [0.5; 32],
        normal_bounce: [0.5; 32],
        collision_radius: 0.1,
        collision_half_extents: None,
        native_launch: None,
        cook_in_hand: true,
        sticky: false,
    };
    let owner = app
        .world_mut()
        .spawn((
            Player {
                yaw: 0.,
                pitch: 0.,
                velocity: Vec3::ZERO,
                height: 1.8,
                grounded: true,
                crouched: false,
                prone: false,
                eye_height: 1.62,
            },
            Transform::from_xyz(0., 0.9, 0.),
            Vitality::human(
                100,
                100,
                1,
                crate::life::PlayerLethalPolicy::SoloOrLastEligiblePlayer,
            )
            .unwrap(),
            WeaponInventory::default(),
            crate::equipment::TacticalInventory::default(),
            crate::equipment::GrenadeInventory::new(
                WeaponId("weapon.frag_grenade_zm".into()),
                &grenade,
            ),
            crate::mines::MineInventory::default(),
        ))
        .id();
    app.world_mut()
        .spawn((Station::Box(0), Transform::from_xyz(0., 0.82, -1.)));
    app.finish();
    app.cleanup();
    app.update();
    let weapon = WeaponId("weapon.c96_zm".into());
    for phase in [
        BoxPhase::Rolling {
            owner,
            weapon: weapon.clone(),
            remaining: 4.,
            teddy: false,
            normal: true,
            refund: 950,
        },
        BoxPhase::Reward {
            owner,
            weapon: weapon.clone(),
            remaining: 12.,
            normal: true,
        },
        BoxPhase::Relocating { remaining: 7. },
    ] {
        let expected = std::mem::discriminant(&phase);
        app.world_mut().resource_mut::<Stations>().boxes[0] = phase;
        for _ in 0..5 {
            *app.world_mut().resource_mut::<InteractionFocus>() = default();
            app.world_mut().run_system_once(interact).unwrap();
            assert!(app.world().resource::<InteractionFocus>().target.is_some());
            assert_eq!(
                std::mem::discriminant(&app.world().resource::<Stations>().boxes[0]),
                expected
            );
        }
    }
    assert_eq!(app.world().resource::<Session>().points, 500);
}
