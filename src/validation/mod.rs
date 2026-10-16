//! Opt-in development BRP. The game owns simulation; this module supplies inputs and observations.
mod actions;
pub(crate) mod cinematic;
mod probe;
mod protocol;
mod state;
#[cfg(test)]
pub(crate) mod tests;

use crate::{GameState, gameplay::Session, session::SessionId};
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemotePlugin, http::RemoteHttpPlugin},
};
use protocol::{Command, Phase, Receipt, Request};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::PathBuf,
    time::{Duration, Instant},
};

#[derive(Resource)]
pub(crate) struct Controller {
    seed: u64,
    report: PathBuf,
    session: Option<u64>,
    heartbeat: Instant,
    paused: bool,
    aborted: bool,
    failure: Option<String>,
    controls: VecDeque<Request>,
    queue: VecDeque<Request>,
    active: Option<(Request, u32)>,
    completed: Vec<(u64, u64)>,
    receipts: BTreeMap<(u64, u64), Receipt>,
    tick: u64,
    simulation_seconds: f64,
    step: Option<(u64, u64)>,
    step_frame: bool,
    suspended: Option<crate::input::SuspendedActions>,
    god: bool,
    label: String,
    evidence: state::Evidence,
}
impl Controller {
    fn new(seed: u64, report: PathBuf) -> Self {
        Self {
            seed,
            report,
            session: None,
            heartbeat: Instant::now(),
            paused: false,
            aborted: false,
            failure: None,
            controls: default(),
            queue: default(),
            active: None,
            completed: default(),
            receipts: default(),
            tick: 0,
            simulation_seconds: 0.,
            step: None,
            step_frame: false,
            suspended: None,
            god: false,
            label: "Waiting for runner".into(),
            evidence: default(),
        }
    }
    pub(crate) fn owns_input(&self) -> bool {
        self.session.is_some()
    }
    pub(crate) fn stepping(&self) -> bool {
        self.step_frame
    }
    pub(crate) fn request_pause_toggle(&mut self) {
        if self.failure.is_none() && !self.aborted {
            let action = if self.paused {
                Command::Resume
            } else {
                Command::Pause
            };
            self.controls.push_back(Request {
                session: self.session.unwrap(),
                id: 0,
                action,
                label: String::new(),
            });
        }
    }
    pub(crate) fn request_abort(&mut self) {
        if let Some(session) = self.session {
            self.controls.push_back(Request {
                session,
                id: 0,
                action: Command::Abort,
                label: String::new(),
            });
        }
    }
}

pub(crate) fn install(app: &mut App) {
    let Some(report) = std::env::var_os("ORIGINS_VALIDATOR_REPORT") else {
        return;
    };
    let report = PathBuf::from(report);
    assert!(
        report.is_absolute(),
        "ORIGINS_VALIDATOR_REPORT must be absolute"
    );
    std::fs::create_dir_all(&report).expect("Create validator report directory");
    let seed = std::env::var("ORIGINS_VALIDATOR_SEED")
        .map(|s| {
            s.parse()
                .expect("Validator seed must be an unsigned integer")
        })
        .unwrap_or(0xB02);
    let port = std::env::var("ORIGINS_VALIDATOR_PORT")
        .map(|s| s.parse().expect("Validator port must be a u16"))
        .unwrap_or(15702);
    let speed = std::env::var("ORIGINS_VALIDATOR_SPEED")
        .map(|s| {
            s.parse::<u32>()
                .expect("Validator speed must be an integer from 1 to 10")
        })
        .unwrap_or(1);
    assert!(
        (1..=10).contains(&speed),
        "Validator speed must be from 1 to 10"
    );
    app.world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_relative_speed_f64(f64::from(speed));
    app.world_mut()
        .resource_mut::<avian3d::prelude::SubstepCount>()
        .0 *= speed;
    install_controller(app, Controller::new(seed, report));
    app.add_plugins((
        RemotePlugin::default()
            .with_method_main("origins/validator/state", remote_state)
            .with_method_main("origins/validator/jump_probes", probe::remote)
            .with_method_main("origins/validator/command", remote_command)
            .with_method_main("origins/validator/input_batch", remote_input_batch)
            .with_method_main("origins/validator/receipt", remote_receipt)
            .with_method_main("origins/validator/heartbeat", remote_heartbeat),
        RemoteHttpPlugin::default()
            .with_address(std::net::Ipv4Addr::LOCALHOST)
            .with_port(port),
    ));
    // Keep ordinary read-only BRP inspection; gameplay changes go through the input owner.
    let old = app
        .world_mut()
        .remove_resource::<bevy::remote::RemoteMethods>()
        .unwrap();
    let mut methods = bevy::remote::RemoteMethods::new();
    for name in old.methods().into_iter().filter(|name| {
        name.starts_with("origins/validator/")
            || matches!(
                name.as_str(),
                "rpc.discover"
                    | "world.query"
                    | "world.get_components"
                    | "world.list_components"
                    | "world.get_resources"
                    | "world.list_resources"
                    | "registry.schema"
            )
    }) {
        methods.insert(name.clone(), *old.get(&name).unwrap());
    }
    app.insert_resource(methods);
}

fn install_controller(app: &mut App, controller: Controller) {
    app.insert_resource(controller)
        .add_systems(First, control.before(bevy::time::TimeSystems))
        .add_systems(First, step_time.after(bevy::time::TimeSystems))
        .add_systems(
            PreUpdate,
            focus
                .after(bevy::input::InputSystems)
                .before(crate::input::collect),
        )
        .add_systems(
            FixedPreUpdate,
            (tick_input, crate::player::look.run_if(scripted_look))
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            FixedUpdate,
            actions::protect_player
                .before(crate::damage::resolve)
                .in_set(crate::gameplay::GameStep::Damage),
        )
        .add_systems(
            FixedPostUpdate,
            state::observe.run_if(in_state(GameState::Playing)),
        )
        .add_systems(Last, finish_frame)
        .add_systems(Update, state::overlay);
}
pub(crate) fn owns_input(world: &World) -> bool {
    world
        .get_resource::<Controller>()
        .is_some_and(Controller::owns_input)
}
pub(crate) fn physical_look(controller: Option<Res<Controller>>) -> bool {
    !controller.as_ref().is_some_and(|c| c.owns_input())
}
fn scripted_look(controller: Res<Controller>) -> bool {
    controller.owns_input() && (!controller.paused || controller.step_frame)
}

pub(crate) fn initialize(world: &mut World, session: &mut Session) {
    world.remove_resource::<cinematic::Playback>();
    let id = world.resource::<SessionId>().0;
    let Some(mut c) = world.get_resource_mut::<Controller>() else {
        return;
    };
    let previous = c.session;
    let start = c
        .controls
        .iter()
        .chain(c.queue.iter())
        .find(|r| matches!(r.action, Command::Start))
        .map(|r| (r.session, r.id));
    // Start is already popped by control; retain its receipt across the ordinary lifecycle transition.
    let start = start.or_else(|| {
        c.receipts
            .iter()
            .rev()
            .find(|(_, r)| r.phase == Phase::Accepted && matches!(r.request.action, Command::Start))
            .map(|(key, _)| *key)
    });
    for receipt in c
        .receipts
        .values_mut()
        .filter(|r| r.phase == Phase::Accepted || r.phase == Phase::Executed)
    {
        if !matches!(receipt.request.action, Command::Start) {
            receipt.phase = Phase::Cancelled;
            receipt.error = Some("Session changed".into());
        }
    }
    c.queue.clear();
    c.active = None;
    c.completed.clear();
    c.controls.clear();
    c.step = None;
    c.step_frame = false;
    c.suspended = None;
    c.session = Some(id);
    c.tick = 0;
    c.simulation_seconds = 0.;
    c.evidence = default();
    c.paused = false;
    c.aborted = false;
    c.failure = None;
    c.god = false;
    c.heartbeat = Instant::now();
    session.seed = c.seed;
    if let Some(key) = start
        && let Some(receipt) = c.receipts.get_mut(&key)
    {
        receipt.resulting_session = Some(id);
    }
    info!("Validator session {previous:?} -> {id}, seed {}", c.seed);
}

fn error(message: impl Into<String>) -> BrpError {
    BrpError {
        code: -32602,
        message: message.into(),
        data: None,
    }
}
fn remote_state(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    Ok(state::snapshot(world))
}
fn remote_receipt(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let params = params.ok_or_else(|| error("Missing command identity"))?;
    let session = params["session"]
        .as_u64()
        .ok_or_else(|| error("Missing session"))?;
    let id = params["id"].as_u64().ok_or_else(|| error("Missing id"))?;
    let receipt = world
        .resource::<Controller>()
        .receipts
        .get(&(session, id))
        .ok_or_else(|| error("Unknown command"))?;
    Ok(serde_json::to_value(receipt).unwrap())
}

fn remote_heartbeat(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let session = params
        .as_ref()
        .and_then(|p| p.get("session"))
        .and_then(Value::as_u64)
        .ok_or_else(|| error("Missing session"))?;
    let current = world.resource::<SessionId>().0;
    if current != session {
        return Err(error("Stale heartbeat session"));
    }
    world.resource_mut::<Controller>().heartbeat = Instant::now();
    Ok(json!({"session":current}))
}
fn remote_command(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Request = serde_json::from_value(params.ok_or_else(|| error("Missing command"))?)
        .map_err(|e| error(e.to_string()))?;
    submit(world, request).map_err(error)
}

fn submit(world: &mut World, request: Request) -> Result<Value, String> {
    if let Some(receipt) = validate_request(world, &request)? {
        return Ok(receipt);
    }
    let key = (request.session, request.id);
    let c = world.resource::<Controller>();
    let receipt = Receipt {
        request: request.clone(),
        phase: Phase::Accepted,
        accepted_tick: c.tick,
        executed_tick: None,
        resulting_session: None,
        error: None,
        after: Value::Null,
    };
    let result = serde_json::to_value(&receipt).unwrap();
    let mut c = world.resource_mut::<Controller>();
    c.receipts.insert(key, receipt);
    if matches!(request.action, Command::Input { .. }) {
        c.queue.push_back(request);
    } else {
        c.controls.push_back(request);
    }
    Ok(result)
}

fn remote_input_batch(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let requests: Vec<Request> =
        serde_json::from_value(params.ok_or_else(|| error("Missing inputs"))?)
            .map_err(|e| error(e.to_string()))?;
    if !(1..=64).contains(&requests.len()) {
        return Err(error("Input batch must contain 1..64 commands"));
    }
    let mut ids = BTreeSet::new();
    for request in &requests {
        if !matches!(request.action, Command::Input { .. }) || !ids.insert(request.id) {
            return Err(error(
                "Input batches require distinct IDs and ordinary input commands",
            ));
        }
        validate_request(world, request).map_err(error)?;
    }
    requests
        .into_iter()
        .map(|request| submit(world, request))
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
        .map_err(error)
}

fn validate_request(world: &World, request: &Request) -> Result<Option<Value>, String> {
    let key = (request.session, request.id);
    if request.id == 0 {
        return Err("Command ID zero is reserved for local controls".into());
    }
    if let Some(receipt) = world.resource::<Controller>().receipts.get(&key) {
        if serde_json::to_value(&receipt.request).unwrap()
            != serde_json::to_value(&request).unwrap()
        {
            return Err("Command ID reused with different arguments".into());
        }
        return Ok(Some(serde_json::to_value(receipt).unwrap()));
    }
    if world.resource::<SessionId>().0 != request.session {
        return Err("Stale command session".into());
    }
    request.action.validate()?;
    if !world.resource::<Controller>().owns_input()
        && matches!(
            request.action,
            Command::Input { .. } | Command::Assist { .. } | Command::Teleport { .. }
        )
    {
        return Err("Start a validator session before submitting gameplay actions".into());
    }
    let c = world.resource::<Controller>();
    if (c.failure.is_some() || c.aborted)
        && !matches!(
            request.action,
            Command::Resume | Command::Abort | Command::Quit | Command::Capture { .. }
        )
    {
        return Err("Run has failed or was aborted".into());
    }
    Ok(None)
}

fn control(world: &mut World) {
    if !owns_input(world) && world.resource::<Controller>().controls.is_empty() {
        return;
    }
    let expired = {
        let c = world.resource::<Controller>();
        c.owns_input()
            && !c.aborted
            && c.failure.is_none()
            && c.heartbeat.elapsed() >= Duration::from_secs(5)
    };
    if expired {
        fail(world, "Runner heartbeat expired".into());
    }
    while let Some(request) = world.resource_mut::<Controller>().controls.pop_front() {
        if request.session != world.resource::<SessionId>().0 {
            reject(world, &request, "Session changed before execution".into());
            continue;
        }
        if !request.label.is_empty() {
            world.resource_mut::<Controller>().label = request.label.clone();
        }
        let result = actions::execute(world, &request);
        match result {
            Ok(true) => executed(world, (request.session, request.id)),
            Ok(false) => {}
            Err(reason) => {
                reject(world, &request, reason.clone());
                if owns_input(world) {
                    fail(world, reason);
                }
            }
        }
    }
}

fn step_time(world: &mut World) {
    if !world.resource::<Controller>().step_frame {
        return;
    }
    let duration = world.resource::<Time<Fixed>>().timestep();
    world.resource_mut::<Time<Virtual>>().advance_by(duration);
    *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
}

fn focus(world: &mut World) {
    if !owns_input(world)
        || world.resource::<Controller>().paused
        || *world.resource::<State<GameState>>().get() != GameState::Playing
    {
        return;
    }
    let focused = world
        .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>()
        .single(world)
        .is_ok_and(|w| w.focused);
    if !focused {
        pause(world);
        world.resource_mut::<Controller>().label = "Focus lost: paused".into();
    }
}

fn tick_input(world: &mut World) {
    let mut arrival_expired = false;
    world.resource_scope(|world, mut c: Mut<Controller>| {
        if !c.owns_input() || c.paused && !c.step_frame {
            return;
        }
        c.tick += 1;
        c.simulation_seconds += 1. / 60.;
        if c.active.as_ref().is_some_and(|(_, n)| *n == 0) {
            let (request, _) = c.active.take().unwrap();
            c.completed.push((request.session, request.id));
        }
        if c.active.is_none() {
            c.active = c.queue.pop_front().map(|request| {
                let Command::Input { ticks, .. } = request.action else {
                    unreachable!()
                };
                (request, ticks)
            });
            if let Some((request, _)) = &c.active
                && !request.label.is_empty()
            {
                c.label = request.label.clone();
            }
        }
        let Some((request, remaining)) = &mut c.active else {
            crate::input::scripted_tick(world, &[], Vec2::ZERO);
            return;
        };
        let Command::Input {
            held,
            look,
            aim,
            aim_entity,
            aim_offset,
            arrival_point,
            arrival_entity,
            arrival_radius,
            ..
        } = &request.action
        else {
            unreachable!()
        };
        let target = aim.map(Vec3::from_array).or_else(|| {
            aim_entity.and_then(Entity::try_from_bits).and_then(|e| {
                world.get::<Transform>(e).map(|t| {
                    let local = aim_offset
                        .map(Vec3::from_array)
                        .or_else(|| {
                            world
                                .get::<crate::hit_regions::Hitboxes>(e)
                                .and_then(state::aim_point)
                        })
                        .unwrap_or(Vec3::ZERO);
                    t.transform_point(local)
                })
            })
        });
        if arrival_radius
            .zip(arrival_point.map(Vec3::from_array).or(target))
            .is_some_and(|(radius, target)| {
                let position = if let Some(entity) = arrival_entity {
                    Entity::try_from_bits(*entity)
                        .and_then(|entity| world.get::<Transform>(entity))
                        .map(|pose| pose.translation)
                } else {
                    world
                        .query_filtered::<&Transform, With<crate::player::Player>>()
                        .single(world)
                        .ok()
                        .map(|pose| pose.translation)
                };
                position.is_some_and(|position| position.xz().distance(target.xz()) <= radius)
            })
        {
            crate::input::scripted_tick(world, &[], Vec2::ZERO);
            *remaining = 0;
            return;
        }
        if arrival_radius.is_some() && *remaining == 1 {
            arrival_expired = true;
            return;
        }
        let look = target.map_or_else(
            || Vec2::from_array(*look),
            |target| actions::look_delta(world, target),
        );
        crate::input::scripted_tick(world, held, look);
        *remaining -= 1;
    });
    if arrival_expired {
        fail(world, "Input arrival waypoint deadline expired".into());
    }
}

fn finish_frame(world: &mut World) {
    if !owns_input(world) {
        return;
    }
    let state = *world.resource::<State<GameState>>().get();
    if state == GameState::Playing
        && (!world.resource::<Controller>().paused || world.resource::<Controller>().step_frame)
    {
        // A completed input batch is acknowledged only after PostUpdate physics.
        let completed = world
            .resource::<Controller>()
            .active
            .as_ref()
            .filter(|(_, n)| *n == 0)
            .map(|(r, _)| (r.session, r.id));
        if let Some(key) = completed {
            world.resource_mut::<Controller>().active = None;
            executed(world, key);
        }
    }
    let completed = std::mem::take(&mut world.resource_mut::<Controller>().completed);
    for key in completed {
        executed(world, key);
    }
    let starts: Vec<_> = world
        .resource::<Controller>()
        .receipts
        .iter()
        .filter(|(_, r)| {
            r.phase == Phase::Accepted
                && r.resulting_session.is_some()
                && matches!(r.request.action, Command::Start)
                && state == GameState::Playing
        })
        .map(|(key, _)| *key)
        .collect();
    for key in starts {
        executed(world, key);
    }
    if world.resource::<Controller>().step_frame {
        let key = world.resource_mut::<Controller>().step.take();
        world.resource_mut::<Controller>().step_frame = false;
        pause(world);
        if let Some(key) = key {
            executed(world, key);
        }
    }
}

fn executed(world: &mut World, key: (u64, u64)) {
    let mut c = world.resource_mut::<Controller>();
    let tick = c.tick;
    if let Some(receipt) = c.receipts.get_mut(&key) {
        if receipt.phase != Phase::Accepted {
            return;
        }
        receipt.phase = Phase::Executed;
        receipt.executed_tick = Some(tick);
    }
}
fn reject(world: &mut World, request: &Request, reason: String) {
    if let Some(receipt) = world
        .resource_mut::<Controller>()
        .receipts
        .get_mut(&(request.session, request.id))
    {
        receipt.phase = Phase::Rejected;
        receipt.error = Some(reason);
    }
}
fn pause(world: &mut World) {
    if world.resource::<Controller>().paused {
        return;
    }
    let saved = crate::input::suspend_script(world);
    if let Some(mut movie) = world.get_resource_mut::<cinematic::Playback>() {
        movie.pause(true);
    }
    {
        let mut c = world.resource_mut::<Controller>();
        c.paused = true;
        c.suspended = Some(saved);
    }
    world.resource_mut::<Time<Virtual>>().pause();
    // Clear this frame's delta too: pausing after TimeSystems must not advance physics.
    world
        .resource_mut::<Time<Virtual>>()
        .advance_by(Duration::ZERO);
    *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
    let overstep = world.resource::<Time<Fixed>>().overstep();
    world
        .resource_mut::<Time<Fixed>>()
        .discard_overstep(overstep);
    if matches!(
        world.resource::<State<GameState>>().get(),
        GameState::Playing
    ) {
        world
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Paused);
    }
}
pub(crate) fn fail(world: &mut World, reason: String) {
    if world.resource::<Controller>().failure.is_some() {
        return;
    }
    pause(world);
    {
        let mut c = world.resource_mut::<Controller>();
        c.failure = Some(reason.clone());
        c.label = format!("FAILED: {reason}");
        c.controls.clear();
        c.queue.clear();
        c.active = None;
        c.completed.clear();
        c.step = None;
        c.step_frame = false;
        for r in c
            .receipts
            .values_mut()
            .filter(|r| r.phase == Phase::Accepted)
        {
            r.phase = Phase::Failed;
            r.error = Some(reason.clone());
        }
    }
    let snapshot = state::snapshot(world);
    let path = world
        .resource::<Controller>()
        .report
        .join("game-failure.json");
    if let Err(error) = std::fs::write(&path, serde_json::to_vec_pretty(&snapshot).unwrap()) {
        error!("Write validator failure {}: {error}", path.display());
    }
    warn!("Validator paused: {reason}");
}
