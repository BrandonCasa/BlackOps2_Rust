use super::{
    parts::Parts,
    weather::{Weather, WeatherKind},
};
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    life::{LifeState, Vitality},
    origins::{ContentId, WeaponId},
    player::Player,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldAssetRoot};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigLocation {
    pub id: String,
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub zone: String,
    pub group: String,
    pub initial: bool,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DigDefinition {
    pub shovels: Vec<DigLocation>,
    pub mounds: Vec<DigLocation>,
    pub hidden: Vec<DigLocation>,
    pub shovel: ContentId,
    pub mound: ContentId,
    pub blood_mound: ContentId,
    pub perk_bottle: ContentId,
}
impl DigDefinition {
    pub fn valid(&self, game: &GameDefinition) -> bool {
        let mut ids = BTreeSet::new();
        (1..=32).contains(&self.shovels.len())
            && (1..=256).contains(&self.mounds.len())
            && (4..=128).contains(&self.hidden.len())
            && self
                .shovels
                .iter()
                .chain(&self.mounds)
                .chain(&self.hidden)
                .all(|l| {
                    !l.id.is_empty()
                        && l.id.len() < 128
                        && l.id
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
                        && ids.insert(&l.id)
                        && l.position.iter().all(|n| n.is_finite())
                        && l.rotation.iter().all(|n| n.is_finite())
                        && (Quat::from_array(l.rotation).length() - 1.).abs() < 0.001
                        && game.content.zones.iter().any(|z| z.id.0 == l.zone)
                        && !l.group.is_empty()
                        && l.group.len() < 128
                })
            && [
                &self.shovel,
                &self.mound,
                &self.blood_mound,
                &self.perk_bottle,
            ]
            .iter()
            .all(|id| {
                game.content
                    .presentations
                    .iter()
                    .any(|p| p.id == **id && p.scene.is_some())
            })
            && [
                "ballista_zm",
                "c96_zm",
                "870mcs_zm",
                "dsr50_zm",
                "srm1216_zm",
                "claymore_zm",
                "ak74u_zm",
                "ksg_zm",
                "mp40_zm",
                "mp44_zm",
                "frag_grenade_zm",
            ]
            .iter()
            .all(|id| {
                game.content
                    .weapons
                    .iter()
                    .any(|w| w.id.0 == format!("weapon.{id}") && w.pickup.is_some())
            })
            && [
                "nuke",
                "double_points",
                "fire_sale",
                "insta_kill",
                "full_ammo",
                "zombie_blood",
                "bonus_points_player",
            ]
            .iter()
            .all(|id| {
                game.content
                    .powerups
                    .iter()
                    .any(|p| p.id.0 == format!("powerup.{id}"))
            })
    }
}

#[derive(Component, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Digger {
    pub shovel: bool,
    pub golden: bool,
    pub helmet: bool,
    pub qualifying_digs: u32,
    pub losing_streak: u32,
    pub hidden_found: u32,
    pub hidden_wait_round: bool,
    pub hidden_used: BTreeSet<usize>,
}

#[derive(Resource, Default, Serialize)]
pub struct Digging {
    pub active: BTreeSet<usize>,
    pub misses: BTreeMap<String, u32>,
    pub powerups_this_round: u32,
    pub bloods_this_round: u32,
    pub rare_spawned: BTreeSet<String>,
    pub last_prize_rare: bool,
    pub box_moved: bool,
    pub events: Vec<serde_json::Value>,
    pub expired_weapons: u32,
    pub replenish_round: Option<u32>,
    replenish_delay: f32,
    #[serde(skip)]
    spawn_queue: VecDeque<usize>,
    spawn_delay: f32,
}
impl Digging {
    fn record(&mut self, value: serde_json::Value) {
        if self.events.len() >= 256 {
            self.events.remove(0);
        }
        self.events.push(value);
    }
}

#[derive(Component, Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DigTarget {
    Shovel(usize),
    Mound(usize),
    Hidden(usize),
    Weapon(WeaponId),
    Part(ContentId),
}
#[derive(Component)]
pub struct RisingPickup {
    pub remaining: f32,
    pub position: Vec3,
    pub depth: f32,
    pub duration: f32,
}
#[derive(Component)]
pub struct WeaponTimeout(pub f32);
#[derive(Component)]
struct WeaponOrigin(Vec3);
#[derive(Component)]
pub struct PerkSlotBottle {
    pub owner: Entity,
}
#[derive(Component)]
struct SecondGrenade {
    owner: Entity,
    position: Vec3,
    remaining: f32,
}
#[derive(Component)]
struct DigVoice {
    remaining: f32,
}

pub struct DiggingPlugin;
impl Plugin for DiggingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Digging>()
            .register_required_components::<Player, Digger>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                spawn.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (replenish, animate, sink_weapons)
                    .chain()
                    .after(super::weather::tick)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn scene(game: &GameDefinition, id: &ContentId) -> Option<String> {
    game.content
        .presentations
        .iter()
        .find(|p| p.id == *id)
        .and_then(|p| p.scene.clone())
}
fn visual(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    id: &ContentId,
    position: Vec3,
    rotation: Quat,
) -> Entity {
    let mut entity = commands.spawn((
        SessionEntity,
        Transform::from_translation(position).with_rotation(rotation),
        Visibility::Inherited,
    ));
    if let Some(scene) = scene(game, id) {
        entity.insert(WorldAssetRoot(server.load(scene)));
    }
    entity.id()
}
pub(crate) fn mound(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    index: usize,
    hidden: bool,
) -> Entity {
    let d = game.environment.digging.as_ref().unwrap();
    let spot = if hidden {
        &d.hidden[index]
    } else {
        &d.mounds[index]
    };
    let position = Vec3::from_array(spot.position);
    let entity = visual(
        commands,
        server,
        game,
        if hidden { &d.blood_mound } else { &d.mound },
        position - Vec3::Y * 1.016,
        Quat::from_array(spot.rotation),
    );
    commands.entity(entity).insert((
        if hidden {
            DigTarget::Hidden(index)
        } else {
            DigTarget::Mound(index)
        },
        RisingPickup {
            remaining: 3.,
            position,
            depth: 1.016,
            duration: 3.,
        },
    ));
    entity
}
pub fn spawn_part(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    parts: &mut Parts,
    id: &ContentId,
    position: Vec3,
) -> Option<Entity> {
    if parts.pieces.contains_key(&id.0) {
        return None;
    }
    let definition = game.environment.parts.iter().find(|p| p.id == *id)?;
    let entity = visual(
        commands,
        server,
        game,
        &definition.presentation,
        position,
        Quat::IDENTITY,
    );
    if !parts.reveal(&id.0, entity) {
        commands.entity(entity).despawn();
        return None;
    }
    commands.entity(entity).insert((
        DigTarget::Part(id.clone()),
        Visibility::Hidden,
        RisingPickup {
            remaining: 0.55,
            position: position + Vec3::Y * 0.8128,
            depth: 0.8128,
            duration: 0.05,
        },
    ));
    Some(entity)
}

fn spawn(
    mut commands: Commands,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut state: ResMut<Digging>,
    mut session: ResMut<Session>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    let Some(d) = &game.environment.digging else {
        return;
    };
    let mut groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, spot) in d.shovels.iter().enumerate() {
        groups.entry(&spot.group).or_default().push(i);
    }
    for indices in groups.values() {
        let i = indices[session.random() as usize % indices.len()];
        let spot = &d.shovels[i];
        let entity = visual(
            &mut commands,
            &server,
            &game,
            &d.shovel,
            Vec3::from_array(spot.position),
            Quat::from_array(spot.rotation),
        );
        commands.entity(entity).insert((
            DigTarget::Shovel(i),
            crate::rendering::opaque::NativeShaderStart(0.),
            crate::rendering::NativeArtInputs::default(),
        ));
    }
    for (i, spot) in d.mounds.iter().enumerate() {
        if spot.initial {
            state.active.insert(i);
            mound(&mut commands, &server, &game, i, false);
        }
    }
    for part in &game.environment.parts {
        if part.dig_zone.is_some() {
            state.misses.insert(part.id.0.clone(), part.initial_misses);
        }
    }
}

fn replenish(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    id: Res<crate::session::SessionId>,
    mut ending: MessageReader<crate::rounds::RoundEnding>,
    game: Res<GameDefinition>,
    weather: Res<Weather>,
    server: Res<AssetServer>,
    mut state: ResMut<Digging>,
    parts: Res<Parts>,
    mut session: ResMut<Session>,
    mut players: Query<&mut Digger>,
) {
    let Some(d) = &game.environment.digging else {
        return;
    };
    for event in ending.read() {
        if event.session != *id {
            continue;
        }
        state.replenish_round = Some(event.round);
        state.replenish_delay = 2.;
        state.powerups_this_round = 0;
        state.bloods_this_round = 0;
        state.rare_spawned.clear();
        for mut digger in &mut players {
            if digger.hidden_wait_round && digger.hidden_found < 4 {
                digger.hidden_wait_round = false;
                new_hidden(&mut commands, &server, &game, &mut session, &mut digger);
            }
        }
    }
    if state.replenish_round.is_some() {
        state.replenish_delay -= time.delta_secs();
        if state.replenish_delay <= 0. {
            let round = state.replenish_round.take().unwrap();
            let mut indices: Vec<_> = (0..d.mounds.len()).collect();
            shuffle(&mut indices, &mut session);
            let max = match weather.kind {
                WeatherKind::Snow => 0,
                WeatherKind::Rain => 5,
                WeatherKind::Clear => 3,
            };
            if weather.kind != WeatherKind::Snow {
                let _solo_replenishment_roll = session.random();
            }
            let mut selected = Vec::new();
            for i in indices {
                if !state.active.contains(&i)
                    && selected.len() < max
                    && state.active.len() + selected.len() <= 15
                {
                    selected.push(i);
                }
            }
            if weather.kind == WeatherKind::Snow {
                for part in &game.environment.parts {
                    let Some(zone) = &part.dig_zone else {
                        continue;
                    };
                    if parts.pieces.contains_key(&part.id.0) {
                        continue;
                    }
                    let active = state
                        .active
                        .iter()
                        .chain(selected.iter())
                        .filter(|&&i| d.mounds[i].zone.contains(zone))
                        .count();
                    let candidates: Vec<_> = (0..d.mounds.len())
                        .filter(|i| {
                            d.mounds[*i].zone.contains(zone)
                                && !state.active.contains(i)
                                && !selected.contains(i)
                        })
                        .collect();
                    if active < 2
                        && !candidates.is_empty()
                        && state.active.len() + selected.len() <= 15
                    {
                        selected.push(candidates[session.random() as usize % candidates.len()]);
                    }
                }
            }
            state.record(serde_json::json!({"event":"replenishment","round":round,"weather":weather.kind,"selected":selected}));
            state.spawn_queue.extend(selected);
        }
    }
    state.spawn_delay -= time.delta_secs();
    if state.spawn_delay <= 0.
        && let Some(i) = state.spawn_queue.pop_front()
    {
        state.active.insert(i);
        mound(&mut commands, &server, &game, i, false);
        state.spawn_delay = 0.05;
    }
}
fn shuffle(values: &mut [usize], session: &mut Session) {
    for i in (1..values.len()).rev() {
        values.swap(i, session.random() as usize % (i + 1));
    }
}

fn new_hidden(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    session: &mut Session,
    digger: &mut Digger,
) {
    let Some(d) = &game.environment.digging else {
        return;
    };
    if digger.hidden_found >= 4 {
        return;
    }
    let mut candidates: Vec<_> = (0..d.hidden.len()).collect();
    shuffle(&mut candidates, session);
    if let Some(&i) = candidates.iter().find(|i| !digger.hidden_used.contains(i)) {
        digger.hidden_used.insert(i);
        mound(commands, server, game, i, true);
    }
}

fn animate(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    effects: Res<crate::powerups::PowerupEffects>,
    mut state: ResMut<Digging>,
    mut session: ResMut<Session>,
    mut rising: Query<(Entity, &mut RisingPickup, &mut Transform)>,
    mut weapons: Query<(Entity, &mut WeaponTimeout)>,
    hidden: Query<(Entity, &DigTarget)>,
    mut bottles: Query<(Entity, &PerkSlotBottle, &mut Transform), Without<RisingPickup>>,
    mut players: Query<
        (
            &Player,
            &Transform,
            &Vitality,
            &mut crate::solo_revive::PerkInventory,
        ),
        (Without<PerkSlotBottle>, Without<RisingPickup>),
    >,
    mut grenades: Query<(Entity, &mut SecondGrenade)>,
    mut voices: Query<(Entity, &mut DigVoice)>,
) {
    let dt = time.delta_secs();
    for (e, mut rise, mut pose) in &mut rising {
        rise.remaining = (rise.remaining - dt).max(0.);
        let t = (1. - rise.remaining / rise.duration).clamp(0., 1.);
        pose.translation = rise.position - Vec3::Y * rise.depth * (1. - t);
        if rise.remaining == 0. {
            commands.entity(e).remove::<RisingPickup>();
            if hidden
                .get(e)
                .is_ok_and(|(_, t)| matches!(t, DigTarget::Part(_)))
            {
                commands.entity(e).insert(Visibility::Inherited);
                sound(&mut commands, &game, "evt_staff_digup", e);
                if game.audio.aliases.contains_key("evt_staff_digup_lp") {
                    crate::audio::emit_loop(
                        &mut commands,
                        "evt_staff_digup_lp",
                        crate::audio::SoundSource::attached(e),
                    );
                }
            }
        }
    }
    for (e, mut timer) in &mut weapons {
        timer.0 -= dt;
        if timer.0 <= 0. {
            commands.entity(e).despawn();
            state.expired_weapons += 1;
        }
    }
    for (e, target) in &hidden {
        if matches!(target, DigTarget::Hidden(_)) {
            commands.entity(e).insert(if effects.zombie_blood > 0. {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
    for (e, bottle, mut pose) in &mut bottles {
        pose.rotate_y(dt * std::f32::consts::TAU / 5.);
        if let Ok((player, body, health, mut perks)) = players.get_mut(bottle.owner)
            && health.state() == LifeState::Standing
            && (body.translation - Vec3::Y * player.height * 0.5).distance(pose.translation)
                < 1.6256
        {
            perks.earned_slots = (perks.earned_slots + 1).min(4);
            commands.entity(e).despawn();
            state.record(
                serde_json::json!({"event":"perk_slot_collected","limit":perks.purchase_limit()}),
            );
        }
    }
    for (e, mut grenade) in &mut grenades {
        grenade.remaining -= dt;
        if grenade.remaining <= 0. {
            if session.random() % 2 == 0 {
                crate::equipment::dig_grenade(
                    &mut commands,
                    &server,
                    &game,
                    grenade.owner,
                    grenade.position,
                    Vec3::new(1.27, 7.62, -1.27),
                );
            }
            commands.entity(e).despawn();
        }
    }
    for (e, mut voice) in &mut voices {
        voice.remaining -= dt;
        if voice.remaining <= 0. {
            commands.entity(e).remove::<DigVoice>();
        }
    }
}

fn interact(
    mut commands: Commands,
    game: Res<GameDefinition>,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    server: Res<AssetServer>,
    spatial: SpatialQuery,
    mut state: ResMut<Digging>,
    weather: Res<Weather>,
    effects: Res<crate::powerups::PowerupEffects>,
    drops: Res<crate::powerups::Drops>,
    mut parts: ResMut<Parts>,
    mut session: ResMut<Session>,
    targets: Query<
        (
            Entity,
            &DigTarget,
            &Transform,
            Option<&RisingPickup>,
            Option<&WeaponTimeout>,
        ),
        Without<Player>,
    >,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut Digger,
        &mut WeaponInventory,
        &mut crate::mines::MineInventory,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
        Has<crate::perks::PerkDrinking>,
        Has<DigVoice>,
    )>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    mut latch: Local<(Option<Entity>, bool)>,
) {
    let Some(d) = &game.environment.digging else {
        return;
    };
    let Ok((
        owner,
        player,
        body,
        health,
        mut digger,
        mut inventory,
        mut mines,
        grenades,
        tactical,
        drinking,
        voice_cooldown,
    )) = players.single_mut()
    else {
        return;
    };
    if latch.0 != Some(owner) {
        *latch = (Some(owner), false);
    }
    if !input.use_held {
        latch.1 = false;
    }
    if health.state() != LifeState::Standing || focus.target.is_some() {
        return;
    }
    let eye = body.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x(player.pitch + inventory.recoil.y)
        * Vec3::NEG_Z;
    let candidate = targets
        .iter()
        .filter_map(|(e, target, pose, rising, timer)| {
            if rising.is_some() || timer.is_some_and(|t| t.0 <= 0.) {
                return None;
            }
            let point = match target {
                DigTarget::Shovel(_) if digger.shovel => return None,
                DigTarget::Shovel(_) => pose.translation + Vec3::Y * 0.8128,
                DigTarget::Mound(i) if !state.active.contains(i) => return None,
                DigTarget::Mound(_) => pose.translation + Vec3::Y * 0.508,
                DigTarget::Hidden(_)
                    if !digger.golden
                        || effects.zombie_blood <= 0.
                        || digger.hidden_wait_round
                        || digger.hidden_found >= 4 =>
                {
                    return None;
                }
                DigTarget::Hidden(_) => pose.translation + Vec3::Y * 0.508,
                _ => pose.translation,
            };
            let delta = point - eye;
            let distance = delta.length();
            let reach = if matches!(target, DigTarget::Shovel(_)) {
                1.6256
            } else {
                2.54
            };
            if distance > reach || distance < 0.01 {
                return None;
            }
            let direction = Dir3::new(delta).ok()?;
            let dot = direction.dot(forward);
            if (!matches!(target, DigTarget::Hidden(_)) && dot < 0.85)
                || spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.12).max(0.),
                        true,
                        &super::solids([owner]),
                    )
                    .is_some()
            {
                return None;
            }
            Some((dot, distance, e, target, pose.translation))
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));
    let Some((_, _, entity, target, position)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    let key = settings.label(crate::input::Action::Interact);
    focus.prompt = match target {
        DigTarget::Shovel(_) => format!("Press {key} to take shovel"),
        DigTarget::Mound(_) if !digger.shovel => "You need a shovel".into(),
        DigTarget::Mound(_) | DigTarget::Hidden(_) => format!("Press {key} to dig"),
        DigTarget::Weapon(id) => format!(
            "Press {key} to take {}",
            game.content
                .weapons
                .iter()
                .find(|w| w.id == *id)
                .map_or(id.0.as_str(), |w| w.name.as_str())
        ),
        DigTarget::Part(_) => format!("Press {key} to take staff part"),
    };
    if !input.use_held || latch.1 {
        return;
    }
    latch.1 = true;
    if inventory.action != WeaponAction::Ready
        || mines.busy()
        || grenades.busy()
        || tactical.busy()
        || drinking
    {
        focus.rejection = Some(crate::interactions::InteractionRejection::Busy);
        return;
    }
    match target {
        DigTarget::Shovel(i) => {
            digger.shovel = true;
            commands.entity(entity).despawn();
            sound(&mut commands, &game, "zmb_craftable_pickup", owner);
            state.record(serde_json::json!({"event":"shovel","site":i}));
            dialogue(
                &mut commands,
                &game,
                &session,
                owner,
                "pickup_shovel",
                voice_cooldown,
            );
        }
        DigTarget::Mound(i) => {
            if !digger.shovel {
                return;
            }
            state.active.remove(i);
            commands.entity(entity).despawn();
            sound(&mut commands, &game, "evt_dig", owner);
            let point = position + Vec3::Y * 0.508;
            let mut part = None;
            for p in &game.environment.parts {
                if parts.pieces.contains_key(&p.id.0)
                    || !p
                        .dig_zone
                        .as_ref()
                        .is_some_and(|z| d.mounds[*i].zone.contains(z))
                {
                    continue;
                }
                let misses = *state.misses.get(&p.id.0).unwrap_or(&p.initial_misses);
                let chance = if weather.kind == WeatherKind::Snow {
                    100 / (misses + 1)
                } else {
                    101
                };
                if session.random() % 100 > chance || (misses > 3 && chance < 100) {
                    part = Some(p.id.clone());
                } else {
                    state.misses.insert(p.id.0.clone(), misses + 1);
                }
                break;
            }
            let outcome;
            if let Some(id) = part {
                spawn_part(&mut commands, &server, &game, &mut parts, &id, point);
                outcome = id.0;
                dialogue(
                    &mut commands,
                    &game,
                    &session,
                    owner,
                    "dig_staff_part",
                    voice_cooldown,
                );
            } else {
                let mut good = 50;
                if digger.qualifying_digs == 0 || digger.losing_streak == 3 {
                    digger.losing_streak = 0;
                    good = 100;
                }
                if digger.golden {
                    if !digger.helmet && session.random() % 100 >= 95 {
                        digger.helmet = true;
                        sound(&mut commands, &game, "zmb_squest_golden_anything", owner);
                        state.record(serde_json::json!({"event":"dig","site":i,"outcome":"golden_helmet","count":digger.qualifying_digs}));
                        focus.accepted += 1;
                        return;
                    }
                    good = 70;
                }
                if session.random() % 100 > good {
                    digger.losing_streak += 1;
                    if session.random() % 2 == 0 {
                        crate::equipment::dig_grenade(
                            &mut commands,
                            &server,
                            &game,
                            owner,
                            point,
                            Vec3::Y * 7.62,
                        );
                        let extra = session.random() % 4;
                        if extra != 0 {
                            commands.spawn((
                                SessionEntity,
                                SecondGrenade {
                                    owner,
                                    position: point,
                                    remaining: 0.3,
                                },
                            ));
                        }
                        sound(&mut commands, &game, "evt_grenade_digup", owner);
                        outcome = "grenade".into();
                        dialogue(
                            &mut commands,
                            &game,
                            &session,
                            owner,
                            "dig_grenade",
                            voice_cooldown,
                        );
                    } else {
                        dug_zombie(&mut commands, &game, &mut session, position);
                        outcome = "zombie".into();
                        dialogue(
                            &mut commands,
                            &game,
                            &session,
                            owner,
                            "dig_zombie",
                            voice_cooldown,
                        );
                    }
                } else if session.random() % 2 == 0 {
                    outcome = powerup_loot(&mut state, &mut session, &digger, drops.count);
                    spawn_powerup(&mut commands, &server, &game, &outcome, point);
                    dialogue(
                        &mut commands,
                        &game,
                        &session,
                        owner,
                        if outcome == "bonus_points_player" {
                            "dig_cash"
                        } else {
                            "dig_powerup"
                        },
                        voice_cooldown,
                    );
                } else {
                    let common = ["ballista_zm", "c96_zm", "870mcs_zm"];
                    let rare = [
                        "dsr50_zm",
                        "srm1216_zm",
                        "claymore_zm",
                        "ak74u_zm",
                        "ksg_zm",
                        "mp40_zm",
                        "mp44_zm",
                    ];
                    let choices: &[&str] = if session.random() % 100 < 90 {
                        &common
                    } else {
                        &rare[..if digger.golden { 7 } else { 2 }]
                    };
                    let id = WeaponId(format!(
                        "weapon.{}",
                        choices[session.random() as usize % choices.len()]
                    ));
                    let weapon = game.content.weapons.iter().find(|w| w.id == id).unwrap();
                    let entity = visual(
                        &mut commands,
                        &server,
                        &game,
                        weapon.pickup.as_ref().unwrap(),
                        point + Vec3::Y * 1.016,
                        Quat::from_rotation_y(player.yaw + std::f32::consts::FRAC_PI_2),
                    );
                    commands.entity(entity).insert((
                        DigTarget::Weapon(id.clone()),
                        WeaponTimeout(12.),
                        WeaponOrigin(point + Vec3::Y * 1.016),
                    ));
                    sound(&mut commands, &game, "evt_weapon_digup", entity);
                    outcome = id.0;
                    dialogue(
                        &mut commands,
                        &game,
                        &session,
                        owner,
                        "dig_gun",
                        voice_cooldown,
                    );
                }
            }
            if !digger.golden {
                digger.qualifying_digs += 1;
                if digger.qualifying_digs >= 30 {
                    digger.golden = true;
                    new_hidden(&mut commands, &server, &game, &mut session, &mut digger);
                    sound(&mut commands, &game, "zmb_squest_golden_anything", owner);
                }
            }
            state.record(serde_json::json!({"event":"dig","site":i,"outcome":outcome,"count":digger.qualifying_digs,"losing_streak":digger.losing_streak,"golden":digger.golden}));
        }
        DigTarget::Hidden(i) => {
            if !digger.shovel {
                return;
            }
            commands.entity(entity).despawn();
            sound(&mut commands, &game, "evt_dig", owner);
            let bottle = visual(
                &mut commands,
                &server,
                &game,
                &d.perk_bottle,
                position + Vec3::Y * 1.016,
                Quat::IDENTITY,
            );
            commands.entity(bottle).insert(PerkSlotBottle { owner });
            digger.hidden_found += 1;
            digger.hidden_wait_round = true;
            state.record(
                serde_json::json!({"event":"hidden_dig","site":i,"found":digger.hidden_found}),
            );
        }
        DigTarget::Weapon(id) => {
            let Some(weapon) = game.content.weapons.iter().find(|w| w.id == *id) else {
                return;
            };
            let accepted = if let Some(profile) = &weapon.mine {
                if mines.definition.as_ref() == Some(id) {
                    mines.count = profile.stock;
                    true
                } else {
                    mines.receive(weapon).is_ok()
                }
            } else if inventory.owned().any(|(_, w)| w.definition == *id) {
                inventory.refill_weapon(weapon);
                true
            } else {
                inventory.receive(weapon).is_ok()
            };
            if !accepted {
                return;
            }
            commands.entity(entity).despawn();
            crate::audio::binding(
                &mut commands,
                &game,
                &format!("{}.purchase", id.0),
                crate::audio::SoundSource::attached(owner),
            );
            state.record(serde_json::json!({"event":"weapon_collected","id":id.0}));
        }
        DigTarget::Part(id) => {
            if !parts.take(&id.0, entity, owner) {
                return;
            }
            commands.entity(entity).despawn();
            sound(&mut commands, &game, "zmb_craftable_pickup", owner);
            state.record(serde_json::json!({"event":"part_collected","id":id.0}));
        }
    }
    focus.accepted += 1;
}

fn sink_weapons(mut pickups: Query<(&WeaponTimeout, &WeaponOrigin, &mut Transform)>) {
    for (timer, origin, mut pose) in &mut pickups {
        pose.translation = origin.0 - Vec3::Y * 1.016 * (1. - timer.0 / 12.).clamp(0., 1.);
    }
}

fn powerup_loot(
    state: &mut Digging,
    session: &mut Session,
    digger: &Digger,
    drop_count: u32,
) -> String {
    let mut rare = vec!["nuke", "double_points"];
    if state.box_moved && !state.rare_spawned.contains("fire_sale") {
        rare.push("fire_sale");
    }
    if digger.golden {
        rare.extend(["insta_kill", "full_ammo"]);
    }
    rare.retain(|id| !state.rare_spawned.contains(*id));
    if state.powerups_this_round + drop_count > 4
        || state.last_prize_rare
        || rare.is_empty()
        || session.random() % 100 < 80
    {
        state.last_prize_rare = false;
        if state.bloods_this_round < 1 && session.random() % 100 > 70 {
            state.bloods_this_round += 1;
            state.powerups_this_round += 1;
            "zombie_blood".into()
        } else {
            "bonus_points_player".into()
        }
    } else {
        let id = rare[session.random() as usize % rare.len()];
        state.last_prize_rare = true;
        state.powerups_this_round += 1;
        state.rare_spawned.insert(id.into());
        id.into()
    }
}

pub(crate) fn spawn_powerup(
    commands: &mut Commands,
    server: &AssetServer,
    game: &GameDefinition,
    id: &str,
    position: Vec3,
) -> Option<Entity> {
    let Some((index, definition)) = game
        .content
        .powerups
        .iter()
        .enumerate()
        .find(|(_, p)| p.id.0 == format!("powerup.{id}"))
    else {
        return None;
    };
    let Some(presentation) = &definition.presentation else {
        return None;
    };
    let entity = visual(
        commands,
        server,
        game,
        presentation,
        position,
        Quat::IDENTITY,
    );
    commands.entity(entity).insert((
        crate::powerups::Pickup {
            definition: index,
            remaining: 27.1,
        },
        RisingPickup {
            remaining: 0.6,
            position: position + Vec3::Y * 1.016,
            depth: 1.016,
            duration: 0.6,
        },
    ));
    Some(entity)
}

fn dug_zombie(
    commands: &mut Commands,
    game: &GameDefinition,
    session: &mut Session,
    position: Vec3,
) {
    let Some(definition) = game.content.enemies.iter().find(|e| e.family == "german") else {
        return;
    };
    let gait = game.rounds.gait(session.round, session.random());
    let position = position + Vec3::Y * 0.85;
    let zombie = crate::enemies::Zombie::new(position, definition.id.clone(), gait);
    let zombie = if let Some(profile) = definition
        .traversals
        .get("rise")
        .and_then(|profiles| profiles.first())
    {
        zombie.dug_rising(profile, Quat::IDENTITY)
    } else {
        zombie
    };
    let health = game.rounds.health(session.round);
    let entity = commands
        .spawn((
            SessionEntity,
            zombie,
            Vitality::ordinary_actor(health, health).unwrap(),
            crate::rounds::PopulationKind::Special,
            crate::player::capsule(0.3, 1.7),
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id();
    sound(commands, game, "evt_zombie_dig_dirt", entity);
}

fn sound(commands: &mut Commands, game: &GameDefinition, alias: &str, owner: Entity) {
    if game.audio.aliases.contains_key(alias) {
        crate::audio::emit(commands, alias, crate::audio::SoundSource::attached(owner));
    }
}
fn dialogue(
    commands: &mut Commands,
    game: &GameDefinition,
    session: &Session,
    owner: Entity,
    event: &str,
    cooldown: bool,
) {
    if cooldown {
        return;
    }
    crate::audio::voice(commands, game, session, event, owner, session.shots);
    if event != "pickup_shovel" {
        commands.entity(owner).insert(DigVoice { remaining: 60. });
    }
}
