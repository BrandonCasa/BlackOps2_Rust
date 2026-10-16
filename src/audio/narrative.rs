use super::*;
use crate::{
    gameplay::{GameStep, Session},
    player::Player,
};
use std::collections::VecDeque;
#[derive(Message, Clone, Copy)]
pub(crate) struct NarrativeProgress {
    pub player: Entity,
}
#[derive(Message)]
pub(crate) enum NarrativeEvent {
    DroneBuilt(SoundSource),
    StaffTaken(usize),
    Samantha { alias: String, source: SoundSource },
}
#[derive(Message, Clone, Copy)]
pub(crate) struct RobotPurgeReady {
    pub player: Entity,
    pub robot: usize,
}
pub(crate) fn samantha(commands: &mut Commands, alias: impl Into<String>, source: SoundSource) {
    commands.write_message(NarrativeEvent::Samantha {
        alias: alias.into(),
        source,
    });
}
struct Line {
    alias: String,
    source: SoundSource,
    robot: Option<usize>,
    gap: f32,
    purge_ready: Option<usize>,
    intro_end: bool,
}
#[derive(Resource, Default)]
struct Narrative {
    entries: u32,
    evacuations: u32,
    raid: Option<String>,
    staffs_claimed: BTreeSet<usize>,
    queue: VecDeque<Line>,
    active: Option<Entity>,
    owner: Option<Entity>,
    wait: f32,
    opening: f32,
    opening_done: bool,
    intro_done: bool,
    robot: Option<usize>,
    occupancy_seconds: f32,
    purge_started: bool,
    pools: BTreeMap<String, Vec<String>>,
    encourage_after: f32,
    clock: f32,
    pap_discovered: BTreeSet<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Narrative>()
        .add_observer(discover_pap)
        .add_message::<NarrativeProgress>()
        .add_message::<NarrativeEvent>()
        .add_message::<RobotPurgeReady>()
        .add_systems(OnEnter(GameState::StartingSession), reset)
        .add_systems(
            FixedUpdate,
            events
                .after(GameStep::Rounds)
                .run_if(in_state(GameState::Playing)),
        );
}

fn discover_pap(
    event: On<crate::environment::triggers::TriggerActivated>,
    mut state: ResMut<Narrative>,
    game: Res<GameDefinition>,
    mut session: ResMut<Session>,
    reservation: Res<StoryReservation>,
    players: Query<(Entity, &Player, &Transform)>,
    triggers: Query<&crate::environment::triggers::MapTrigger>,
    sounds: Query<&PlayingSound>,
) {
    if state.pap_discovered.contains(&event.activator) || reservation.0.is_some() {
        return;
    }
    let Ok(trigger) = triggers.get(event.trigger) else {
        return;
    };
    let Some(source) = game.environment.triggers.iter().find(|source| {
        source.id == trigger.0
            && source
                .source
                .get("targetname")
                .is_some_and(|name| name == "pack_a_punch_intro_trigger")
    }) else {
        return;
    };
    let Some(target) = source.target_position.map(Vec3::from_array) else {
        return;
    };
    let Ok((speaker, player, pose)) = players.get(event.activator) else {
        return;
    };
    let origin = pose.translation - Vec3::Y * player.height * 0.5;
    let direction = Vec3::new(
        -player.yaw.sin() * player.pitch.cos(),
        player.pitch.sin(),
        -player.yaw.cos() * player.pitch.cos(),
    );
    if direction.dot((target - origin).normalize_or_zero()) <= 0.8
        || sounds.iter().any(|sound| {
            sound.channel == Channel::Dialogue
                && matches!(sound.source, SoundSource::Entity { entity, .. } if entity == speaker)
        })
    {
        return;
    }
    if category(
        &mut state,
        &game,
        &mut session,
        "pap_discovered",
        SoundSource::attached(speaker),
        None,
    ) {
        for (entity, _, location) in &players {
            if location.translation.distance(pose.translation) < 800. * 0.0254 {
                state.pap_discovered.insert(entity);
            }
        }
    }
}

#[cfg(test)]
mod trigger_tests {
    use super::*;

    #[test]
    fn environment_trigger_discovery_observer_access() {
        let mut world = World::new();
        world.add_observer(discover_pap);
        world.flush();
    }

    fn app() -> App {
        let mut app = App::new();
        app.insert_resource(super::super::dialogue::tests::game())
            .init_resource::<Time<Fixed>>()
            .init_resource::<Session>()
            .init_resource::<crate::robots::Robots>()
            .init_resource::<crate::generators::GeneratorRaids>()
            .init_resource::<crate::crafting::Crafting>()
            .init_resource::<crate::side_secrets::Secrets>()
            .init_resource::<StoryReservation>()
            .insert_resource(crate::session::SessionId(1))
            .insert_resource(Narrative {
                opening_done: true,
                ..default()
            })
            .add_message::<PlaybackEvent>()
            .add_message::<NarrativeProgress>()
            .add_message::<NarrativeEvent>()
            .add_message::<crate::rounds::RoundStarted>()
            .add_message::<crate::rounds::RoundEnding>()
            .add_message::<RobotPurgeReady>()
            .add_systems(Update, events);
        super::super::dialogue::tests::player(app.world_mut());
        app
    }

    #[test]
    fn narrative_uses_round_receipts_instead_of_the_already_incremented_round() {
        let mut app = app();
        app.world_mut().resource_mut::<Session>().round = 2;
        app.world_mut().write_message(crate::rounds::RoundEnding {
            session: crate::session::SessionId(1),
            round: 1,
        });
        app.update();
        let narrative = app.world().resource::<Narrative>();
        assert_eq!(
            narrative
                .queue
                .iter()
                .map(|line| line.alias.as_str())
                .collect::<Vec<_>>(),
            ["vox_plr_0_end_round_1_5_0", "vox_plr_0_end_round_1_6_1"]
        );
        app.world_mut().resource_mut::<Narrative>().queue.clear();
        app.world_mut().resource_mut::<Session>().round = 5;
        app.update();
        assert!(app.world().resource::<Narrative>().queue.is_empty());
        app.world_mut().write_message(crate::rounds::RoundStarted {
            session: crate::session::SessionId(1),
            round: 5,
        });
        app.update();
        assert_eq!(
            app.world().resource::<Narrative>().queue[0].alias,
            "vox_sam_sam_help_5_0"
        );
    }

    #[test]
    fn staff_conversation_starts_on_pickup_and_samantha_waits_for_her_intro() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<crate::crafting::Crafting>()
            .crafted
            .insert("elemental_staff_fire".into());
        app.update();
        assert!(app.world().resource::<Narrative>().queue.is_empty());
        app.world_mut().write_message(NarrativeEvent::StaffTaken(0));
        app.update();
        assert!(
            app.world()
                .resource::<Narrative>()
                .queue
                .iter()
                .all(|line| line.alias.starts_with("vox_plr_0_pickup_fire_"))
        );
        app.world_mut().resource_mut::<Narrative>().queue.clear();
        app.world_mut().write_message(NarrativeEvent::Samantha {
            alias: "vox_sam_fire_puz_solve_0".into(),
            source: SoundSource::World(Vec3::ZERO),
        });
        app.update();
        assert!(app.world().resource::<Narrative>().queue.is_empty());
        app.world_mut().resource_mut::<Narrative>().intro_done = true;
        app.world_mut().write_message(NarrativeEvent::Samantha {
            alias: "vox_sam_fire_puz_solve_0".into(),
            source: SoundSource::World(Vec3::ZERO),
        });
        app.update();
        assert_eq!(
            app.world().resource::<Narrative>().queue[0].alias,
            "vox_sam_fire_puz_solve_0"
        );
    }
}
fn reset(
    mut commands: Commands,
    mut state: ResMut<Narrative>,
    mut cues: ResMut<Messages<NarrativeEvent>>,
) {
    cues.clear();
    *state = default();
    state.opening = 10.;
    state.owner = Some(
        commands
            .spawn((SessionEntity, Name::new("Narrative conversation")))
            .id(),
    );
}
fn choose(
    state: &mut Narrative,
    game: &GameDefinition,
    session: &mut Session,
    prefix: &str,
) -> Option<String> {
    super::dialogue::choose(&mut state.pools, game, session, prefix)
}
fn push(
    state: &mut Narrative,
    game: &GameDefinition,
    alias: impl Into<String>,
    source: SoundSource,
    robot: Option<usize>,
    gap: f32,
) {
    let alias = alias.into();
    if resolve_alias(game, &alias).is_some() {
        state.queue.push_back(Line {
            alias,
            source,
            robot,
            gap,
            purge_ready: None,
            intro_end: false,
        });
    }
}
fn category(
    state: &mut Narrative,
    game: &GameDefinition,
    session: &mut Session,
    event: &str,
    source: SoundSource,
    robot: Option<usize>,
) -> bool {
    let event = match event {
        "recapture_generator_attacked" => "recapture_initiated",
        "buildable_built" => "build_zs_final",
        "pap_discovered" => "capture_zones",
        other => other,
    };
    let prefix = format!("vox_plr_{}_{}", session.character, event);
    if let Some(alias) = choose(state, game, session, &prefix) {
        push(state, game, alias, source, robot, 1.);
        true
    } else {
        false
    }
}
fn events(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<Narrative>,
    game: Res<GameDefinition>,
    mut session: ResMut<Session>,
    robots: Res<crate::robots::Robots>,
    raids: Res<crate::generators::GeneratorRaids>,
    crafting: Res<crate::crafting::Crafting>,
    secrets: Res<crate::side_secrets::Secrets>,
    players: Query<(Entity, &Transform), With<Player>>,
    mut playback: MessageReader<PlaybackEvent>,
    (mut progress, mut crafting_voice): (
        MessageReader<NarrativeProgress>,
        MessageReader<NarrativeEvent>,
    ),
    (mut rounds, mut endings, session_id): (
        MessageReader<crate::rounds::RoundStarted>,
        MessageReader<crate::rounds::RoundEnding>,
        Res<crate::session::SessionId>,
    ),
    sounds: Query<&PlayingSound>,
    mut reservation: ResMut<StoryReservation>,
    mut purge: MessageWriter<RobotPurgeReady>,
) {
    let Ok((player, pose)) = players.single() else {
        return;
    };
    let source = SoundSource::attached(player);
    let dt = time.delta_secs();
    state.clock += dt;
    for event in playback.read() {
        if state.active == Some(event.entity) {
            match event.status {
                PlaybackStatus::Completed => {
                    state.active = None;
                    if let Some(line) = state.queue.pop_front() {
                        state.wait = line.gap;
                        state.intro_done |= line.intro_end;
                    }
                }
                PlaybackStatus::Rejected | PlaybackStatus::Cancelled => {
                    state.active = None;
                    state.wait = 0.25;
                }
                PlaybackStatus::Accepted => {
                    if let Some(line) = state.queue.front_mut()
                        && let Some(robot) = line.purge_ready
                        && robots
                            .occupancy
                            .is_some_and(|o| o.player == player && o.robot == robot)
                    {
                        line.robot = None;
                        line.purge_ready = None;
                        purge.write(RobotPurgeReady { player, robot });
                    }
                }
            }
        }
    }
    let interior = robots.occupancy.map(|o| o.robot);
    if interior != state.robot {
        if state.queue.front().is_some_and(|line| line.robot.is_some())
            && let Some(active) = state.active.take()
        {
            commands.entity(active).try_despawn();
        }
        state.queue.retain(|line| line.robot.is_none());
        state.robot = interior;
        state.occupancy_seconds = 0.;
        state.purge_started = false;
    }
    if interior.is_some() {
        state.occupancy_seconds += dt;
    }
    if !state.opening_done && session.round >= 1 {
        reservation.0 = state.owner;
        state.opening = (state.opening - dt).max(0.);
        if state.opening == 0. {
            let queued = std::mem::take(&mut state.queue);
            let character = session.character;
            push(
                &mut state,
                &game,
                format!("vox_plr_{character}_game_start_0"),
                source,
                None,
                if character == 2 { 1. } else { 0. },
            );
            if character == 2 {
                push(
                    &mut state,
                    &game,
                    "vox_plr_2_game_start_1",
                    source,
                    None,
                    1.,
                );
            }
            state.queue.extend(queued);
            state.opening_done = true;
        }
    }
    for event in rounds.read().filter(|event| event.session == *session_id) {
        match event.round {
            5 => {
                push(&mut state, &game, "vox_sam_sam_help_5_0", source, None, 0.);
                if session.character != 2 {
                    category(
                        &mut state,
                        &game,
                        &mut session,
                        "hear_samantha_1",
                        source,
                        None,
                    );
                }
            }
            7 => {
                push(
                    &mut state,
                    &game,
                    "vox_sam_hear_samantha_3_0",
                    source,
                    None,
                    0.,
                );
                category(
                    &mut state,
                    &game,
                    &mut session,
                    "hear_samantha_3",
                    source,
                    None,
                );
                if let Some(line) = state.queue.back_mut() {
                    line.intro_end = true;
                }
            }
            _ => {}
        }
    }
    // rounds::advance increments Session.round before intermission. Use the
    // ending receipt so Round 1/2 conversations belong to the round just ended.
    for event in endings.read().filter(|event| event.session == *session_id) {
        let character = session.character;
        if event.round == 1 {
            match character {
                0 => {
                    push(
                        &mut state,
                        &game,
                        "vox_plr_0_end_round_1_5_0",
                        source,
                        None,
                        1.,
                    );
                    push(
                        &mut state,
                        &game,
                        "vox_plr_0_end_round_1_6_1",
                        source,
                        None,
                        1.,
                    );
                }
                1 => push(
                    &mut state,
                    &game,
                    "vox_plr_1_end_round_1_9_0",
                    source,
                    None,
                    0.,
                ),
                2 => push(
                    &mut state,
                    &game,
                    "vox_plr_2_end_round_1_7_0",
                    source,
                    None,
                    0.,
                ),
                _ => push(
                    &mut state,
                    &game,
                    "vox_plr_3_end_round_1_8_0",
                    source,
                    None,
                    0.,
                ),
            }
        }
        if event.round == 2 {
            match character {
                0 => push(
                    &mut state,
                    &game,
                    "vox_plr_0_end_round_2_1_0",
                    source,
                    None,
                    0.,
                ),
                1 => push(
                    &mut state,
                    &game,
                    "vox_plr_1_end_round_2_5_0",
                    source,
                    None,
                    0.,
                ),
                2 => {
                    push(
                        &mut state,
                        &game,
                        "vox_plr_2_end_round_2_2_0",
                        source,
                        None,
                        1.,
                    );
                    push(
                        &mut state,
                        &game,
                        "vox_plr_2_end_round_2_3_1",
                        source,
                        None,
                        1.,
                    );
                }
                _ => push(
                    &mut state,
                    &game,
                    "vox_plr_3_end_round_2_4_0",
                    source,
                    None,
                    0.,
                ),
            }
        }
    }
    let raid = raids
        .target
        .as_ref()
        .filter(|_| raids.under_attack)
        .map(|id| id.0.clone());
    if state.raid != raid {
        if raid.is_some() {
            category(
                &mut state,
                &game,
                &mut session,
                "recapture_generator_attacked",
                source,
                None,
            );
        }
        state.raid = raid;
    }
    if robots.entries > state.entries {
        state.entries = robots.entries;
        emit(
            &mut commands,
            "mus_event_location_robot",
            SoundSource::World(pose.translation),
        );
        category(
            &mut state,
            &game,
            &mut session,
            "enter_robot",
            source,
            interior,
        );
    }
    if robots.evacuations > state.evacuations {
        state.evacuations = robots.evacuations;
        category(&mut state, &game, &mut session, "exit_robot", source, None);
    }
    if let Some(robot) = interior
        && state.occupancy_seconds >= 15.
        && !state.purge_started
        && !secrets.logs_playing(robot)
        && state.queue.is_empty()
        && state.active.is_none()
    {
        let origin = SoundSource::World(Vec3::from_array(
            [
                [-171.704, 3.90525, 166.03345],
                [-158.08325, 3.90525, 166.03345],
                [-144.87425, 3.90525, 166.03345],
            ][robot],
        ));
        push(
            &mut state,
            &game,
            "vox_maxi_purge_robot_0",
            origin,
            Some(robot),
            0.,
        );
        category(
            &mut state,
            &game,
            &mut session,
            "purge_robot",
            source,
            Some(robot),
        );
        push(
            &mut state,
            &game,
            "vox_maxi_purge_countdown_0",
            origin,
            Some(robot),
            1.,
        );
        push(
            &mut state,
            &game,
            "vox_maxi_purge_now_0",
            origin,
            Some(robot),
            0.,
        );
        if let Some(line) = state.queue.back_mut() {
            line.purge_ready = Some(robot);
        }
        state.purge_started = true;
    }
    for event in crafting_voice.read() {
        match event {
            NarrativeEvent::Samantha { alias, source } => {
                if state.intro_done {
                    push(&mut state, &game, alias.clone(), *source, None, 0.);
                }
            }
            NarrativeEvent::DroneBuilt(origin) => {
                if state.queue.is_empty() {
                    state.wait = state.wait.max(1.);
                }
                for alias in ["vox_maxi_maxis_drone_1_0", "vox_maxi_maxis_drone_4_0"] {
                    push(&mut state, &game, alias, *origin, None, 1.);
                }
                if session.character == 2 {
                    push(
                        &mut state,
                        &game,
                        "vox_plr_2_maxis_drone_5_0",
                        source,
                        None,
                        0.,
                    );
                }
            }
            NarrativeEvent::StaffTaken(element) => {
                if reservation.0.is_none() && state.queue.is_empty() {
                    category(
                        &mut state,
                        &game,
                        &mut session,
                        ["pickup_fire", "pickup_ice", "pickup_wind", "pickup_light"][*element],
                        source,
                        None,
                    );
                }
                if !state.staffs_claimed.insert(*element) {
                    continue;
                }
                let count = crafting
                    .crafted
                    .iter()
                    .filter(|id| id.starts_with("elemental_staff_"))
                    .count();
                if count == 4 {
                    let character = session.character;
                    for alias in [
                        "vox_sam_4th_staff_crafted_0".to_owned(),
                        format!("vox_plr_{character}_4th_staff_crafted_0"),
                        "vox_sam_4th_staff_crafted_1".into(),
                        format!("vox_plr_{character}_4th_staff_crafted_1"),
                        "vox_sam_generic_encourage_6".into(),
                    ] {
                        if state.intro_done || !alias.starts_with("vox_sam_") {
                            push(&mut state, &game, alias, source, None, 0.);
                        }
                    }
                } else if state.intro_done && (1..=3).contains(&count) {
                    let ordinal = ["1st", "2nd", "3rd"][count - 1];
                    push(
                        &mut state,
                        &game,
                        format!("vox_sam_{ordinal}_staff_crafted_0"),
                        source,
                        None,
                        0.,
                    );
                }
            }
        }
    }
    for event in progress.read() {
        if event.player == player
            && state.intro_done
            && state.clock >= state.encourage_after
            && state.queue.is_empty()
        {
            if let Some(alias) =
                choose(&mut state, &game, &mut session, "vox_sam_generic_encourage")
            {
                push(&mut state, &game, alias, source, None, 0.);
                category(
                    &mut state,
                    &game,
                    &mut session,
                    "generic_encourage",
                    source,
                    None,
                );
                state.encourage_after = state.clock + 300. + (session.random() % 300) as f32;
            }
        }
    }
    state.wait = (state.wait - dt).max(0.);
    if !state.queue.is_empty() && reservation.0.is_none() {
        reservation.0 = state.owner;
    }
    if state.active.is_none()
        && state.wait == 0.
        && state.opening_done
        && !sounds.iter().any(|sound| {
            sound.channel == Channel::Dialogue
                && matches!(sound.source, SoundSource::Entity { entity, .. } if entity == player)
        })
        && let Some(line) = state.queue.front()
    {
        if reservation.0.is_none() || reservation.0 == state.owner {
            reservation.0 = state.owner;
            if state.owner.is_some() {
                state.active = Some(emit(&mut commands, line.alias.clone(), line.source));
            }
        }
    }
    if state.opening_done
        && state.queue.is_empty()
        && state.active.is_none()
        && state.wait == 0.
        && reservation.0 == state.owner
    {
        reservation.0 = None;
    }
}
