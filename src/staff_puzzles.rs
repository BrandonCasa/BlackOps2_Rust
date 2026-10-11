use crate::{gameplay::{GameStep, SessionEntity}, environment::support::{EnvironmentOwner, EnvironmentalHit, ShootableEnvironment}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, weapons::{WeaponAction, WeaponInventory}, GameState};
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
    fire_corpses: Vec<f32>,
    ignition: f32,
    ice_lock: f32,
    notes: BTreeSet<String>,
    ringing: f32,
    latched: bool,
}
#[derive(Component)]
struct PianoKey(String);
#[derive(Component)]
pub(crate) struct Relay(String);
pub struct StaffPuzzlePlugin;
impl Plugin for StaffPuzzlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StaffPuzzles>()
            .add_systems(OnEnter(GameState::PreparingShaders), (setup, setup_ice, setup_fire).chain().in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, fire.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, ice.in_set(GameStep::Rewards).run_if(in_state(GameState::Playing)))
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
pub(crate) fn relay(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, mut puzzles: ResMut<StaffPuzzles>, mut focus: ResMut<InteractionFocus>, spatial: SpatialQuery,
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

#[derive(Component)]
struct IceTile(u8);
#[derive(Component)]
struct IceDigit { slot: usize, value: i8 }
#[derive(Component)]
struct IceGem;
#[derive(Component)]
struct IceStone(usize);
fn setup_ice(mut commands: Commands, mut puzzles: ResMut<StaffPuzzles>, mut session: ResMut<crate::gameplay::Session>, game: Res<crate::assets::GameDefinition>, server: Res<AssetServer>,
    mut brushes: Query<(Entity, &crate::environment::brushes::MapBrush, &mut Transform)>) {
    let positions: Vec<_> = game.environment.brush_entities.iter().filter(|b| b.source.get("targetname").is_some_and(|name| name == "ice_tile_original")).map(|b| Vec3::from_array(b.position)).collect();
    let mut available: Vec<_> = (0_u8..12).collect();
    let mut selected = BTreeMap::new();
    for position in positions { let index = session.random() as usize % available.len(); selected.insert(available.remove(index), position); }
    puzzles.ice_tiles = selected.keys().copied().collect(); puzzles.ice_remaining = puzzles.ice_tiles.clone();
    choose_ice_value(&mut puzzles, &mut session);
    for (entity, brush, mut pose) in &mut brushes {
        let Some(source) = game.environment.brush_entities.iter().find(|b| b.id == brush.id) else { continue; };
        if source.source.get("targetname").is_some_and(|name| name == "ice_chamber_digit") {
            if let (Some(slot), Some(value)) = (source.source.get("script_noteworthy").and_then(|s| s.parse::<usize>().ok()), source.source.get("script_string").and_then(|s| s.parse::<i8>().ok())) { commands.entity(entity).insert(IceDigit { slot, value }); }
        }
        if source.source.get("script_noteworthy").is_some_and(|name| name == "ice_ceiling_tile") {
            let Some(value) = source.source.get("script_string").and_then(|s| s.parse::<u8>().ok()) else { continue; };
            if let Some(position) = selected.get(&value) {
                pose.translation = *position;
                commands.entity(entity).insert((IceTile(value), EnvironmentOwner::Quest, ShootableEnvironment, Visibility::Inherited));
            } else { commands.entity(entity).insert((Visibility::Hidden, ColliderDisabled)); }
        }
    }
    commands.spawn((SessionEntity, IceGem, Transform::from_translation(Vec3::new(290.4998,-6.7056,187.1218)), Collider::sphere(0.55), Sensor,
        CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), EnvironmentOwner::Quest, ShootableEnvironment));
    for (index, point) in [[74.273156,4.66725,12.209018], [17.470882,4.778756,7.70636], [-29.95295,-3.182366,-123.754642]].into_iter().enumerate() {
        let mut stone = commands.spawn((SessionEntity, IceStone(index), Transform::from_translation(Vec3::from_array(point)), Collider::cuboid(0.75,1.5,0.4), Sensor,
            CollisionLayers::new(crate::environment::brushes::BRUSH_LAYER, LayerMask::NONE), EnvironmentOwner::Quest, ShootableEnvironment));
        if let Some(scene) = game.content.presentations.iter().filter_map(|p| p.scene.as_ref()).find(|scene| scene.contains("p6_zm_tm_note_rock_01_anim_lod0")) { stone.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
}
fn choose_ice_value(puzzles: &mut StaffPuzzles, session: &mut crate::gameplay::Session) {
    puzzles.ice_value = if puzzles.ice_remaining.is_empty() { None } else { puzzles.ice_remaining.iter().nth(session.random() as usize % puzzles.ice_remaining.len()).copied() };
}
fn ice(mut commands: Commands, time: Res<Time<Fixed>>, mut puzzles: ResMut<StaffPuzzles>, mut session: ResMut<crate::gameplay::Session>, game: Res<crate::assets::GameDefinition>,
    mut hits: MessageReader<EnvironmentalHit>, mut tiles: Query<(&IceTile, &mut Transform)>, gems: Query<(), With<IceGem>>, stones: Query<&IceStone>, mut digits: Query<(&IceDigit, &mut Visibility)>) {
    let ternary = [[-1,0,-1],[-1,1,-1],[-1,2,-1],[1,-1,0],[1,-1,1],[1,-1,2],[2,-1,0],[2,-1,1],[2,-1,2],[1,0,0],[1,0,1],[1,0,2]];
    for (digit, mut visibility) in &mut digits { *visibility = if puzzles.ice_value.is_some_and(|value| digit.slot < 3 && ternary[value as usize][digit.slot] == digit.value) { Visibility::Inherited } else { Visibility::Hidden }; }
    puzzles.ice_lock = (puzzles.ice_lock - time.delta_secs()).max(0.);
    for hit in hits.read() {
        let water = hit.weapon.0.contains("staff_water");
        if !puzzles.first[1] && water && puzzles.ice_lock <= 0. {
            if gems.contains(hit.target) { choose_ice_value(&mut puzzles, &mut session); }
            if let Ok((tile, _)) = tiles.get(hit.target) && puzzles.ice_remaining.contains(&tile.0) {
                if puzzles.ice_value == Some(tile.0) {
                    puzzles.ice_remaining.remove(&tile.0); puzzles.ice_lock = 0.2;
                    if let Ok((_, mut pose)) = tiles.get_mut(hit.target) { pose.rotate_local_z(std::f32::consts::PI); }
                    crate::audio::emit(&mut commands, "zmb_squest_ice_tile_flip", crate::audio::SoundSource::attached(hit.target));
                } else {
                    let solved: BTreeSet<_> = puzzles.ice_tiles.difference(&puzzles.ice_remaining).copied().collect();
                    for (tile, mut pose) in &mut tiles { if solved.contains(&tile.0) { pose.rotate_local_z(std::f32::consts::PI); } }
                    puzzles.ice_remaining = puzzles.ice_tiles.clone(); puzzles.ice_lock = 2.; puzzles.rejected += 1;
                }
                puzzles.first[1] = puzzles.ice_remaining.is_empty(); choose_ice_value(&mut puzzles, &mut session);
            }
        }
        if puzzles.first[1] && !puzzles.second[1] && let Ok(stone) = stones.get(hit.target) {
            if !puzzles.frozen_stones[stone.0] && water {
                puzzles.frozen_stones[stone.0] = true;
                crate::audio::emit(&mut commands, "zmb_squest_ice_stone_freeze", crate::audio::SoundSource::attached(hit.target));
            } else if puzzles.frozen_stones[stone.0] && !hit.weapon.0.contains("staff") && game.content.weapons.iter().find(|w| w.id == hit.weapon).is_some_and(|w| matches!(w.category, crate::origins::DamageCategory::Bullet)) {
                puzzles.broken_stones[stone.0] = true; commands.entity(hit.target).insert((Visibility::Hidden, ColliderDisabled)).remove::<ShootableEnvironment>();
                crate::audio::emit(&mut commands, "zmb_squest_ice_stone_shatter", crate::audio::SoundSource::World(hit.point));
                puzzles.second[1] = puzzles.broken_stones.into_iter().all(|b| b);
            }
        }
    }
}

#[derive(Component)]
struct FireSymbol(u8);
fn setup_fire(mut commands: Commands, mut puzzles: ResMut<StaffPuzzles>, mut session: ResMut<crate::gameplay::Session>, game: Res<crate::assets::GameDefinition>, brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>) {
    let mut groups: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for brush in &game.environment.brush_entities {
        if brush.source.get("script_noteworthy").is_some_and(|name| name == "fire_torch_ternary") {
            if let (Some(group), Some(value)) = (brush.source.get("targetname"), brush.source.get("script_int").and_then(|s| s.parse::<u8>().ok())) { groups.entry(group.clone()).or_default().push(value); }
        }
    }
    for values in groups.values() { puzzles.fire_torches.insert(values[session.random() as usize % values.len()]); }
    for (entity, brush) in &brushes {
        if let Some(source) = game.environment.brush_entities.iter().find(|b| b.id == brush.id) && source.source.get("script_noteworthy").is_some_and(|name| name == "fire_torch_ternary") {
            if let Some(value) = source.source.get("script_int").and_then(|s| s.parse::<u8>().ok()) { commands.entity(entity).insert((FireSymbol(value), Visibility::Hidden)); }
        }
    }
}
fn fire(mut commands: Commands, time: Res<Time<Fixed>>, mut puzzles: ResMut<StaffPuzzles>, mut impacts: MessageReader<crate::projectiles::ProjectileImpact>, mut deaths: MessageReader<crate::damage::EnemyDeath>,
    crafting: Res<crate::crafting::Crafting>, game: Res<crate::assets::GameDefinition>, spatial: SpatialQuery, volumes: Query<&crate::environment::triggers::MapTrigger>,
    enemies: Query<&Transform, With<crate::enemies::Zombie>>, mut symbols: Query<(&FireSymbol, &mut Visibility)>) {
    let dt = time.delta_secs(); puzzles.ignition = (puzzles.ignition - dt).max(0.);
    for duration in puzzles.lit_torches.values_mut() { *duration -= dt; }
    puzzles.lit_torches.retain(|_, duration| *duration > 0.);
    let inside = |point| spatial.shape_intersections(&Collider::sphere(0.1), point, Quat::IDENTITY, &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER)).iter().filter_map(|e| volumes.get(*e).ok()).any(|v| game.environment.triggers.iter().find(|t| t.id == v.0).is_some_and(|t| t.source.get("targetname").is_some_and(|name| name == "fire_sacrifice_volume")));
    for impact in impacts.read() {
        if !impact.weapon.0.contains("staff_fire") { continue; }
        if !puzzles.first[0] && inside(impact.point) { puzzles.ignition = 6.; }
        if puzzles.first[0] && !puzzles.second[0] {
            for (number, point) in source::TORCHES {
                if impact.point.distance_squared(Vec3::from_array(*point)) <= 1.6256_f32.powi(2) {
                    puzzles.lit_torches.insert(*number, 15.);
                    if !puzzles.fire_torches.contains(number) { puzzles.rejected += 1; }
                    crate::audio::emit(&mut commands, "zmb_squest_fire_torch_ignite", crate::audio::SoundSource::World(Vec3::from_array(*point)));
                }
            }
            puzzles.second[0] = puzzles.lit_torches.keys().copied().collect::<BTreeSet<_>>() == puzzles.fire_torches;
        }
    }
    for death in deaths.read() {
        if puzzles.first[0] || !crafting.crafted.contains("elemental_staff_fire") || !death.0.can_supply_soul() { continue; }
        if let Ok(pose) = enemies.get(death.0.impact.target) && inside(pose.translation) {
            if puzzles.fire_corpses.len() >= 15 { puzzles.fire_corpses.remove(0); }
            puzzles.fire_corpses.push(2.);
        }
    }
    for timer in &mut puzzles.fire_corpses { *timer -= dt; }
    if puzzles.ignition > 0. {
        let ready = puzzles.fire_corpses.iter().filter(|t| **t <= 0.).count();
        puzzles.fire_corpses.retain(|t| *t > 0.); puzzles.sacrifices += ready as u32;
        puzzles.first[0] = puzzles.sacrifices >= 32;
    }
    for (symbol, mut visibility) in &mut symbols { *visibility = if puzzles.first[0] && puzzles.fire_torches.contains(&symbol.0) { Visibility::Inherited } else { Visibility::Hidden }; }
}
