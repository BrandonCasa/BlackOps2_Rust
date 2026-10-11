use crate::{gameplay::{GameStep, SessionEntity}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, quests::{Element, MainQuest, QuestStage, SoulCollected, SoulCollector}, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
pub const ELEMENTS: [Element; 4] = [Element::Fire, Element::Ice, Element::Wind, Element::Lightning];
pub const WEAPONS: [&str; 4] = ["weapon.staff_fire_zm", "weapon.staff_water_zm", "weapon.staff_air_zm", "weapon.staff_lightning_zm"];
const CHARGERS: [Vec3; 4] = [Vec3::new(256.6416,-9.179052,207.057752), Vec3::new(268.6304,-9.230106,195.06946), Vec3::new(268.88186,-9.205468,206.8068), Vec3::new(256.6416,-9.230868,195.083176)];
#[derive(Resource, Default)]
pub struct StaffUpgrades {
    pub rings: [u8; 4],
    pub orb_shot: [bool; 4],
    pub unlocked: [bool; 4],
    pub inserted: [bool; 4],
    pub charged: [bool; 4],
    pub souls: [u32; 4],
    latched: bool,
    ring_delay: f32,
}
#[derive(Component)]
pub(crate) enum Target { Lever(usize), Charger(usize) }
#[derive(Component)]
struct Orb(usize);
#[derive(Component)]
struct Disc(usize);
pub struct StaffUpgradePlugin;
impl Plugin for StaffUpgradePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaffUpgrades>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, (orbs, progress).chain().in_set(GameStep::Rewards).after(crate::quests::souls).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, interact.in_set(GameStep::Interactions).after(crate::staff_puzzles::relay).run_if(in_state(GameState::Playing)));
    }
}
fn randomize(state: &mut StaffUpgrades, session: &mut crate::gameplay::Session) {
    let mut previous = 0;
    for ring in &mut state.rings { *ring = (previous + 1 + session.random() as u8 % 2) % 4; previous = *ring; }
}
fn setup(mut commands: Commands, mut state: ResMut<StaffUpgrades>, mut session: ResMut<crate::gameplay::Session>, game: Res<crate::assets::GameDefinition>, server: Res<AssetServer>, extras: Query<(Entity, &bevy::gltf::GltfExtras)>) {
    *state = default(); randomize(&mut state, &mut session);
    for (index, point) in crate::staff_puzzles::source::LEVERS { commands.spawn((SessionEntity, Target::Lever(*index), Transform::from_translation(Vec3::from_array(*point)))); }
    for index in 0..4 {
        commands.spawn((SessionEntity, Target::Charger(index), Transform::from_translation(CHARGERS[index]), SoulCollector { radius: 76.2, collected: 0, required: 20, active: false, ordinary_only: true }));
    }
    for (index, point) in [[0.8128,-14.3256,0.8128],[-0.8128,-14.3256,-0.8128],[0.8128,-14.3256,-0.8128],[-0.8128,-14.3256,0.8128]].into_iter().enumerate() {
        let mut orb = commands.spawn((SessionEntity, Orb(index), Transform::from_translation(Vec3::from_array(point)), Visibility::Hidden, Collider::sphere(0.4), Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), crate::environment::support::EnvironmentOwner::Quest, crate::environment::support::ShootableEnvironment));
        let model = ["p6_zm_tm_orb_fire_lod0", "p6_zm_tm_orb_ice_lod0", "p6_zm_tm_orb_wind_lod0", "p6_zm_tm_orb_lightning_lod0"][index];
        if let Some(scene) = game.content.presentations.iter().filter_map(|p| p.scene.as_ref()).find(|s| s.contains(model)) { orb.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
    let scene = game.content.presentations.iter().filter_map(|p| p.scene.as_ref()).find(|s| s.contains("p6_zm_tm_chamber_mid_piece_lod0"));
    for (index, point) in [[-0.0254,-11.9888,0.0127],[-0.0254,-10.5664,0.0127],[-0.0254,-9.144,0.0127],[-0.0508,-7.7216,0.0254]].into_iter().enumerate() {
        let mut disc = commands.spawn((SessionEntity, Disc(index), Transform::from_translation(Vec3::from_array(point))));
        if let Some(scene) = scene { disc.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
    for (entity, extras) in &extras { if serde_json::from_str::<serde_json::Value>(&extras.value).ok().is_some_and(|data| data["static_instance"] == true && data["source_model"].as_str().is_some_and(|model| model.contains("p6_zm_tm_chamber_mid_piece"))) { commands.entity(entity).insert(Visibility::Hidden); } }
}
fn orbs(mut commands: Commands, time: Res<Time<Fixed>>, mut state: ResMut<StaffUpgrades>, puzzles: Res<crate::staff_puzzles::StaffPuzzles>, mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    mut orbs: Query<(&Orb, &mut Visibility)>, mut discs: Query<(&Disc, &mut Transform)>, mut session: ResMut<crate::gameplay::Session>) {
    state.ring_delay = (state.ring_delay - time.delta_secs()).max(0.);
    for hit in hits.read() { if let Ok((orb, _)) = orbs.get(hit.target) && puzzles.first[orb.0] && puzzles.second[orb.0] && hit.weapon.0 == WEAPONS[orb.0] { state.orb_shot[orb.0] = true; } }
    for (index, orientation) in [2,0,3,1].into_iter().enumerate() {
        if state.orb_shot[index] && !state.unlocked[index] && state.ring_delay <= 0. && state.rings.into_iter().all(|ring| ring == orientation) {
            state.unlocked[index] = true; randomize(&mut state, &mut session);
            crate::audio::emit(&mut commands, "zmb_squest_crystal_leave", crate::audio::SoundSource::World(Vec3::new(0.,-10.,0.)));
        }
    }
    for (orb, mut visibility) in &mut orbs { *visibility = if puzzles.first[orb.0] && puzzles.second[orb.0] && !state.unlocked[orb.0] { Visibility::Inherited } else { Visibility::Hidden }; }
    for (disc, mut pose) in &mut discs { pose.rotation = Quat::from_rotation_y(state.rings[disc.0] as f32 * std::f32::consts::FRAC_PI_2); }
}
fn progress(mut commands: Commands, mut state: ResMut<StaffUpgrades>, mut events: MessageReader<SoulCollected>, targets: Query<&Target>, mut quest: ResMut<MainQuest>) {
    for event in events.read() {
        let Ok(Target::Charger(index)) = targets.get(event.collector) else { continue; };
        state.souls[*index] = (state.souls[*index] + 1).min(20);
        if event.completed {
            state.charged[*index] = true; quest.upgraded.insert(ELEMENTS[*index]);
            crate::audio::emit(&mut commands, "zmb_squest_charge_soul_full", crate::audio::SoundSource::attached(event.collector));
        }
    }
    if quest.upgraded.len() == 4 { quest.transition(QuestStage::UpgradeStaffs, QuestStage::PlaceStaffs); }
}
pub(crate) fn interact(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, mut state: ResMut<StaffUpgrades>, mut focus: ResMut<InteractionFocus>, game: Res<crate::assets::GameDefinition>, spatial: SpatialQuery, quest: Res<MainQuest>,
    mut targets: Query<(Entity, &Target, &Transform, Option<&mut SoulCollector>), Without<Player>>, mut players: Query<(Entity, &Player, &Transform, &Vitality, &mut WeaponInventory, Has<crate::perks::PerkDrinking>)>) {
    if !input.use_held { state.latched = false; }
    let Ok((owner, player, pose, health, mut inventory, drinking)) = players.single_mut() else { return; };
    if health.state() != LifeState::Standing || focus.target.is_some() { return; }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let selected = targets.iter().filter_map(|(entity, _, pose, _)| { let delta = pose.translation - eye; let distance = delta.length(); let direction = Dir3::new(delta).ok()?;
        (distance < 3.048 && forward.dot(*direction) > 0.7 && spatial.cast_ray(eye, direction, (distance - 0.1).max(0.), true, &crate::environment::solids([owner])).is_none()).then_some((entity,distance)) }).min_by(|a,b| a.1.total_cmp(&b.1)).map(|(e,_)| e);
    let Some(entity) = selected else { return; };
    let Ok((_, target, _, collector)) = targets.get_mut(entity) else { return; };
    focus.target = Some(entity); focus.progress = 0.; focus.hold_seconds = 0.;
    let key = settings.label(crate::input::Action::Interact);
    let pressed = input.use_held && !state.latched && !drinking && inventory.action == WeaponAction::Ready;
    match target {
        Target::Lever(index) => {
            focus.prompt = format!("Press {key} to rotate excavation ring ({})", state.rings[*index]);
            if !pressed || state.ring_delay > 0. { return; }
            state.rings[*index] = (state.rings[*index] + 3) % 4; state.ring_delay = 1.;
            crate::audio::emit(&mut commands, "zmb_crypt_lever", crate::audio::SoundSource::attached(entity));
        }
        Target::Charger(index) => {
            if !state.unlocked[*index] { focus.prompt = "Complete the elemental puzzles and excavation alignment".into(); return; }
            if state.inserted[*index] && !state.charged[*index] { focus.prompt = format!("Staff charging: {}/20", state.souls[*index]); return; }
            focus.prompt = if state.inserted[*index] { format!("Press {key} to retrieve upgraded staff") } else { format!("Press {key} to insert staff") };
            if !pressed { return; }
            if state.inserted[*index] {
                let id = WEAPONS[*index].replace("_zm", "_upgraded_zm");
                let Some(definition) = game.content.weapons.iter().find(|w| w.id.0 == id) else { return; };
                if inventory.exchange_staff(definition).is_err() { return; } inventory.fill_received(definition); state.inserted[*index] = false;
            } else {
                let expected = if state.charged[*index] { WEAPONS[*index].replace("_zm", "_upgraded_zm") } else { WEAPONS[*index].to_string() };
                if state.charged[*index] && quest.stage != QuestStage::Finale { return; }
                if !inventory.active().is_some_and(|w| w.definition.0 == expected) || inventory.deposit_active().is_err() { return; }
                state.inserted[*index] = true;
                if let Some(mut collector) = collector { collector.active = !state.charged[*index]; }
                if inventory.active().is_none() && let Some(fists) = game.content.weapons.iter().find(|w| w.id.0 == "weapon.zombie_fists_zm") { let _ = inventory.grant(fists); }
            }
            crate::audio::emit(&mut commands, "zmb_squest_charge_place_staff", crate::audio::SoundSource::attached(entity));
        }
    }
    state.latched = true; focus.accepted += 1;
}
