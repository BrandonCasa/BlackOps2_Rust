use crate::{
    GameState,
    assets::GameDefinition,
    environment::parts::{PartState, Parts},
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    player::Player,
    weapons::WeaponInventory,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod source;
pub(crate) const SHIELD_HEALTH: u32 = 1500;

#[derive(Clone, serde::Deserialize)]
pub struct BenchDefinition {
    pub id: String,
    pub source_trigger: String,
    pub targetname: String,
    pub equipname: String,
    pub weaponname: String,
    pub hint: String,
    pub position: [f32; 3],
    pub angles: [f32; 3],
    pub dimensions: [f32; 3],
    pub use_seconds: f32,
    pub persistent: bool,
    pub delete_source: bool,
    pub require_look_at: bool,
}

#[derive(Component, Default)]
pub struct BuildableInventory {
    pub equipped: Option<String>,
    pub owned: BTreeSet<String>,
    pub shield_health: u32,
    pub shield_held: bool,
    pub shield_attack: f32,
    pub deployed_shield: Option<Entity>,
}
#[derive(Resource, Default)]
pub struct Crafting {
    pub crafted: BTreeSet<String>,
    pub consumed: BTreeSet<String>,
    pub records: BTreeSet<String>,
    pub gramophone: bool,
    benches: BTreeMap<usize, String>,
    bench_positions: Vec<Vec3>,
    selected: BTreeMap<usize, usize>,
    active: Option<Entity>,
    progress: f32,
    latched: bool,
    sound: Option<Entity>,
}
#[derive(Component)]
pub(crate) enum CraftTarget {
    Part(String),
    Bench(usize, String),
}
const SHIELD: &str = "tomb_shield_zm";
const DRONE: &str = "equip_dieseldrone_zm";
const STAFFS: &[(&str, &str)] = &[
    ("elemental_staff_fire", "weapon.staff_fire_zm"),
    ("elemental_staff_air", "weapon.staff_air_zm"),
    ("elemental_staff_water", "weapon.staff_water_zm"),
    ("elemental_staff_lightning", "weapon.staff_lightning_zm"),
];
fn part_id(name: &str) -> String {
    if let Some(family) = name.strip_suffix("_gem") {
        return format!("part.{family}.gem");
    }
    if let Some((family, piece)) = name.rsplit_once('_') {
        return format!("part.{family}.{piece}");
    }
    format!("part.{name}")
}
fn requirements(recipe: &str) -> Vec<String> {
    let pieces: &[&str] = if recipe == SHIELD {
        &["top", "door", "bracket"]
    } else if recipe == DRONE {
        &["body", "brain", "engine"]
    } else {
        &["upper_staff", "middle_staff", "lower_staff", "gem"]
    };
    pieces
        .iter()
        .map(|piece| format!("part.{recipe}.{piece}"))
        .collect()
}
fn available(recipe: &str, parts: &Parts) -> bool {
    requirements(recipe)
        .iter()
        .all(|id| matches!(parts.pieces.get(id), Some(PartState::Owned { .. })))
}
pub struct CraftingPlugin;
impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            recover_crystals
                .in_set(GameStep::PlayerLife)
                .run_if(in_state(GameState::Playing)),
        );
        app.init_resource::<Crafting>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                (setup, bind_parts)
                    .chain()
                    .in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (interact, bind_parts, craft_audio)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::stations::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut crafting: ResMut<Crafting>,
    mut parts: ResMut<Parts>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
) {
    *crafting = Crafting::default();
    for (name, sites) in source::PART_SITES {
        let position = Vec3::from_array(sites[session.random() as usize % sites.len()]);
        let id = part_id(name);
        let entity = commands
            .spawn((
                CraftTarget::Part(id.clone()),
                SessionEntity,
                Transform::from_translation(position),
                Visibility::Inherited,
            ))
            .id();
        parts.reveal(&id, entity);
    }
    for (index, bench) in game.environment.crafting_benches.iter().enumerate() {
        let position = Vec3::from_array(bench.position);
        crafting.bench_positions.push(position);
        commands.spawn((
            CraftTarget::Bench(index, bench.equipname.clone()),
            SessionEntity,
            Transform::from_translation(position),
        ));
    }
}
fn bind_parts(
    mut commands: Commands,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    parts: Query<(Entity, &CraftTarget, &Transform), Without<WorldAssetRoot>>,
) {
    for (entity, target, pose) in &parts {
        let CraftTarget::Part(id) = target else {
            continue;
        };
        let Some((_, model)) = source::PART_MODELS.iter().find(|(key, _)| *key == id) else {
            continue;
        };
        let Some(scene) = crate::assets::native_scene(&game, model) else {
            continue;
        };
        let mut transform = *pose;
        if let Some((_, angles)) = source::PART_ROTATIONS.iter().find(|(position, _)| {
            Vec3::from_array(*position).distance_squared(pose.translation) < 0.0004
        }) {
            transform.rotation = Quat::from_rotation_y(angles[1].to_radians())
                * Quat::from_rotation_z(-angles[0].to_radians())
                * Quat::from_rotation_x(angles[2].to_radians());
        }
        commands
            .entity(entity)
            .insert((WorldAssetRoot(server.load(scene.clone())), transform));
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    definition: Res<GameDefinition>,
    spatial: SpatialQuery,
    targets: Query<(Entity, &CraftTarget, &Transform), Without<Player>>,
    mut players: Query<
        (
            Entity,
            &Player,
            &Transform,
            &Vitality,
            &mut WeaponInventory,
            &mut BuildableInventory,
            &mut crate::equipment::GrenadeInventory,
            &mut crate::equipment::TacticalInventory,
            &mut crate::mines::MineInventory,
            Has<crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
    deployment: Res<crate::buildables::BuildableState>,
    reserved: Query<&crate::actions::HandReservation>,
    portals: Res<crate::portals::Portals>,
    upgrades: Res<crate::staff_upgrades::StaffUpgrades>,
    quest: Res<crate::quests::MainQuest>,
    mut crafting: ResMut<Crafting>,
    mut parts: ResMut<Parts>,
    mut focus: ResMut<InteractionFocus>,
) {
    if !input.use_held {
        crafting.latched = false;
        crafting.progress = 0.;
    }
    let Ok((
        owner,
        player,
        body,
        vitality,
        mut inventory,
        mut equipment,
        mut grenades,
        mut tactical,
        mut mines,
        drinking,
    )) = players.single_mut()
    else {
        return;
    };
    let building = reserved
        .get(owner)
        .is_ok_and(|reservation| reservation.action == crate::actions::HandAction::Crafting);
    if building && !input.use_held {
        commands
            .entity(owner)
            .remove::<crate::actions::HandReservation>();
    }
    if vitality.state() != LifeState::Standing || focus.target.is_some() {
        crafting.active = None;
        crafting.progress = 0.;
        if building {
            commands
                .entity(owner)
                .remove::<crate::actions::HandReservation>();
        }
        return;
    }
    let eye = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward =
        Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch) * Vec3::NEG_Z;
    let candidate = targets
        .iter()
        .filter_map(|(entity, target, pose)| {
            if let CraftTarget::Part(id) = target {
                if id.ends_with(".gem") {
                    let index = if id.contains("_fire.") {
                        0
                    } else if id.contains("_air.") {
                        1
                    } else if id.contains("_lightning.") {
                        2
                    } else {
                        3
                    };
                    if !portals.crystals_ready[index] {
                        return None;
                    }
                }
                if !matches!(parts.pieces.get(id), Some(PartState::Available { .. })) {
                    return None;
                }
            }
            let delta = pose.translation + Vec3::Y * 0.35 - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            if distance > 2.54
                || forward.dot(*direction) < 0.85
                || spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.12).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_some()
            {
                return None;
            }
            Some((entity, target, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, target, _)) = candidate else {
        crafting.progress = 0.;
        crafting.active = None;
        if building {
            commands
                .entity(owner)
                .remove::<crate::actions::HandReservation>();
        }
        return;
    };
    if crafting.active != Some(entity) {
        crafting.progress = 0.;
        crafting.active = Some(entity);
    }
    focus.target = Some(entity);
    let key = settings.label(crate::input::Action::Interact);
    let busy = (!building && reserved.contains(owner)) || drinking;
    match target {
        CraftTarget::Part(id) => {
            if building {
                commands
                    .entity(owner)
                    .remove::<crate::actions::HandReservation>();
                crafting.progress = 0.;
            }
            focus.prompt = format!(
                "Press {key} to collect {}",
                id.strip_prefix("part.").unwrap_or(id).replace('_', " ")
            );
            if input.use_held && !crafting.latched && !busy && parts.take(id, entity, owner) {
                crafting.latched = true;
                if id.starts_with("part.gramophone_vinyl.") {
                    let record = id.trim_start_matches("part.gramophone_vinyl.");
                    if record == "player" {
                        crafting.gramophone = true;
                    } else {
                        crafting.records.insert(record.into());
                    }
                }
                commands.entity(entity).despawn();
                crate::audio::emit(
                    &mut commands,
                    "zmb_craftable_pickup",
                    crate::audio::SoundSource::attached(owner),
                );
                let line = if id == "part.gramophone_vinyl.player" {
                    "pickup_gramophone"
                } else if id.starts_with("part.gramophone_vinyl.") {
                    "pickup_record"
                } else if id.starts_with("part.tomb_shield_zm.") {
                    "build_zs_pickup"
                } else if id == "part.equip_dieseldrone_zm.brain" {
                    "pickup_brain"
                } else if id.starts_with("part.equip_dieseldrone_zm.") {
                    "build_dd_pickup"
                } else if id.ends_with(".gem") {
                    "pickup_crystal"
                } else {
                    "pickup_staff_part"
                };
                crate::audio::voice(&mut commands, line, owner);
                focus.accepted += 1;
            }
        }
        CraftTarget::Bench(index, name) => {
            let Some(bench) = definition.environment.crafting_benches.get(*index) else {
                return;
            };
            let recipe = if name == "open_table" {
                if let Some(recipe) = crafting.benches.get(index) {
                    recipe.clone()
                } else {
                    let choices: Vec<_> = [SHIELD, DRONE]
                        .into_iter()
                        .filter(|r| !crafting.crafted.contains(*r))
                        .collect();
                    if choices.is_empty() {
                        if building {
                            commands
                                .entity(owner)
                                .remove::<crate::actions::HandReservation>();
                        }
                        focus.prompt = "Workbench unavailable".into();
                        return;
                    }
                    let selection = crafting.selected.entry(*index).or_default();
                    if input.buildable {
                        *selection = (*selection + 1) % choices.len();
                        crafting.progress = 0.;
                    }
                    choices[*crafting.selected.get(index).unwrap_or(&0) % choices.len()].to_string()
                }
            } else {
                name.clone()
            };
            let crafted = crafting.crafted.contains(&recipe);
            focus.prompt = if crafted {
                format!("Press {key} to take {}", recipe.replace('_', " "))
            } else if available(&recipe, &parts) {
                format!("Hold {key} to craft {}", recipe.replace('_', " "))
            } else {
                format!("{}: missing parts", recipe.replace('_', " "))
            };
            focus.hold_seconds = if crafted { 0. } else { bench.use_seconds };
            if !input.use_held || crafting.latched || busy {
                crafting.progress = 0.;
                if building {
                    commands
                        .entity(owner)
                        .remove::<crate::actions::HandReservation>();
                }
                return;
            }
            if !crafted {
                if !available(&recipe, &parts) {
                    if building {
                        commands
                            .entity(owner)
                            .remove::<crate::actions::HandReservation>();
                        crafting.progress = 0.;
                    }
                    focus.rejection = Some(InteractionRejection::Unavailable);
                    return;
                }
                if crafting.progress == 0. {
                    inventory.cancel_action();
                    grenades.cancel();
                    if let Some(tactical) = tactical.0.as_mut() {
                        tactical.cancel();
                    }
                    mines.cancel();
                    commands
                        .entity(owner)
                        .insert(crate::actions::HandReservation::held(
                            crate::actions::HandAction::Crafting,
                        ));
                }
                crafting.progress += time.delta_secs();
                focus.progress = crafting.progress;
                if crafting.progress < bench.use_seconds {
                    return;
                }
                for id in requirements(&recipe) {
                    parts.pieces.insert(id.clone(), PartState::Consumed);
                    crafting.consumed.insert(id);
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_buildable_complete",
                    crate::audio::SoundSource::attached(owner),
                );
                crafting.crafted.insert(recipe.clone());
                if recipe == DRONE {
                    commands.write_message(crate::audio::NarrativeEvent::DroneBuilt(
                        crate::audio::SoundSource::World(Vec3::from_array(bench.position)),
                    ));
                } else if recipe == SHIELD {
                    crate::audio::voice(&mut commands, "build_zs_final", owner);
                }
                crafting.benches.insert(*index, recipe.clone());
                crafting.progress = 0.;
                commands
                    .entity(owner)
                    .remove::<crate::actions::HandReservation>();
                crafting.latched = true;
                focus.accepted += 1;
                return;
            }
            if recipe == SHIELD || recipe == DRONE {
                if recipe == DRONE
                    && (deployment.drone_disabled
                        || deployment.drone.is_some()
                        || deployment.drone_cooldown > 0.)
                {
                    focus.rejection = Some(InteractionRejection::Unavailable);
                    return;
                }
                if equipment.owned.contains(&recipe) {
                    focus.rejection = Some(InteractionRejection::Duplicate);
                    return;
                }
                equipment.owned.insert(recipe.clone());
                if equipment.equipped.is_none() {
                    equipment.equipped = Some(recipe.clone());
                }
                if recipe == SHIELD {
                    if let Some(previous) = equipment.deployed_shield.take() {
                        commands.entity(previous).despawn();
                    }
                    equipment.shield_health = SHIELD_HEALTH;
                    equipment.shield_held = false;
                }
                crate::audio::voice(
                    &mut commands,
                    if recipe == SHIELD {
                        "build_zs_take"
                    } else {
                        "build_dd_take"
                    },
                    owner,
                );
            } else {
                let Some((_, weapon)) = STAFFS.iter().find(|(family, _)| *family == recipe) else {
                    return;
                };
                let Some(element) = crate::staff_upgrades::WEAPONS
                    .iter()
                    .position(|w| w == weapon)
                else {
                    return;
                };
                if quest
                    .placed
                    .contains(&crate::staff_upgrades::ELEMENTS[element])
                {
                    focus.prompt = "Staff is placed for the main quest".into();
                    return;
                }
                if upgrades.inserted[element] {
                    focus.prompt = "Staff is charging in the Crazy Place".into();
                    return;
                }
                let id = if upgrades.charged[element] {
                    weapon.replace("_zm", "_upgraded_zm")
                } else {
                    (*weapon).to_string()
                };
                let Some(d) = definition.content.weapons.iter().find(|d| d.id.0 == id) else {
                    return;
                };
                if inventory.owned().any(|(_, w)| w.definition == d.id) {
                    focus.rejection = Some(InteractionRejection::Duplicate);
                    return;
                }
                if inventory.exchange_staff(d).is_err() {
                    focus.rejection = Some(InteractionRejection::Busy);
                    return;
                }
                if !upgrades.charged[element] {
                    commands.write_message(crate::audio::NarrativeEvent::StaffTaken(element));
                }
            }
            crafting.latched = true;
            focus.accepted += 1;
        }
    }
}

impl Crafting {
    pub fn drone_home(&self) -> Option<Vec3> {
        self.benches
            .iter()
            .find(|(_, recipe)| recipe.as_str() == DRONE)
            .and_then(|(index, _)| self.bench_positions.get(*index))
            .copied()
    }
}

pub fn reveal_part(
    commands: &mut Commands,
    parts: &mut Parts,
    id: &str,
    position: Vec3,
) -> Option<Entity> {
    if parts.pieces.contains_key(id) {
        return None;
    }
    let entity = commands
        .spawn((
            CraftTarget::Part(id.into()),
            SessionEntity,
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id();
    parts.reveal(id, entity);
    Some(entity)
}

fn recover_crystals(
    mut commands: Commands,
    players: Query<(Entity, &Vitality), With<Player>>,
    mut parts: ResMut<Parts>,
) {
    for (owner, health) in &players {
        if health.state() != LifeState::HumanTerminal {
            continue;
        }
        for (name, positions) in source::PART_SITES
            .iter()
            .filter(|(name, _)| name.ends_with("_gem"))
        {
            let id = part_id(name);
            if matches!(parts.pieces.get(&id), Some(PartState::Owned { player }) if *player == owner.to_bits())
            {
                parts.pieces.remove(&id);
                reveal_part(
                    &mut commands,
                    &mut parts,
                    &id,
                    Vec3::from_array(positions[0]),
                );
            }
        }
    }
}

fn craft_audio(
    mut commands: Commands,
    mut crafting: ResMut<Crafting>,
    players: Query<Entity, With<Player>>,
) {
    if crafting.progress > 0. {
        if crafting.sound.is_none() {
            if let Ok(owner) = players.single() {
                crafting.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_craftable_loop",
                    crate::audio::SoundSource::attached(owner),
                ));
            }
        }
    } else if let Some(sound) = crafting.sound.take() {
        // Audio admission and native voice limits may already have retired it.
        commands.entity(sound).try_despawn();
    }
}
