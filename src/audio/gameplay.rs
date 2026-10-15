use super::*;
use crate::{
    gameplay::{GameStep, Session},
    life::{LifeState, Vitality},
    origins::EnemyAnimation,
    player::Player,
};
use avian3d::prelude::*;

#[derive(Default)]
struct ActionClock {
    key: String,
    sequence: u64,
    sampled: f32,
}
struct WeaponNotes {
    animation: usize,
    sequence: u64,
    sampled: f32,
}
#[derive(Default)]
struct ActorClock {
    position: Option<Vec3>,
    vocal: f32,
    action: Option<EnemyAnimation>,
    elapsed: f32,
    talking: Option<Entity>,
    animation: ActionClock,
}
#[derive(Component)]
struct AudioJoints([Option<Entity>; 3]);
#[derive(Resource, Default)]
struct AudioState {
    player: Option<Entity>,
    actions: BTreeMap<Entity, WeaponNotes>,
    actors: BTreeMap<Entity, ActorClock>,
    loops: BTreeMap<String, Entity>,
    perks: BTreeSet<crate::origins::PerkId>,
    round: u32,
    intermission: bool,
    health_heartbeat: f32,
    health_heartbeat_wait: f32,
    downed: bool,
    downed_origin: Vec3,
    heartbeat: f32,
    heartbeat_interval: f32,
    heartbeat_volume: f32,
    revive_gasp: Option<f32>,
    position: Option<Vec3>,
    crawl_distance: f32,
    raid: bool,
    raid_clock: f32,
}
#[derive(Resource, Default)]
struct VocalChoke {
    remaining: f32,
    calls: u8,
}
impl VocalChoke {
    fn allow(&mut self) -> bool {
        if self.calls >= 5 {
            return false;
        }
        self.calls += 1;
        true
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<VocalChoke>()
        .add_observer(bind_actor_joints);
    app.init_resource::<TriggerAudio>()
        .add_observer(trigger_sound)
        .add_systems(OnEnter(GameState::StartingSession), reset_trigger_audio)
        .add_systems(
            FixedUpdate,
            trigger_audio
                .after(GameStep::Rounds)
                .run_if(in_state(GameState::Playing)),
        );
    app.init_resource::<AudioState>().add_systems(
        FixedUpdate,
        (actors, world, feedback)
            .chain()
            .after(GameStep::Rounds)
            .run_if(in_state(GameState::Playing)),
    );
    app.add_systems(
        PostUpdate,
        actions
            .after(bevy::app::AnimationSystems)
            .after(crate::presentation::CameraPose)
            .before(AudioPlayback)
            .run_if(in_state(GameState::Playing)),
    );
    app.add_systems(OnEnter(GameState::StartingSession), reset_gameplay_audio);
}
fn reset_gameplay_audio(mut state: ResMut<AudioState>, mut choke: ResMut<VocalChoke>) {
    *state = AudioState::default();
    *choke = VocalChoke::default();
}

fn bind_actor_joints(
    event: On<Add<crate::hit_regions::SkeletalReady>>,
    mut commands: Commands,
    actors: Query<(), With<crate::enemies::Zombie>>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    if !actors.contains(event.entity) {
        return;
    }
    let mut joints = [None; 3];
    for entity in children.iter_descendants(event.entity) {
        if let Ok(name) = names.get(entity)
            && let Some(index) = ["j_ball_le", "j_ball_ri", "j_mainroot"]
                .iter()
                .position(|joint| *joint == name.as_str())
        {
            joints[index] = Some(entity);
        }
    }
    commands.entity(event.entity).insert(AudioJoints(joints));
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
    mut state: ResMut<AudioState>,
    rigs: Query<(Entity, &crate::presentation::Rig, &AnimationPlayer)>,
) {
    state.actions.retain(|entity, _| {
        rigs.get(*entity)
            .is_ok_and(|(_, rig, player)| rig.action_sample(player).is_some())
    });
    for (entity, rig, player) in &rigs {
        let Some((owner, _, clip, sequence, sampled, looping)) = rig.action_sample(player) else {
            continue;
        };
        let animation_index = || {
            game.content
                .animations
                .iter()
                .position(|animation| animation.id == *clip)
                .unwrap_or_else(|| {
                    panic!(
                        "selected weapon animation {} is missing from the catalog",
                        clip.0
                    )
                })
        };
        let cursor = state.actions.entry(entity).or_insert_with(|| WeaponNotes {
            animation: animation_index(),
            sequence,
            sampled: -0.001,
        });
        if game.content.animations[cursor.animation].id != *clip {
            cursor.animation = animation_index();
            cursor.sampled = -0.001;
        }
        if cursor.sequence != sequence || sampled < cursor.sampled {
            cursor.sampled = -0.001;
        }
        let animation = &game.content.animations[cursor.animation];
        if let Some(notes) = game.audio.timelines.get(&clip.0) {
            for note in notes {
                let count = note_crossings(
                    note.seconds,
                    cursor.sampled,
                    sampled,
                    animation.seconds,
                    looping,
                );
                for _ in 0..count {
                    emit(
                        &mut commands,
                        note.alias.clone(),
                        SoundSource::attached(owner),
                    );
                }
            }
        }
        cursor.sampled = sampled;
        cursor.sequence = sequence;
    }
}
fn actors(
    mut commands: Commands,
    game: Res<GameDefinition>,
    time: Res<Time<Fixed>>,
    mut state: ResMut<AudioState>,
    mut session: ResMut<Session>,
    mut playback: MessageReader<PlaybackEvent>,
    mut choke: ResMut<VocalChoke>,
    mixer: Res<Mixer>,
    mut footstep_frame: Local<(u64, usize)>,
    zombies: Query<(
        Entity,
        &crate::enemies::Zombie,
        &Transform,
        &Vitality,
        Option<&crate::generators::GeneratorDefender>,
        Option<&crate::rounds::PopulationKind>,
        Option<&AudioJoints>,
    )>,
    joints: Query<&GlobalTransform>,
    spatial: SpatialQuery,
    surfaces: Query<&crate::movement::physics::SurfaceFlags>,
) {
    let finished = playback
        .read()
        .filter(|event| event.status != PlaybackStatus::Accepted)
        .map(|event| event.entity)
        .collect::<BTreeSet<_>>();
    let alive = zombies
        .iter()
        .filter(|(_, _, _, health, defender, population, _)| {
            health.state() == LifeState::ActorAlive
                && defender.is_none()
                && population
                    .is_none_or(|kind| *kind != crate::rounds::PopulationKind::GeneratorDefender)
        })
        .count();
    choke.remaining -= time.delta_secs();
    if choke.remaining <= 0. {
        choke.remaining = 0.1;
        choke.calls = 0;
    }
    state.actors.retain(|e, _| zombies.contains(*e));
    if footstep_frame.0 != mixer.native_frame {
        *footstep_frame = (mixer.native_frame, 0);
    }
    for (owner, zombie, pose, health, defender, population, bound_joints) in &zombies {
        let capture_actor = defender.is_some()
            || population
                .is_some_and(|kind| *kind == crate::rounds::PopulationKind::GeneratorDefender);
        let clock = state.actors.entry(owner).or_default();
        let speed_squared = clock.position.map_or(0., |position| {
            position.xz().distance_squared(pose.translation.xz()) / time.delta_secs().powi(2)
        });
        clock.position = Some(pose.translation);
        if clock
            .talking
            .is_some_and(|entity| finished.contains(&entity))
        {
            clock.talking = None;
        }
        if let Some(animation) = game
            .content
            .enemies
            .iter()
            .find(|enemy| enemy.id == zombie.definition)
            .and_then(|enemy| zombie.animation_clip(enemy, &game))
        {
            let clock = &mut clock.animation;
            if clock.key != animation.id.0
                || clock.sequence != zombie.animation_sequence()
                || zombie.elapsed < clock.sampled
            {
                clock.key = animation.id.0.clone();
                clock.sequence = zombie.animation_sequence();
                clock.sampled = -0.001;
            }
            if let Some(notes) = game.audio.timelines.get(&animation.id.0) {
                for note in notes {
                    let count = note_crossings(
                        note.seconds,
                        clock.sampled,
                        zombie.elapsed,
                        animation.seconds,
                        animation.looping,
                    );
                    for _ in 0..count {
                        emit(
                            &mut commands,
                            note.alias.clone(),
                            SoundSource::attached(owner),
                        );
                    }
                }
            }
            for note in animation.notifications.iter().filter(|note| {
                note.event.starts_with("footstep_left_")
                    || note.event.starts_with("footstep_right_")
            }) {
                let count = note_crossings(
                    note.seconds,
                    clock.sampled,
                    zombie.elapsed,
                    animation.seconds,
                    animation.looping,
                );
                if count == 0 || footstep_frame.1 >= 16 {
                    continue;
                }
                let joint_position = |index| {
                    bound_joints
                        .and_then(|bound| bound.0[index])
                        .and_then(|entity| joints.get(entity).ok())
                        .map(GlobalTransform::translation)
                };
                let foot = joint_position(usize::from(note.event.starts_with("footstep_right_")))
                    .unwrap_or(pose.translation);
                let mainroot = joint_position(2);
                let mut surface = spatial
                    .cast_ray_predicate(
                        foot + Vec3::Y * 15. * 0.0254,
                        Dir3::NEG_Y,
                        30. * 0.0254,
                        true,
                        &crate::environment::solids([owner]),
                        &|entity| surfaces.contains(entity),
                    )
                    .and_then(|hit| surfaces.get(hit.entity).ok())
                    .map_or(0, |flags| (flags.0 >> 20 & 63) as usize);
                if game.audio.water.touches(foot) {
                    surface = 20;
                }
                let movement =
                    footstep_movement(speed_squared, mainroot.map(|root| root.y - foot.y));
                let alias = game
                    .audio
                    .footsteps
                    .get(surface)
                    .and_then(|aliases| aliases[movement].as_ref());
                for _ in 0..count {
                    if footstep_frame.1 >= 16 {
                        break;
                    }
                    footstep_frame.1 += 1;
                    if let Some(alias) = alias {
                        emit(&mut commands, alias.clone(), SoundSource::World(foot));
                    }
                }
            }
            clock.sampled = zombie.elapsed;
        }
        if health.state() != LifeState::ActorAlive {
            continue;
        }
        if clock.action != Some(zombie.action) || zombie.elapsed < clock.elapsed {
            let alias = match zombie.action {
                EnemyAnimation::Attack => Some(if capture_actor {
                    "zmb_vocals_capzomb_attack"
                } else {
                    "zmb_vocals_zombie_attack"
                }),
                EnemyAnimation::Tear => Some(match zombie.barrier_vocal() {
                    Some("attack") if capture_actor => "zmb_vocals_capzomb_attack",
                    Some("attack") => "zmb_vocals_zombie_attack",
                    Some("taunt") => "zmb_vocals_zombie_taunt",
                    _ => "zmb_vocals_zombie_teardown",
                }),
                _ => None,
            };
            if let Some(alias) = alias
                && choke.allow()
            {
                emit(&mut commands, alias, SoundSource::attached(owner));
            }
        }
        clock.action = Some(zombie.action);
        clock.elapsed = zombie.elapsed;
        clock.vocal -= time.delta_secs();
        if clock.vocal <= 0. {
            let alias = match (
                capture_actor,
                matches!(
                    zombie.gait,
                    crate::rounds::Gait::Run | crate::rounds::Gait::Sprint
                ),
            ) {
                (true, true) => "zmb_vocals_capzomb_sprint",
                (true, false) => "zmb_vocals_capzomb_ambience",
                (false, true) => "zmb_vocals_zombie_sprint",
                (false, false) => "zmb_vocals_zombie_ambience",
            };
            let alias = if zombie.crawler() && !capture_actor {
                "zmb_vocals_zombie_crawler"
            } else {
                alias
            };
            if clock.talking.is_none() && choke.allow() {
                let alias = if alive <= 1 {
                    format!("{alias}_loud")
                } else {
                    alias.into()
                };
                clock.talking = Some(emit(&mut commands, alias, SoundSource::attached(owner)));
            }
            clock.vocal = 1. + session.random() as f32 / u32::MAX as f32 * 3.;
        }
    }
}
fn note_crossings(
    seconds: f32,
    previous: f32,
    current: f32,
    duration: f32,
    looping: bool,
) -> usize {
    if looping {
        let first = ((previous - seconds) / duration).floor() as i64 + 1;
        let last = ((current - seconds) / duration).floor() as i64;
        (last - first.max(0) + 1).max(0) as usize
    } else {
        usize::from(seconds > previous && seconds <= current)
    }
}
fn footstep_movement(speed_squared: f32, root_height: Option<f32>) -> usize {
    if let Some(height) = root_height {
        if height < 6. * 0.0254 {
            return 5;
        }
        if height < 22. * 0.0254 {
            return if speed_squared >= (100. * 0.0254_f32).powi(2) {
                3
            } else {
                4
            };
        }
    }
    if speed_squared >= (170. * 0.0254_f32).powi(2) {
        0
    } else if speed_squared >= (100. * 0.0254_f32).powi(2) {
        1
    } else {
        2
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
    session_id: Res<crate::session::SessionId>,
    mut started: MessageReader<crate::rounds::RoundStarted>,
) {
    let Ok((player, pose, health, perks, body)) = players.single() else {
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
            health_heartbeat: 2.2,
            health_heartbeat_wait: 0.2,
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
    state.health_heartbeat -= time.delta_secs();
    if state.health_heartbeat <= 0. && health.current().is_some_and(|health| health > 0) {
        state.health_heartbeat += 0.2;
        if health.current().unwrap() as f32 >= health.maximum() as f32 * 0.35 {
            state.health_heartbeat_wait = 0.3;
        } else {
            emit(
                &mut commands,
                "mpl_player_heartbeat",
                SoundSource::attached(player),
            );
            state.health_heartbeat += state.health_heartbeat_wait;
            if state.health_heartbeat_wait <= 0.6 {
                state.health_heartbeat_wait += 0.1;
            }
        }
    }
    let downed = health.state() == LifeState::Downed;
    if downed && !state.downed {
        state.downed_origin = position;
        state.heartbeat = 0.;
        state.heartbeat_interval = 0.5;
        state.heartbeat_volume = 0.5;
        state.revive_gasp = None;
    }
    if downed {
        wanted.insert(
            "player.laststand".into(),
            (
                "evt_laststand_loop".into(),
                SoundSource::World(state.downed_origin),
            ),
        );
        state.heartbeat -= time.delta_secs();
        if state.heartbeat <= 0. {
            let heartbeat = emit(
                &mut commands,
                "chr_heart_beat",
                SoundSource::attached(player),
            );
            commands.entity(heartbeat).insert(SoundControl {
                volume: state.heartbeat_volume,
                ..default()
            });
            state.heartbeat += state.heartbeat_interval;
            state.heartbeat_interval = (state.heartbeat_interval * 1.05).min(2.);
            state.heartbeat_volume = (state.heartbeat_volume * 1.05).min(1.);
        }
    }
    let last_stand_ended = state.downed && !downed;
    if last_stand_ended {
        if let Some(looping) = state.loops.remove("player.laststand") {
            commands.entity(looping).try_insert(SoundFade {
                remaining: 0.7,
                duration: 0.7,
                out: true,
            });
        }
        emit(
            &mut commands,
            "evt_laststand_in",
            SoundSource::World(Vec3::ZERO),
        );
        state.revive_gasp = Some(0.7);
        if health.state() == LifeState::Standing {
            voice(
                &mut commands,
                &game,
                &session,
                "heal_revived",
                player,
                session.kills,
            );
        }
    }
    state.downed = downed;
    if !last_stand_ended && let Some(remaining) = state.revive_gasp.as_mut() {
        *remaining -= time.delta_secs();
        if *remaining <= 0. {
            emit(&mut commands, "revive_gasp", SoundSource::attached(player));
            state.revive_gasp = None;
        }
    }
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
    let mut round_started = state.round == 0;
    for event in started.read() {
        round_started |= event.session == *session_id
            && event.round == session.round
            && event.round > 1
            && state.round != event.round;
    }
    if round_started {
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
            emit(&mut commands, "evt_belch", SoundSource::attached(player));
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
            if key == "player.laststand" {
                commands.entity(entity).insert(SoundControl {
                    volume: 0.3,
                    ..default()
                });
            }
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
    mut choke: ResMut<VocalChoke>,
    mut steps: MessageReader<crate::environment::Footstep>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut damages: MessageReader<crate::player_health::PlayerDamage>,
    mut applied: MessageReader<crate::damage::DamageApplied>,
    poses: Query<&Transform>,
    populations: Query<&crate::rounds::PopulationKind>,
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
        if choke.allow() {
            emit(
                &mut commands,
                if populations
                    .get(receipt.impact.target)
                    .is_ok_and(|kind| *kind == crate::rounds::PopulationKind::GeneratorDefender)
                {
                    "zmb_vocals_capzomb_death"
                } else {
                    "zmb_vocals_zombie_death"
                },
                SoundSource::attached(receipt.impact.target),
            );
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_notifies_cross_loop_seams_once() {
        assert_eq!(note_crossings(0., 0.9, 1.1, 1., true), 1);
        assert_eq!(note_crossings(0.5, 0.9, 1.1, 1., true), 0);
        assert_eq!(note_crossings(0.1, -0.001, 2.2, 1., true), 3);
        assert_eq!(note_crossings(0.5, 0.5, 0.6, 1., false), 0);
        assert_eq!(note_crossings(0., -0.001, 0., 1., false), 1);
    }

    #[test]
    fn footstep_table_uses_native_speed_and_root_height_boundaries() {
        assert_eq!(footstep_movement(0., None), 2);
        assert_eq!(footstep_movement((100. * 0.0254_f32).powi(2), None), 1);
        assert_eq!(footstep_movement((170. * 0.0254_f32).powi(2), None), 0);
        assert_eq!(footstep_movement(99., Some(5. * 0.0254)), 5);
        assert_eq!(footstep_movement(0., Some(6. * 0.0254)), 4);
        assert_eq!(footstep_movement(99., Some(21. * 0.0254)), 3);
        assert_eq!(footstep_movement(0., Some(22. * 0.0254)), 2);
    }
}
