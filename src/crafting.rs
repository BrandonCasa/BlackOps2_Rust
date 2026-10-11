use crate::{assets::GameDefinition, environment::parts::{PartState, Parts}, gameplay::{GameStep, Session, SessionEntity}, input::GameInput, interactions::{InteractionFocus, InteractionRejection}, life::{LifeState, Vitality}, player::Player, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod source;

#[derive(Component, Default)]
pub struct BuildableInventory {
    pub equipped: Option<String>,
    pub owned: BTreeSet<String>,
    pub shield_health: u32,
    pub shield_held: bool,
}
#[derive(Resource, Default)]
pub struct Crafting {
    pub crafted: BTreeSet<String>,
    pub consumed: BTreeSet<String>,
    pub records: BTreeSet<String>,
    pub gramophone: bool,
    benches: BTreeMap<usize, String>,
    selected: BTreeMap<usize, usize>,
    active: Option<Entity>,
    progress: f32,
    latched: bool,
}
#[derive(Component)]
enum CraftTarget { Part(String), Bench(usize, String) }
const SHIELD: &str = "tomb_shield_zm";
const DRONE: &str = "equip_dieseldrone_zm";
const STAFFS: &[(&str, &str)] = &[("elemental_staff_fire", "weapon.staff_fire_zm"), ("elemental_staff_air", "weapon.staff_air_zm"), ("elemental_staff_water", "weapon.staff_water_zm"), ("elemental_staff_lightning", "weapon.staff_lightning_zm")];
fn part_id(name: &str) -> String {
    if let Some(family) = name.strip_suffix("_gem") { return format!("part.{family}.gem"); }
    if let Some((family, piece)) = name.rsplit_once('_') { return format!("part.{family}.{piece}"); }
    format!("part.{name}")
}
fn requirements(recipe: &str) -> Vec<String> {
    let pieces: &[&str] = if recipe == SHIELD { &["top", "door", "bracket"] }
        else if recipe == DRONE { &["body", "brain", "engine"] }
        else { &["upper_staff", "middle_staff", "lower_staff", "gem"] };
    pieces.iter().map(|piece| format!("part.{recipe}.{piece}")).collect()
}
fn available(recipe: &str, parts: &Parts) -> bool {
    requirements(recipe).iter().all(|id| matches!(parts.pieces.get(id), Some(PartState::Owned { .. })))
}
pub struct CraftingPlugin;
impl Plugin for CraftingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Crafting>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, interact.in_set(GameStep::Interactions).after(crate::stations::interact)
                .run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut crafting: ResMut<Crafting>, mut parts: ResMut<Parts>, mut session: ResMut<Session>,
    definition: Res<GameDefinition>, server: Res<AssetServer>) {
    *crafting = Crafting::default();
    for (name, sites) in source::PART_SITES {
        let position = Vec3::from_array(sites[session.random() as usize % sites.len()]);
        let id = part_id(name);
        let entity = commands.spawn((CraftTarget::Part(id.clone()), SessionEntity, Transform::from_translation(position), Visibility::Inherited)).id();
        parts.reveal(&id, entity);
        if let Some(scene) = definition.content.presentations.iter().find(|p| p.id.0.contains(name)).and_then(|p| p.scene.as_ref()) {
            commands.entity(entity).insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    for (index, (name, position)) in source::BENCHES.iter().enumerate() {
        commands.spawn((CraftTarget::Bench(index, (*name).into()), SessionEntity, Transform::from_translation(Vec3::from_array(*position))));
    }
}
pub(crate) fn interact(
    mut commands: Commands, time: Res<Time<Fixed>>, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>,
    definition: Res<GameDefinition>, spatial: SpatialQuery,
    targets: Query<(Entity, &CraftTarget, &Transform), Without<Player>>,
    mut players: Query<(Entity, &Player, &Transform, &Vitality, &mut WeaponInventory, &mut BuildableInventory, &crate::equipment::GrenadeInventory, &crate::equipment::TacticalInventory, &crate::mines::MineInventory, Has<crate::perks::PerkDrinking>), With<Player>>,
    deployment: Res<crate::buildables::BuildableState>,
    mut crafting: ResMut<Crafting>, mut parts: ResMut<Parts>, mut focus: ResMut<InteractionFocus>,
) {
    if !input.use_held { crafting.latched = false; crafting.progress = 0.; }
    let Ok((owner, player, body, vitality, mut inventory, mut equipment, grenades, tactical, mines, drinking)) = players.single_mut() else { return; };
    if vitality.state() != LifeState::Standing || focus.target.is_some() {
        crafting.active = None; crafting.progress = 0.; return;
    }
    let eye = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch) * Vec3::NEG_Z;
    let candidate = targets.iter().filter_map(|(entity, target, pose)| {
        if let CraftTarget::Part(id) = target {
            if !matches!(parts.pieces.get(id), Some(PartState::Available { .. })) { return None; }
        }
        let delta = pose.translation + Vec3::Y * 0.35 - eye;
        let distance = delta.length();
        let direction = Dir3::new(delta).ok()?;
        if distance > 2.54 || forward.dot(*direction) < 0.85 || spatial.cast_ray(eye, direction, (distance - 0.12).max(0.), true, &crate::environment::solids([owner])).is_some() { return None; }
        Some((entity, target, distance))
    }).min_by(|a,b| a.2.total_cmp(&b.2));
    let Some((entity, target, _)) = candidate else { crafting.progress = 0.; crafting.active = None; return; };
    if crafting.active != Some(entity) { crafting.progress = 0.; crafting.active = Some(entity); }
    focus.target = Some(entity);
    let key = settings.label(crate::input::Action::Interact);
    let busy = inventory.action != WeaponAction::Ready || grenades.busy() || tactical.busy() || mines.busy() || drinking;
    match target {
        CraftTarget::Part(id) => {
            focus.prompt = format!("Press {key} to collect {}", id.strip_prefix("part.").unwrap_or(id).replace('_', " "));
            if input.use_held && !crafting.latched && !busy && parts.take(id, entity, owner) {
                crafting.latched = true;
                if id.starts_with("part.gramophone_vinyl.") {
                    let record = id.trim_start_matches("part.gramophone_vinyl.");
                    if record == "player" { crafting.gramophone = true; } else { crafting.records.insert(record.into()); }
                }
                commands.entity(entity).despawn();
                crate::audio::emit(&mut commands, "zmb_craftable_pickup", crate::audio::SoundSource::attached(owner));
                focus.accepted += 1;
            }
        }
        CraftTarget::Bench(index, name) => {
            let recipe = if name == "open_craftable_trigger" {
                if let Some(recipe) = crafting.benches.get(index) { recipe.clone() }
                else {
                    let choices: Vec<_> = [SHIELD, DRONE].into_iter().filter(|r| !crafting.crafted.contains(*r)).collect();
                    if choices.is_empty() { focus.prompt = "Workbench unavailable".into(); return; }
                    let selection = crafting.selected.entry(*index).or_default();
                    if input.buildable { *selection = (*selection + 1) % choices.len(); crafting.progress = 0.; }
                    choices[*crafting.selected.get(index).unwrap_or(&0) % choices.len()].to_string()
                }
            } else { format!("elemental_{}", name.trim_end_matches("_craftable_trigger")) };
            let crafted = crafting.crafted.contains(&recipe);
            focus.prompt = if crafted { format!("Press {key} to take {}", recipe.replace('_', " ")) }
                else if available(&recipe, &parts) { format!("Hold {key} to craft {}", recipe.replace('_', " ")) }
                else { format!("{}: missing parts", recipe.replace('_', " ")) };
            focus.hold_seconds = if crafted { 0. } else { 3. };
            if !input.use_held || crafting.latched || busy { crafting.progress = 0.; return; }
            if !crafted {
                if !available(&recipe, &parts) { focus.rejection = Some(InteractionRejection::Unavailable); return; }
                if crafting.progress == 0. { crate::audio::emit(&mut commands, "zmb_craftable_build", crate::audio::SoundSource::attached(owner)); }
                crafting.progress += time.delta_secs();
                focus.progress = crafting.progress;
                if crafting.progress < 3. { return; }
                for id in requirements(&recipe) { parts.pieces.insert(id.clone(), PartState::Consumed); crafting.consumed.insert(id); }
                crafting.crafted.insert(recipe.clone());
                crafting.benches.insert(*index, recipe.clone());
                crafting.progress = 0.;
                crafting.latched = true;
                focus.accepted += 1;
                return;
            }
            if recipe == SHIELD || recipe == DRONE {
                if recipe == DRONE && (deployment.drone.is_some() || deployment.drone_cooldown > 0.) { focus.rejection = Some(InteractionRejection::Unavailable); return; }
                if equipment.owned.contains(&recipe) { focus.rejection = Some(InteractionRejection::Duplicate); return; }
                equipment.owned.insert(recipe.clone());
                equipment.equipped = Some(recipe.clone());
                if recipe == SHIELD { equipment.shield_health = 1500; equipment.shield_held = false; }
            } else {
                let Some((_, weapon)) = STAFFS.iter().find(|(family, _)| *family == recipe) else { return; };
                let Some(d) = definition.content.weapons.iter().find(|d| d.id.0 == *weapon) else { return; };
                if inventory.owned().any(|(_, w)| w.definition == d.id) { focus.rejection = Some(InteractionRejection::Duplicate); return; }
                if inventory.exchange_staff(d).is_err() { focus.rejection = Some(InteractionRejection::Busy); return; }
            }
            crafting.latched = true;
            focus.accepted += 1;
        }
    }
}

impl Crafting {
    pub fn drone_home(&self) -> Option<Vec3> {
        self.benches.iter().find(|(_, recipe)| recipe.as_str() == DRONE)
            .and_then(|(index, _)| source::BENCHES.get(*index)).map(|(_, p)| Vec3::from_array(*p))
    }
}

pub fn reveal_part(commands: &mut Commands, parts: &mut Parts, id: &str, position: Vec3) -> Option<Entity> {
    if parts.pieces.contains_key(id) { return None; }
    let entity = commands.spawn((CraftTarget::Part(id.into()), SessionEntity, Transform::from_translation(position), Visibility::Inherited)).id();
    parts.reveal(id, entity);
    Some(entity)
}
