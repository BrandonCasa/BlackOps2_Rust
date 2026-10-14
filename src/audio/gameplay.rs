use super::*;
use crate::{
    gameplay::{GameStep, Session},
    life::{LifeState, Vitality},
    origins::{EnemyAnimation, WeaponAnimation},
    player::Player,
    weapons::{WeaponInventory, WeaponSignal},
};
use avian3d::prelude::*;

#[derive(Default)]
struct ActionClock {
    key: String,
    sequence: u64,
    age: f32,
    sampled: f32,
}
#[derive(Default)]
struct ActorClock {
    position: Option<Vec3>,
    distance: f32,
    vocal: f32,
    action: Option<EnemyAnimation>,
    elapsed: f32,
}
#[derive(Resource, Default)]
struct AudioState {
    player: Option<Entity>,
    actions: BTreeMap<(Entity, bool), ActionClock>,
    actors: BTreeMap<Entity, ActorClock>,
    loops: BTreeMap<String, Entity>,
    perks: BTreeSet<crate::origins::PerkId>,
    round: u32,
    intermission: bool,
    critical: bool,
    downed: bool,
    position: Option<Vec3>,
    crawl_distance: f32,
    raid: bool,
    raid_clock: f32,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<TriggerAudio>()
        .add_observer(trigger_sound)
        .add_systems(OnEnter(GameState::PreparingShaders), reset_trigger_audio)
        .add_systems(
            FixedUpdate,
            trigger_audio
                .after(GameStep::Rounds)
                .run_if(in_state(GameState::Playing)),
        );
    app.init_resource::<AudioState>().add_systems(
        FixedUpdate,
        (actions, actors, world, feedback)
            .chain()
            .after(GameStep::Rounds)
            .run_if(in_state(GameState::Playing)),
    );
}

#[derive(Resource, Default)]
struct TriggerAudio {
    clock: f32,
    capture_poll: f32,
    capture_enter_poll: BTreeMap<Entity, f32>,
    capture: Option<(Entity, Entity)>,
    capture_sound: Option<Entity>,
    background: String,
    background_until: f32,
    background_poll: BTreeMap<Entity, f32>,
    background_sample: f32,
    background_selected: String,
    background_pending: Option<(String, f32)>,
    background_sound: Option<Entity>,
    maelstrom_until: f32,
    maelstrom_poll: f32,
    maelstrom_sound: Option<Entity>,
    doors: BTreeSet<String>,
    door_poll: BTreeMap<String, f32>,
    stinger: Option<(Entity, f32)>,
}

fn reset_trigger_audio(mut state: ResMut<TriggerAudio>) {
    *state = TriggerAudio::default();
    state.background = "mus_underscore_default".into();
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn environment_trigger_audio_query_access() {
        let mut world = World::new();
        IntoSystem::into_system(trigger_audio).initialize(&mut world);
        world.add_observer(trigger_sound);
        world.flush();
    }
}

fn trigger_sound(
    event: On<crate::environment::triggers::TriggerActivated>,
    mut commands: Commands,
    game: Res<GameDefinition>,
    mut state: ResMut<TriggerAudio>,
    mut music: ResMut<MusicOverride>,
    players: Query<(), With<Player>>,
    triggers: Query<&crate::environment::triggers::MapTrigger>,
) {
    let Ok(trigger) = triggers.get(event.trigger) else {
        return;
    };
    let Some(source) = game
        .environment
        .triggers
        .iter()
        .find(|source| source.id == trigger.0)
    else {
        return;
    };
    if source
        .source
        .get("targetname")
        .is_some_and(|name| name == "sndMaelstrom")
        && players.contains(event.activator)
        && state.clock >= state.maelstrom_poll
    {
        state.maelstrom_poll = state.clock + 0.1;
        state.maelstrom_until = state.clock + 2.;
        if state.maelstrom_sound.is_none() {
            let entity = emit_loop(
                &mut commands,
                "amb_maelstrom",
                SoundSource::World(Vec3::ZERO),
            );
            commands.entity(entity).insert(SoundFade {
                remaining: 3.,
                duration: 3.,
                out: false,
            });
            state.maelstrom_sound = Some(entity);
        }
    }
    if source
        .source
        .get("script_noteworthy")
        .is_none_or(|name| name != "sndMusicDoor")
        || state.doors.contains(&source.id)
        || state
            .door_poll
            .get(&source.id)
            .is_some_and(|next| state.clock < *next)
    {
        return;
    }
    if music.0.is_some() {
        let next = state.clock + 0.1;
        state.door_poll.insert(source.id.clone(), next);
        return;
    }
    let Some(alias) = source.source.get("script_sound") else {
        return;
    };
    let Some(length) = source
        .source
        .get("script_int")
        .and_then(|value| value.parse::<f32>().ok())
        .filter(|length| length.is_finite() && *length >= 0.)
    else {
        return;
    };
    state.doors.insert(source.id.clone());
    if let Some(target) = source.source.get("target") {
        for paired in game
            .environment
            .triggers
            .iter()
            .filter(|paired| paired.source.get("targetname") == Some(target))
        {
            state.doors.insert(paired.id.clone());
        }
    }
    let sound = emit(&mut commands, alias.clone(), SoundSource::World(Vec3::ZERO));
    music.0 = Some(sound);
    state.stinger = Some((sound, state.clock + length));
}

fn trigger_audio(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut state: ResMut<TriggerAudio>,
    mut music: ResMut<MusicOverride>,
    quest: Res<crate::quests::MainQuest>,
    spatial: SpatialQuery,
    players: Query<(Entity, &Transform, &Collider), With<Player>>,
    triggers: Query<(
        &crate::environment::triggers::MapTrigger,
        &Transform,
        &crate::environment::triggers::TriggerState,
    )>,
) {
    state.clock += time.delta_secs();
    let now = state.clock;
    if let Some((entity, until)) = state.stinger {
        if now >= until {
            if music.0 == Some(entity) {
                music.0 = None;
            }
            if now >= until + 0.05 {
                commands.entity(entity).try_despawn();
                state.stinger = None;
            }
        }
    }
    if now >= state.maelstrom_until
        && let Some(entity) = state.maelstrom_sound.take()
    {
        commands.entity(entity).try_insert(SoundFade {
            remaining: 3.,
            duration: 3.,
            out: true,
        });
    }
    let Ok((player, pose, collider)) = players.single() else {
        return;
    };
    let touching = spatial.shape_intersections(
        collider,
        pose.translation,
        pose.rotation,
        &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
    );
    if now >= state.capture_poll {
        state.capture_poll = now + 0.1;
        if state
            .capture
            .is_some_and(|(trigger, owner)| owner != player || !touching.contains(&trigger))
        {
            if let Some((trigger, _)) = state.capture.take() {
                if let Ok((_, location, _)) = triggers.get(trigger) {
                    emit(
                        &mut commands,
                        "zmb_zone_plate_up",
                        SoundSource::World(location.translation),
                    );
                }
            }
            if let Some(entity) = state.capture_sound.take() {
                commands.entity(entity).try_insert(SoundFade {
                    remaining: 1.,
                    duration: 1.,
                    out: true,
                });
            }
        }
    }
    for entity in &touching {
        let capture = (|| {
            let (trigger, location, trigger_state) = triggers.get(*entity).ok()?;
            let source = game
                .environment
                .triggers
                .iter()
                .find(|source| source.id == trigger.0)?;
            (trigger_state.enabled
                && source
                    .source
                    .get("targetname")
                    .is_some_and(|name| name == "sndCaptureZone"))
            .then_some(location.translation)
        })();
        if let Some(location) = capture {
            if state
                .capture_enter_poll
                .get(entity)
                .is_some_and(|next| now < *next)
            {
                continue;
            }
            state.capture_enter_poll.insert(*entity, now + 0.05);
            if state.capture.is_none() {
                emit(
                    &mut commands,
                    "zmb_zone_plate_down",
                    SoundSource::World(location),
                );
                let sound = emit_loop(
                    &mut commands,
                    "zmb_zone_plate_loop",
                    SoundSource::World(location),
                );
                commands.entity(sound).insert(SoundFade {
                    remaining: 2.,
                    duration: 2.,
                    out: false,
                });
                state.capture = Some((*entity, player));
                state.capture_poll = now + 0.1;
                state.capture_sound = Some(sound);
            }
        }
    }
    for entity in &touching {
        let Ok((trigger, _, trigger_state)) = triggers.get(*entity) else {
            continue;
        };
        let Some(source) = game
            .environment
            .triggers
            .iter()
            .find(|source| source.id == trigger.0)
        else {
            continue;
        };
        if !trigger_state.enabled
            || source
                .source
                .get("targetname")
                .is_none_or(|name| name != "sndBG")
        {
            continue;
        }
        if state
            .background_poll
            .get(entity)
            .is_some_and(|next| now < *next)
        {
            continue;
        }
        state.background_poll.insert(*entity, now + 0.5);
        if let Some(alias) = source.source.get("script_sound") {
            state.background = if alias == "mus_underscore_chamber" && quest.enabled {
                match quest.stage {
                    crate::quests::QuestStage::UpgradeFists => {
                        "mus_underscore_chamber_fight_1".into()
                    }
                    crate::quests::QuestStage::Finale => "mus_underscore_chamber_fight_2".into(),
                    crate::quests::QuestStage::Complete => "null".into(),
                    _ => alias.clone(),
                }
            } else {
                alias.clone()
            };
            state.background_until = now + 4.;
        }
    }
    if now >= state.background_until {
        state.background = "mus_underscore_default".into();
    }
    if now >= state.background_sample {
        state.background_sample = now + 2.;
        if state.background_selected != state.background {
            state.background_selected = state.background.clone();
            if let Some(entity) = state.background_sound.take() {
                commands.entity(entity).try_insert(SoundFade {
                    remaining: 2.,
                    duration: 2.,
                    out: true,
                });
            }
            state.background_pending = Some((state.background.clone(), now + 2.));
        }
    }
    if state
        .background_pending
        .as_ref()
        .is_some_and(|(_, until)| now >= *until)
    {
        let (alias, _) = state.background_pending.take().unwrap();
        if alias != "null" {
            let sound = emit_loop(&mut commands, alias, SoundSource::World(Vec3::ZERO));
            commands.entity(sound).insert(SoundFade {
                remaining: 2.,
                duration: 2.,
                out: false,
            });
            state.background_sound = Some(sound);
        }
    }
}
pub(crate) fn voice(
    commands: &mut Commands,
    game: &GameDefinition,
    session: &Session,
    event: &str,
    speaker: Entity,
    index: u32,
) {
    let prefix = format!("vox_plr_{}_{}", session.character, event);
    let choices: Vec<_> = game
        .audio
        .aliases
        .keys()
        .filter(|n| n.starts_with(&prefix))
        .collect();
    if !choices.is_empty() {
        emit(
            commands,
            choices[index as usize % choices.len()].clone(),
            SoundSource::attached(speaker),
        );
    }
}
fn actions(
    mut commands: Commands,
    game: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    mut state: ResMut<AudioState>,
    players: Query<
        (
            Entity,
            &WeaponInventory,
            &crate::equipment::GrenadeInventory,
            &crate::equipment::TacticalInventory,
            &crate::mines::MineInventory,
            Option<&crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
) {
    state.actions.retain(|(e, _), _| players.contains(*e));
    for (owner, inventory, lethal, tactical, mines, drinking) in &players {
        let selected = drinking
            .map(|d| d.selection())
            .or_else(|| crate::presentation::offhand::selection(lethal, tactical, mines, &game));
        let Some(id) = selected
            .as_ref()
            .map(|s| &s.weapon)
            .or_else(|| inventory.active().map(|w| &w.definition))
        else {
            continue;
        };
        let Some(weapon) = game.content.weapons.iter().find(|w| w.id == *id) else {
            continue;
        };
        for left in [false, true] {
            let weapon = if left {
                if selected.is_some() {
                    continue;
                }
                let Some(partner) = weapon
                    .dual_weapon
                    .as_ref()
                    .and_then(|id| game.content.weapons.iter().find(|w| w.id == *id))
                else {
                    continue;
                };
                partner
            } else {
                weapon
            };
            let signal = inventory.hand_signal(left);
            let (action, shot_seek) = selected.as_ref().map_or_else(
                || crate::presentation::hand_action(inventory, weapon, &game, left),
                |s| (s.action, None),
            );
            if left && action == WeaponAnimation::Melee {
                state.actions.remove(&(owner, left));
                continue;
            }
            let animation_weapon = if action == WeaponAnimation::Melee {
                game.content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.knife_zm")
                    .unwrap_or(weapon)
            } else {
                weapon
            };
            let Some(clip) = animation_weapon.animation_actions.get(&action) else {
                continue;
            };
            let clock = state.actions.entry((owner, left)).or_default();
            let sequence = if selected.is_some() {
                0
            } else {
                inventory.hand_sequence(left)
            };
            let timing = selected
                .as_ref()
                .and_then(|s| s.timing)
                .or_else(|| inventory.hand_reload_timing(left))
                .or_else(|| inventory.presentation_timing(weapon, animation_weapon));
            let backwards = timing
                .is_some_and(|(duration, remaining)| duration - remaining + 0.001 < clock.age);
            let finishing_shot =
                selected.is_none() && shot_seek.is_some() && signal == WeaponSignal::Idle;
            let changed =
                clock.key != clip.0 || (clock.sequence != sequence && !finishing_shot) || backwards;
            if changed {
                clock.key = clip.0.clone();
                clock.sequence = sequence;
                clock.age = 0.;
                clock.sampled = -0.001;
                let event = match action {
                    WeaponAnimation::Equip => Some("equip"),
                    WeaponAnimation::Unequip => Some("unequip"),
                    WeaponAnimation::Prime => Some("prime"),
                    WeaponAnimation::Melee => Some("melee"),
                    WeaponAnimation::Fire if selected.is_some() => Some("throw"),
                    _ => None,
                };
                if let Some(event) = event {
                    binding(
                        &mut commands,
                        &game,
                        &format!("{}.{}", weapon.id.0, event),
                        SoundSource::attached(owner),
                    );
                }
            }
            let native_duration = game
                .content
                .animations
                .iter()
                .find(|a| a.id == *clip)
                .and_then(|a| a.notifications.last())
                .map_or(1., |n| n.seconds);
            let sampled = if let Some((duration, remaining)) = timing {
                clock.age = (duration - remaining).max(0.);
                clock.age / duration * native_duration
            } else if selected.is_none() && shot_seek.is_some() {
                clock.age = shot_seek.unwrap_or(0.);
                clock.age.min(native_duration)
            } else {
                clock.age += time.delta_secs();
                clock.age
            };
            if let Some(notes) = game.audio.timelines.get(&clip.0) {
                for note in notes
                    .iter()
                    .filter(|n| n.seconds > clock.sampled && n.seconds <= sampled)
                {
                    emit(
                        &mut commands,
                        note.alias.clone(),
                        SoundSource::attached(owner),
                    );
                }
            }
            clock.sampled = sampled;
            clock.sequence = sequence;
        }
    }
}
fn actors(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<AudioState>,
    zombies: Query<(
        Entity,
        &crate::enemies::Zombie,
        &Transform,
        &Vitality,
        Option<&crate::generators::GeneratorDefender>,
    )>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
) {
    state.actors.retain(|e, _| zombies.contains(*e));
    for (owner, zombie, pose, health, defender) in &zombies {
        let new_actor = !state.actors.contains_key(&owner);
        let clock = state.actors.entry(owner).or_default();
        if new_actor && defender.is_some() {
            emit(
                &mut commands,
                "zmb_vocals_capzomb_spawn",
                SoundSource::attached(owner),
            );
        }
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        let moving = matches!(
            zombie.action,
            EnemyAnimation::Walk | EnemyAnimation::Run | EnemyAnimation::Sprint
        );
        let distance = clock
            .position
            .map_or(0., |p| p.xz().distance(pose.translation.xz()));
        clock.position = Some(pose.translation);
        if moving && distance < 0.5 {
            clock.distance += distance;
            if clock.distance >= 0.75 {
                clock.distance %= 0.75;
                let surface = ground_surface(&query, &surfaces, pose.translation);
                let gait = match zombie.action {
                    EnemyAnimation::Run => "run",
                    EnemyAnimation::Sprint => "sprint",
                    _ => "walk",
                };
                emit(
                    &mut commands,
                    format!("fly_step_{gait}_npc_{surface}"),
                    SoundSource::attached(owner),
                );
            }
        }
        if clock.action != Some(zombie.action) || zombie.elapsed < clock.elapsed {
            if zombie.action == EnemyAnimation::Attack {
                emit(
                    &mut commands,
                    "zmb_attack_whoosh",
                    SoundSource::attached(owner),
                );
            }
            let alias = match zombie.action {
                EnemyAnimation::Attack => Some(if defender.is_some() {
                    "zmb_vocals_capzomb_attack"
                } else {
                    "zmb_vocals_zombie_attack"
                }),
                EnemyAnimation::Tear => Some("zmb_vocals_zombie_teardown"),
                _ => None,
            };
            if let Some(alias) = alias {
                emit(&mut commands, alias, SoundSource::attached(owner));
            }
        }
        clock.action = Some(zombie.action);
        clock.elapsed = zombie.elapsed;
        clock.vocal -= time.delta_secs();
        if clock.vocal <= 0. {
            emit(
                &mut commands,
                match (defender.is_some(), zombie.action == EnemyAnimation::Sprint) {
                    (true, true) => "zmb_vocals_capzomb_sprint",
                    (true, false) => "zmb_vocals_capzomb_ambience",
                    (false, true) => "zmb_vocals_zombie_sprint",
                    (false, false) => "zmb_vocals_zombie_ambience",
                },
                SoundSource::attached(owner),
            );
            clock.vocal = 4. + (owner.to_bits() % 37) as f32 * 0.1;
        }
    }
}
fn world(
    mut commands: Commands,
    game: Res<GameDefinition>,
    session: Res<Session>,
    mut state: ResMut<AudioState>,
    players: Query<
        (
            Entity,
            &Transform,
            &Vitality,
            &crate::player_health::Recovery,
            &crate::solo_revive::PerkInventory,
            &Player,
        ),
        With<Player>,
    >,
    bombs: Query<(Entity, &crate::equipment::Grenade, &Transform)>,
    exists: Query<Entity>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
    (time, generators, music): (
        Res<Time<Fixed>>,
        Res<crate::generators::Generators>,
        Res<MusicOverride>,
    ),
) {
    let Ok((player, pose, health, recovery, perks, body)) = players.single() else {
        return;
    };
    if state.player != Some(player) {
        for entity in state.loops.values() {
            if exists.contains(*entity) {
                commands.entity(*entity).try_despawn();
            }
        }
        *state = AudioState {
            player: Some(player),
            ..default()
        };
    }
    let position = pose.translation;
    let distance = state
        .position
        .map_or(0., |p| p.xz().distance(position.xz()));
    state.position = Some(position);
    if (body.prone || health.state() == LifeState::Downed) && body.grounded && distance < 0.5 {
        state.crawl_distance += distance;
        if state.crawl_distance >= 0.6 {
            state.crawl_distance %= 0.6;
            let surface = ground_surface(&query, &surfaces, position);
            let alias = if surface == "water" {
                "fly_prone_water".to_owned()
            } else {
                format!("fly_prone_{surface}_plr")
            };
            if game.audio.aliases.contains_key(&alias) {
                emit(&mut commands, alias, SoundSource::attached(player));
            }
        }
    } else {
        state.crawl_distance = 0.;
    }
    let mut wanted = BTreeMap::<String, (String, SoundSource)>::new();
    let critical = recovery.critical || health.state() == LifeState::Downed;
    if critical {
        wanted.insert(
            "player.critical".into(),
            ("chr_breathing_hurt".into(), SoundSource::attached(player)),
        );
        if !state.critical {
            emit(
                &mut commands,
                "chr_breathing_hurt_start",
                SoundSource::attached(player),
            );
        }
    } else if state.critical {
        emit(
            &mut commands,
            "chr_breathing_better",
            SoundSource::attached(player),
        );
    }
    state.critical = critical;
    let downed = health.state() == LifeState::Downed;
    if state.downed && !downed {
        emit(
            &mut commands,
            "chr_revive_gasp",
            SoundSource::attached(player),
        );
        voice(
            &mut commands,
            &game,
            &session,
            "heal_revived",
            player,
            session.kills,
        );
    }
    state.downed = downed;
    if generators.raid_active != state.raid {
        emit(
            &mut commands,
            if generators.raid_active {
                "mus_zombie_dog_start"
            } else {
                "mus_zombie_dog_end"
            },
            SoundSource::World(position),
        );
        state.raid = generators.raid_active;
        state.raid_clock = 0.;
        if !state.raid
            && let Some(looping) = state.loops.remove("raid.music")
        {
            commands.entity(looping).try_insert(SoundFade {
                remaining: 2.,
                duration: 2.,
                out: true,
            });
        }
    }
    if state.raid {
        state.raid_clock += time.delta_secs();
    }
    if state.raid && state.raid_clock >= 5. && music.0.is_none() {
        wanted.insert(
            "raid.music".into(),
            (
                "mus_recapture_round_loop".into(),
                SoundSource::World(position),
            ),
        );
    }
    if state.round != session.round {
        if !state.raid {
            emit(
                &mut commands,
                "mus_zombie_round_start",
                SoundSource::World(position),
            );
        }
        state.round = session.round;
    }
    if session.intermission.is_some() && !state.intermission && !state.raid {
        emit(
            &mut commands,
            "mus_zombie_round_over",
            SoundSource::World(position),
        );
    }
    state.intermission = session.intermission.is_some();
    for perk in game
        .content
        .perks
        .iter()
        .filter(|p| perks.owned.contains(&p.id) && !state.perks.contains(&p.id))
    {
        let native = perk_music(&perk.id.0);
        if let Some(native) = native {
            emit(
                &mut commands,
                format!("mus_perks_{native}_sting"),
                SoundSource::World(position),
            );
            let event = match native {
                "jugganog" => Some("perk_jugga"),
                "mulekick" => Some("perk_mule"),
                "revive" => Some("perk_revive"),
                "speed" => Some("perk_speed"),
                "stamin" => Some("perk_stamine"),
                _ => None,
            };
            if let Some(event) = event {
                voice(&mut commands, &game, &session, event, player, 0);
            }
        }
    }
    state.perks = perks.owned.clone();
    for placement in &game.content.placements {
        if placement.definition.0 == "perk.quick_revive"
            && perks.solo_purchases >= game.solo_revive.maximum_purchases
            && !perks.owned.contains(&game.solo_revive.perk)
        {
            continue;
        }
        if !game
            .content
            .perks
            .iter()
            .any(|p| p.id.0 == placement.definition.0)
        {
            continue;
        }
        let Some(native) = perk_music(&placement.definition.0) else {
            continue;
        };
        let p = Vec3::from_array(placement.position);
        wanted.insert(
            placement.id.0.clone(),
            (format!("mus_perks_{native}_jingle"), SoundSource::World(p)),
        );
    }
    for (i, emitter) in game.audio.emitters.iter().enumerate() {
        let p = Vec3::from_array(emitter.position);
        wanted.insert(
            format!("ambient.{i}"),
            (emitter.alias.clone(), SoundSource::World(p)),
        );
    }
    for (e, bomb, _) in &bombs {
        if bomb.lure_active {
            wanted.insert(
                format!("monkey.{e:?}"),
                ("zmb_monkey_song".into(), SoundSource::attached(e)),
            );
        } else if bomb.definition.0 == "weapon.sticky_grenade_zm" {
            wanted.insert(
                format!("semtex.{e:?}"),
                ("wpn_semtex_charge".into(), SoundSource::attached(e)),
            );
        }
    }
    state.loops.retain(|key, e| {
        let keep = wanted.contains_key(key) && exists.contains(*e);
        if !keep && exists.contains(*e) {
            commands.entity(*e).try_despawn();
        }
        keep
    });
    for (key, (alias, source)) in wanted {
        if !state.loops.contains_key(&key) && game.audio.aliases.contains_key(&alias) {
            let entity = emit_loop(&mut commands, alias, source);
            if key == "raid.music" {
                commands.entity(entity).insert(SoundFade {
                    remaining: 5.,
                    duration: 5.,
                    out: false,
                });
            }
            state.loops.insert(key, entity);
        }
    }
}
fn perk_music(id: &str) -> Option<&'static str> {
    Some(match id {
        "perk.quick_revive" => "revive",
        "perk.juggernog" => "jugganog",
        "perk.speed_cola" => "speed",
        "perk.stamin_up" => "stamin",
        "perk.mule_kick" => "mulekick",
        "perk.double_tap" => "doubletap",
        "perk.deadshot" => "deadshot",
        "perk.electric_cherry" => "cherry",
        "perk.phd_flopper" => "phdflopper",
        _ => return None,
    })
}
fn feedback(
    mut commands: Commands,
    game: Res<GameDefinition>,
    session: Res<Session>,
    mut steps: MessageReader<crate::environment::Footstep>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut damages: MessageReader<crate::player_health::PlayerDamage>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    poses: Query<&Transform>,
    defenders: Query<(), With<crate::generators::GeneratorDefender>>,
    nuked: Query<(), With<crate::powerups::NukeTarget>>,
    players: Query<(Entity, &crate::movement::SprintState, &Player), With<Player>>,
    query: SpatialQuery,
    surfaces: Query<&SurfaceSound>,
) {
    let Ok((player, _, _)) = players.single() else {
        return;
    };
    for damage in applied.read() {
        if matches!(
            damage.impact.category,
            crate::origins::DamageCategory::Melee
        ) && let Ok(pose) = poses.get(damage.impact.target)
        {
            binding(
                &mut commands,
                &game,
                "weapon.knife_zm.hit",
                SoundSource::World(pose.translation),
            );
        }
    }
    for step in steps.read() {
        let surface = match step.surface {
            crate::environment::FootstepSurface::Mud => "mud",
            _ => ground_surface(&query, &surfaces, step.position + Vec3::Y * 0.25),
        };
        let gait = if players.get(step.owner).is_ok_and(|(_, s, _)| s.sprinting) {
            "sprint"
        } else if players.get(step.owner).is_ok_and(|(_, _, p)| p.crouched) {
            "crouch_walk"
        } else {
            "walk"
        };
        emit(
            &mut commands,
            format!("fly_step_{gait}_plr_{surface}"),
            SoundSource::Entity {
                entity: step.owner,
                offset: players
                    .get(step.owner)
                    .map_or(Vec3::ZERO, |(_, _, body)| Vec3::NEG_Y * body.height * 0.5),
            },
        );
    }
    for crate::damage::EnemyDeath(receipt) in deaths.read() {
        emit(
            &mut commands,
            if defenders.contains(receipt.impact.target) {
                "zmb_vocals_capzomb_death"
            } else {
                "zmb_vocals_zombie_death"
            },
            SoundSource::attached(receipt.impact.target),
        );
        if nuked.contains(receipt.impact.target) {
            continue;
        }
        let event = if receipt.impact.region.head() {
            Some("kill_headshot")
        } else if matches!(
            receipt.impact.source,
            crate::damage::ImpactSource::Projectile(_)
        ) {
            Some("kill_explo")
        } else if matches!(
            receipt.impact.category,
            crate::origins::DamageCategory::Melee
        ) {
            Some("kill_close")
        } else {
            None
        };
        if let Some(event) = event {
            voice(&mut commands, &game, &session, event, player, session.kills);
        }
    }
    for damage in damages.read() {
        if damage.impact.target == player {
            voice(
                &mut commands,
                &game,
                &session,
                if damage.after < 25 {
                    "exert_pain_high"
                } else {
                    "exert_pain_low"
                },
                player,
                session.shots,
            );
        }
    }
}

fn ground_surface<'a>(
    query: &SpatialQuery,
    surfaces: &'a Query<&SurfaceSound>,
    position: Vec3,
) -> &'a str {
    query
        .cast_ray_predicate(
            position,
            Dir3::NEG_Y,
            2.,
            true,
            &crate::environment::solids([]),
            &|e| surfaces.contains(e),
        )
        .and_then(|hit| surfaces.get(hit.entity).ok())
        .map_or("default", |s| s.0.as_str())
}
