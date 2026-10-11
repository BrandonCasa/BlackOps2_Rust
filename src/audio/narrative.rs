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
#[derive(Message, Clone, Copy)]
pub(crate) struct RobotPurgeReady {
    pub player: Entity,
    pub robot: usize,
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
    round: u32,
    intermission: bool,
    entries: u32,
    evacuations: u32,
    raid: Option<String>,
    crafted: BTreeSet<String>,
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
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Narrative>()
        .add_message::<NarrativeProgress>()
        .add_message::<RobotPurgeReady>()
        .add_systems(OnEnter(GameState::PreparingShaders), reset)
        .add_systems(
            FixedUpdate,
            events
                .after(GameStep::Rounds)
                .run_if(in_state(GameState::Playing)),
        );
}
fn reset(mut commands: Commands, mut state: ResMut<Narrative>) {
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
    let pool = state.pools.entry(prefix.into()).or_default();
    if pool.is_empty() {
        pool.extend(
            game.audio
                .aliases
                .keys()
                .filter(|name| {
                    !(prefix == "vox_sam_generic_encourage" && name.ends_with("_6"))
                        && name.strip_prefix(prefix).is_some_and(|tail| {
                            tail.starts_with('_')
                                && tail[1..].chars().all(|c| c.is_ascii_digit() || c == '_')
                        })
                })
                .cloned(),
        );
    }
    if pool.is_empty() {
        return None;
    }
    let index = session.random() as usize % pool.len();
    Some(pool.remove(index))
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
    mut progress: MessageReader<NarrativeProgress>,
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
                1.,
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
    if state.round != session.round {
        state.round = session.round;
        match session.round {
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
            2..=4 => {
                let round = session.round;
                category(
                    &mut state,
                    &game,
                    &mut session,
                    &format!("during_round_{round}"),
                    source,
                    None,
                );
            }
            _ => {}
        }
    }
    let intermission = session.intermission.is_some();
    if intermission && !state.intermission {
        let character = session.character;
        if session.round == 1 {
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
                    1.,
                ),
                2 => push(
                    &mut state,
                    &game,
                    "vox_plr_2_end_round_1_7_0",
                    source,
                    None,
                    1.,
                ),
                _ => push(
                    &mut state,
                    &game,
                    "vox_plr_3_end_round_1_8_0",
                    source,
                    None,
                    1.,
                ),
            }
        }
        if session.round == 2 {
            match character {
                0 => push(
                    &mut state,
                    &game,
                    "vox_plr_0_end_round_2_1_0",
                    source,
                    None,
                    1.,
                ),
                1 => push(
                    &mut state,
                    &game,
                    "vox_plr_1_end_round_2_5_0",
                    source,
                    None,
                    1.,
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
                    1.,
                ),
            }
        }
    }
    state.intermission = intermission;
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
    for recipe in &crafting.crafted {
        if recipe.starts_with("elemental_staff_") && !state.intro_done {
            continue;
        }
        if state.crafted.insert(recipe.clone()) {
            if recipe == "equip_dieseldrone_zm" {
                push(
                    &mut state,
                    &game,
                    "vox_maxi_maxis_drone_1_0",
                    source,
                    None,
                    0.,
                );
                push(
                    &mut state,
                    &game,
                    "vox_maxi_maxis_drone_4_0",
                    source,
                    None,
                    0.,
                );
            } else if recipe == "tomb_shield_zm" {
                category(
                    &mut state,
                    &game,
                    &mut session,
                    "build_zs_final",
                    source,
                    None,
                );
            } else if recipe.starts_with("elemental_staff_") {
                let count = state
                    .crafted
                    .iter()
                    .filter(|id| id.starts_with("elemental_staff_"))
                    .count();
                if count < 4 {
                    push(
                        &mut state,
                        &game,
                        format!(
                            "vox_sam_{count}{}",
                            if count == 1 {
                                "st_staff_crafted_0"
                            } else if count == 2 {
                                "nd_staff_crafted_0"
                            } else {
                                "rd_staff_crafted_0"
                            }
                        ),
                        source,
                        None,
                        0.,
                    );
                } else {
                    let character = session.character;
                    for alias in [
                        "vox_sam_4th_staff_crafted_0".to_owned(),
                        format!("vox_plr_{character}_4th_staff_crafted_0"),
                        "vox_sam_4th_staff_crafted_1".into(),
                        format!("vox_plr_{character}_4th_staff_crafted_1"),
                        "vox_sam_generic_encourage_6".into(),
                    ] {
                        push(&mut state, &game, alias, source, None, 0.);
                    }
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
    if state.active.is_none()
        && state.wait == 0.
        && state.opening_done
        && let Some(line) = state.queue.front()
    {
        if reservation.0.is_none() || reservation.0 == state.owner {
            reservation.0 = state.owner;
            if let Some(owner) = state.owner {
                state.active = Some(emit_owned(
                    &mut commands,
                    line.alias.clone(),
                    line.source,
                    owner,
                ));
            }
        }
    }
    if state.opening_done
        && state.queue.is_empty()
        && state.active.is_none()
        && reservation.0 == state.owner
    {
        reservation.0 = None;
    }
}
