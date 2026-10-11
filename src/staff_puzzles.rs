use crate::{gameplay::{GameStep, SessionEntity}, environment::support::{EnvironmentOwner, EnvironmentalHit, ShootableEnvironment}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
mod source;
#[derive(Resource, Default)]
pub struct StaffPuzzles {
    pub first: [bool; 4],
    pub second: [bool; 4],
    pub chords: usize,
    pub rejected: u32,
    pub relays: BTreeMap<String, u8>,
    pub wind_rings: [u8; 4],
    pub smoke: [bool; 3],
    notes: BTreeSet<String>,
    ringing: f32,
    latched: bool,
}
#[derive(Component)]
struct PianoKey(String);
#[derive(Component)]
struct Relay(String);
pub struct StaffPuzzlePlugin;
impl Plugin for StaffPuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaffPuzzles>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, wind.in_set(GameStep::Rewards).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, piano.in_set(GameStep::Rewards).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, relay.in_set(GameStep::Interactions).after(crate::rituals::claim).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut puzzles: ResMut<StaffPuzzles>, mut session: ResMut<crate::gameplay::Session>, game: Res<crate::assets::GameDefinition>, server: Res<AssetServer>, extras: Query<(Entity, &bevy::gltf::GltfExtras)>) {
    *puzzles = default();
    for (entity, extras) in &extras {
        if serde_json::from_str::<serde_json::Value>(&extras.value).ok().is_some_and(|data| data["static_instance"] == true && data["source_model"].as_str().is_some_and(|model| model.contains("p6_zm_tm_wind_ceiling_ring"))) { commands.entity(entity).insert(Visibility::Hidden); }
    }
    for (index, target) in [0_u8, 3, 2, 3].into_iter().enumerate() {
        puzzles.wind_rings[index] = (target + 1 + session.random() as u8 % 3) % 4;
        let radius = 2.286 + index as f32 * 1.524;
        let shape = Collider::compound((0..32).map(|segment| {
            let angle = segment as f32 * std::f32::consts::TAU / 32.;
            (Vec3::new(angle.sin() * radius, 12.5, angle.cos() * radius), Quat::from_rotation_y(angle), Collider::cuboid(0.48, 1.05, 1.45))
        }).collect());
        let model = format!("p6_zm_tm_wind_ceiling_ring_{}_lod0", index + 1);
        let mut entity = commands.spawn((SessionEntity, WindRing(index), Transform::from_translation(Vec3::new(285.877,-14.2748,220.0402)).with_rotation(Quat::from_rotation_y(puzzles.wind_rings[index] as f32 * std::f32::consts::FRAC_PI_2)),
            shape, Sensor, CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), EnvironmentOwner::Quest, ShootableEnvironment));
        if let Some(scene) = game.content.presentations.iter().filter_map(|p| p.scene.as_ref()).find(|s| s.contains(&model)) { entity.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
    for (index, point) in [[-48.1711,4.826,-0.2032], [61.1886,4.9149,-13.4366], [-22.9997,4.6482,29.6291]].into_iter().enumerate() {
        commands.spawn((SessionEntity, Smoke(index), Transform::from_translation(Vec3::from_array(point)), Collider::sphere(0.65), Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), EnvironmentOwner::Quest, ShootableEnvironment));
    }
    for (name, value) in [("bunker",2), ("tank_platform",1), ("start",3), ("elec",1), ("ruins",3), ("air",0), ("ice",1), ("village",1)] { puzzles.relays.insert(name.into(), value); }
    for (note, point) in source::KEYS {
        commands.spawn((SessionEntity, PianoKey((*note).into()), Transform::from_translation(Vec3::from_array(*point)),
            Collider::sphere(0.24), Sensor, CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), EnvironmentOwner::Quest, ShootableEnvironment));
    }
    for (name, point) in source::RELAYS { commands.spawn((SessionEntity, Relay((*name).into()), Transform::from_translation(Vec3::from_array(*point)))); }
}
fn piano(mut commands: Commands, time: Res<Time<Fixed>>, mut puzzles: ResMut<StaffPuzzles>, mut hits: MessageReader<EnvironmentalHit>, keys: Query<&PianoKey>) {
    puzzles.ringing = (puzzles.ringing - time.delta_secs()).max(0.);
    for hit in hits.read() {
        let Ok(key) = keys.get(hit.target) else { continue; };
        if puzzles.first[3] || puzzles.ringing > 0. || hit.weapon.0 != "weapon.staff_lightning_zm" { continue; }
        if puzzles.notes.len() >= 3 { puzzles.notes.clear(); }
        puzzles.notes.insert(key.0.clone());
        crate::audio::emit(&mut commands, &format!("zmb_kbd_{}", key.0), crate::audio::SoundSource::attached(hit.target));
        if puzzles.notes.len() != 3 { continue; }
        let chords = [["a", "c", "e"], ["e", "g", "b"], ["d", "f", "a"]];
        if chords[puzzles.chords].iter().all(|n| puzzles.notes.contains(*n)) {
            puzzles.chords += 1; puzzles.notes.clear(); puzzles.ringing = 4.;
            if puzzles.chords == 3 { puzzles.first[3] = true; }
        } else { puzzles.rejected += 1; }
    }
    if puzzles.first[3] && !puzzles.second[3] {
        puzzles.second[3] = [("elec",0), ("ice",3), ("village",2), ("air",2), ("start",1), ("tank_platform",0), ("ruins",2)]
            .into_iter().all(|(name, value)| puzzles.relays.get(name) == Some(&value));
        if puzzles.second[3] { crate::audio::emit(&mut commands, "zmb_squest_puzzle_complete", crate::audio::SoundSource::World(Vec3::ZERO)); }
    }
}
fn relay(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, mut puzzles: ResMut<StaffPuzzles>, mut focus: ResMut<InteractionFocus>, spatial: SpatialQuery,
    targets: Query<(Entity, &Relay, &Transform), Without<Player>>, players: Query<(Entity, &Player, &Transform, &Vitality, &WeaponInventory, Has<crate::perks::PerkDrinking>)>) {
    if !input.use_held { puzzles.latched = false; }
    let Ok((owner, player, pose, health, inventory, drinking)) = players.single() else { return; };
    if focus.target.is_some() || health.state() != LifeState::Standing || puzzles.second[3] { return; }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let target = targets.iter().filter_map(|(entity, relay, pose)| {
        let delta = pose.translation - eye; let distance = delta.length(); let direction = Dir3::new(delta).ok()?;
        (distance < 2.54 && forward.dot(*direction) > 0.7 && spatial.cast_ray(eye, direction, (distance - 0.1).max(0.), true, &crate::environment::solids([owner])).is_none()).then_some((entity, relay, distance))
    }).min_by(|a,b| a.2.total_cmp(&b.2));
    let Some((entity, relay, _)) = target else { return; };
    focus.target = Some(entity); focus.hold_seconds = 0.; focus.progress = 0.;
    focus.prompt = format!("Press {} to rotate electrical panel ({})", settings.label(crate::input::Action::Interact), puzzles.relays[&relay.0]);
    if !input.use_held || puzzles.latched || drinking || inventory.action != WeaponAction::Ready { return; }
    puzzles.latched = true;
    let position = puzzles.relays.entry(relay.0.clone()).or_default(); *position = (*position + 1) % 4;
    crate::audio::emit(&mut commands, "zmb_squest_elec_switch", crate::audio::SoundSource::attached(entity)); focus.accepted += 1;
}

#[derive(Component)]
struct WindRing(usize);
#[derive(Component)]
struct Smoke(usize);
fn wind(mut commands: Commands, mut puzzles: ResMut<StaffPuzzles>, mut hits: MessageReader<EnvironmentalHit>,
    mut rings: Query<(&WindRing, &mut Transform), Without<Player>>, smoke: Query<(&Smoke, &Transform), (Without<WindRing>, Without<Player>)>,
    players: Query<&Transform, (With<Player>, Without<WindRing>)>) {
    for hit in hits.read() {
        if hit.weapon.0 != "weapon.staff_air_zm" { continue; }
        if !puzzles.first[2] && let Ok((ring, mut pose)) = rings.get_mut(hit.target) {
            puzzles.wind_rings[ring.0] = (puzzles.wind_rings[ring.0] + 1) % 4;
            pose.rotation = Quat::from_rotation_y(puzzles.wind_rings[ring.0] as f32 * std::f32::consts::FRAC_PI_2);
            crate::audio::emit(&mut commands, "zmb_squest_wind_ring_turn", crate::audio::SoundSource::attached(hit.target));
            puzzles.first[2] = puzzles.wind_rings == [0,3,2,3];
            if puzzles.first[2] { crate::audio::emit(&mut commands, "zmb_squest_step1_finished", crate::audio::SoundSource::attached(hit.target)); }
        }
        if puzzles.first[2] && !puzzles.second[2] && let Ok((smoke, pose)) = smoke.get(hit.target) && let Ok(attacker) = players.get(hit.attacker) {
            let destination = Vec3::new(0.1016,14.8336,0.);
            let direction = (hit.point - attacker.translation).normalize_or_zero();
            puzzles.smoke[smoke.0] = direction.dot((destination - pose.translation).normalize_or_zero()) > 25_f32.to_radians().cos();
            puzzles.second[2] = puzzles.smoke.into_iter().all(|solved| solved);
            if puzzles.second[2] { crate::audio::emit(&mut commands, "zmb_squest_step1_finished", crate::audio::SoundSource::attached(hit.target)); }
        }
    }
}
