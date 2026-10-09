use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{Session, SessionEntity, Zombie},
    input::RestartGame,
    life::Vitality,
    player::Player,
    weapons::WeaponInventory,
};
use bevy::{
    input::{
        ButtonState, InputSystems,
        keyboard::{Key, KeyboardInput, NativeKey, keyboard_input_system},
        mouse::{
            MouseButtonInput, accumulate_mouse_motion_system, accumulate_mouse_scroll_system,
            mouse_button_input_system,
        },
    },
    prelude::*,
    remote::{BrpError, BrpResult, RemotePlugin, http::RemoteHttpPlugin},
    window::PrimaryWindow,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeMap, net::Ipv4Addr};

#[derive(Resource, Default)]
struct RemoteState {
    tick: u64,
    epoch: u64,
    reset: bool,
    queue: BTreeMap<u64, Vec<Edge>>,
    held: BTreeMap<String, Edge>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Edge {
    tick: u64,
    key: String,
    pressed: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    events: Vec<Edge>,
}
#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Actor {
    position: [f32; 3],
    health: u32,
    defender: Option<String>,
}
impl Default for Actor {
    fn default() -> Self {
        Self {
            position: [0.; 3],
            health: 100,
            defender: None,
        }
    }
}
#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Setup {
    points: Option<u32>,
    position: Option<[f32; 3]>,
    yaw: Option<f32>,
    pitch: Option<f32>,
    weapons: Option<Vec<String>>,
    clear_enemies: bool,
    pending: Option<u32>,
    pause_spawning_seconds: Option<f32>,
    enemies: Vec<Actor>,
    blast: Option<RemoteBlast>,
    owned_perks: Option<Vec<String>>,
    round_event: Option<RemoteRound>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteRound {
    session_id: u64,
    round: u32,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemoteBlast {
    session_id: u64,
    center: [f32; 3],
    damage: u32,
    radius: f32,
}
pub(super) fn install(app: &mut App, port: u16) {
    app.init_resource::<RemoteState>()
        .configure_sets(PreUpdate, InputSystems.run_if(|| false))
        .add_plugins((
            RemotePlugin::default()
                .with_method_main("origins/schema", schema)
                .with_method_main("origins/state", snapshot)
                .with_method_main("origins/reset", reset)
                .with_method_main("origins/input", input)
                .with_method_main("origins/release", release)
                .with_method_main("origins/setup", setup)
                .with_method_main("origins/quit", quit),
            RemoteHttpPlugin::default()
                .with_address(Ipv4Addr::LOCALHOST)
                .with_port(port),
        ))
        .add_systems(
            FixedPreUpdate,
            (
                scheduled,
                (
                    keyboard_input_system,
                    mouse_button_input_system,
                    accumulate_mouse_motion_system,
                    accumulate_mouse_scroll_system,
                ),
                crate::input::collect,
                crate::input::pause_and_restart,
            )
                .chain(),
        );
}
fn error(message: impl Into<String>) -> BrpError {
    BrpError {
        code: -32602,
        message: message.into(),
        data: None,
    }
}
fn parse<T: serde::de::DeserializeOwned>(value: Option<Value>) -> Result<T, BrpError> {
    serde_json::from_value(value.unwrap_or_else(|| json!({}))).map_err(|e| error(e.to_string()))
}
fn window(world: &mut World) -> Result<Entity, BrpError> {
    world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .map_err(|_| error("Primary window unavailable"))
}
fn mouse(key: &str) -> Option<MouseButton> {
    match key {
        "MouseLeft" => Some(MouseButton::Left),
        "MouseRight" => Some(MouseButton::Right),
        "MouseMiddle" => Some(MouseButton::Middle),
        _ => None,
    }
}
fn key_code(key: &str) -> Result<KeyCode, BrpError> {
    serde_json::from_value(json!(key)).map_err(|_| error(format!("Unknown physical key: {key}")))
}
fn send(world: &mut World, edge: &Edge) -> Result<(), BrpError> {
    let window = window(world)?;
    let state = if edge.pressed {
        ButtonState::Pressed
    } else {
        ButtonState::Released
    };
    if let Some(button) = mouse(&edge.key) {
        world.write_message(MouseButtonInput {
            button,
            state,
            window,
        });
    } else {
        world.write_message(KeyboardInput {
            key_code: key_code(&edge.key)?,
            logical_key: Key::Unidentified(NativeKey::Unidentified),
            state,
            text: None,
            repeat: false,
            window,
        });
    }
    Ok(())
}
fn scheduled(world: &mut World) {
    world.resource_scope(|world, mut remote: Mut<RemoteState>| {
        remote.tick += 1;
        let tick = remote.tick;
        if remote.reset && *world.resource::<State<GameState>>().get() == GameState::GameOver {
            world.write_message(RestartGame);
            remote.reset = false;
        }
        if let Some(edges) = remote.queue.remove(&tick) {
            for edge in edges {
                if send(world, &edge).is_ok() {
                    if edge.pressed {
                        remote.held.insert(edge.key.clone(), edge);
                    } else {
                        remote.held.remove(&edge.key);
                    }
                }
            }
        }
    });
}
fn schema(_: In<Option<Value>>, port: Res<bevy::remote::http::HostPort>) -> BrpResult {
    Ok(json!({
        "version":1,"transport":"Bevy Remote Protocol","tick_schedule":"FixedPreUpdate",
        "bevy_version":"0.20.0-rc.2",
        "endpoints":{"main":format!("http://127.0.0.1:{}",port.0),"render":"http://127.0.0.1:15703"},
        "discovery":{"methods":"rpc.discover","schemas":"registry.schema"},
        "methods":{
            "origins/state":{"params":{},"result":"fixed_tick, epoch, session_id, state, queued_inputs, held_inputs, session, player, enemies, generators, population, round_evidence, equipment, drops, audio, controls, focus"},
            "origins/reset":{"params":{},"effect":"Release inputs, transition to GameOver, send normal RestartGame"},
            "origins/input":{"params":{"events":[{"tick":"relative integer 1..3600","key":"Bevy KeyCode name or MouseLeft/MouseRight/MouseMiddle","pressed":"boolean"}]},"max_events":512,"effect":"Queue native input messages; automatic release after final scheduled tick"},
            "origins/release":{"params":{},"effect":"Cancel queued input and release remotely held buttons"},
            "origins/quit":{"params":{},"effect":"Release remote inputs and emit normal AppExit"},
            "origins/setup":{"params":{"points":"optional u32","position":"optional [x,y,z] player center","yaw":"optional radians","pitch":"optional radians","weapons":"optional 1..2 native weapon IDs","owned_perks":"optional 0..8 unique native perk IDs","blast":{"session_id":"u64","center":"[x,y,z]","damage":"0..1000000","radius":"0.01..100"},"round_event":{"session_id":"u64","round":"1..10000"},"clear_enemies":"optional bool","pending":"optional 0..10000","pause_spawning_seconds":"optional 0..300","enemies":[{"position":"[x,y,z]","health":"1..1000000, default100","defender":"optional generator ID"}]},"max_enemies":64,"effect":"Direct scenario setup only; not evidence of gameplay"}
        }
    }))
}
fn input(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Inputs = parse(value)?;
    window(world)?;
    if request.events.is_empty() || request.events.len() > 512 {
        return Err(error("Expected 1..512 events"));
    }
    for event in &request.events {
        if !(1..=3600).contains(&event.tick) {
            return Err(error("Tick outside 1..3600"));
        }
        if mouse(&event.key).is_none() {
            key_code(&event.key)?;
        }
    }
    let mut remote = world.resource_mut::<RemoteState>();
    if !remote.queue.is_empty() || !remote.held.is_empty() {
        return Err(error("Release or finish the existing input batch first"));
    }
    let start = remote.tick;
    let last = request.events.iter().map(|e| e.tick).max().unwrap();
    let mut keys = BTreeMap::new();
    for edge in request.events {
        keys.insert(edge.key.clone(), edge.clone());
        remote
            .queue
            .entry(start + edge.tick)
            .or_default()
            .push(edge);
    }
    for (_, mut edge) in keys {
        edge.pressed = false;
        remote.queue.entry(start + last + 1).or_default().push(edge);
    }
    Ok(json!({"start_tick":start,"finish_tick":start+last+1}))
}
fn release(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    let held = {
        let mut remote = world.resource_mut::<RemoteState>();
        remote.queue.clear();
        std::mem::take(&mut remote.held)
    };
    for (_, mut edge) in held {
        edge.pressed = false;
        send(world, &edge)?;
    }
    Ok(json!({"released":true}))
}
fn reset(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    if !matches!(
        *world.resource::<State<GameState>>().get(),
        GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return Err(error("Session is not ready"));
    }
    release(In(None), world)?;
    world
        .resource_mut::<NextState<GameState>>()
        .set(GameState::GameOver);
    let mut remote = world.resource_mut::<RemoteState>();
    remote.epoch += 1;
    remote.reset = true;
    Ok(json!({"epoch":remote.epoch,"reset_requested":true}))
}
fn quit(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    release(In(None), world)?;
    world.write_message(AppExit::Success);
    Ok(json!({"exit_requested":true}))
}
fn snapshot(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    let remote = world.resource::<RemoteState>();
    let mut result = json!({"fixed_tick":remote.tick,"epoch":remote.epoch,"reset_pending":remote.reset,"state":format!("{:?}",world.resource::<State<GameState>>().get()),"queued_inputs":remote.queue.values().map(Vec::len).sum::<usize>(),"held_inputs":remote.held.keys().collect::<Vec<_>>(),"player":null});
    let session = world.resource::<Session>();
    result["session_id"] = json!(world.resource::<crate::session::SessionId>().0);
    let equipment = world.resource::<crate::equipment::EquipmentEvidence>();
    result["equipment"] = json!({"retired_blasts":equipment.retired_blasts,"self_impacts":equipment.self_impacts,"enemy_impacts":equipment.enemy_impacts,"round_awards":equipment.round_awards});
    result["session"] = json!({"round":session.round,"pending":session.pending,"points":session.points,"score":session.score,"kills":session.kills,"shots":session.shots,"downs":session.downs,"spawn_remaining":session.spawn_timer.remaining_secs()});
    let entities: Vec<_> = world
        .query_filtered::<Entity, With<Player>>()
        .iter(world)
        .collect();
    if let Some(&entity) = entities.first() {
        let player = world.get::<Player>(entity).unwrap();
        let pose = world.get::<Transform>(entity).unwrap();
        let inventory = world.get::<WeaponInventory>(entity).unwrap();
        let weapons: Vec<_> = inventory.owned().map(|(slot,w)| {
            let definition = world.resource::<GameDefinition>().content.weapons.iter().find(|d|d.id==w.definition).unwrap();
            json!({"slot":slot,"id":w.definition.0,"magazine":w.magazine,"left_magazine":w.left_magazine,"reserve":inventory.reserve(definition)})
        }).collect();
        result["player"] = json!({"entity":entity.to_bits(),"position":pose.translation.to_array(),"yaw":player.yaw,"pitch":player.pitch,"height":player.height,"eye_height":player.eye_height,"health":world.get::<Vitality>(entity).and_then(|v| v.current()),"active_slot":inventory.active_slot(),"action":format!("{:?}",inventory.action),"weapons":weapons,"right_reload":inventory.hand_reload_timing(false),"left_reload":inventory.hand_reload_timing(true)});
    }
    if let Some(&entity) = entities.first() {
        let perks = world
            .get::<crate::solo_revive::PerkInventory>(entity)
            .unwrap();
        result["player"]["perks"] = json!(perks.owned.iter().map(|p| &p.0).collect::<Vec<_>>());
        result["player"]["solo_charges"] = json!(perks.solo_charges);
        result["player"]["drinking"] =
            json!(world.get::<crate::perks::PerkDrinking>(entity).is_some());
        result["player"]["grenades"] = json!(
            world
                .get::<crate::equipment::GrenadeInventory>(entity)
                .unwrap()
                .count
        );
    }
    result["enemies"] = json!(world.query::<(Entity,&Zombie,&Transform,Option<&Vitality>,Option<&crate::generators::GeneratorDefender>,Has<avian3d::prelude::ColliderDisabled>)>().iter(world).map(|(e,_,p,v,d,c)|json!({"entity":e.to_bits(),"position":p.translation.to_array(),"health":v.and_then(|v| v.current()),"defender":d.map(|d|&d.generator.0),"collider_disabled":c})).collect::<Vec<_>>());
    if let Some(g) = world.get_resource::<crate::generators::Generators>() {
        let definitions = world.get_resource::<GameDefinition>();
        let sites: Vec<_> = g.sites.iter().map(|(id,s)| {
            let d = definitions.and_then(|g|g.content.generators.iter().find(|d|d.id==*id));
            let position = d.and_then(|d|definitions?.content.placements.iter().find(|p|p.id==d.placement)).map(|p|p.position);
            let reach = d.and_then(|d|definitions?.content.interactions.iter().find(|i|i.placement==d.placement)).map(|i|i.reach);
            json!({"id":id.0,"number":d.map(|d|d.number),"state":format!("{:?}",s.state),"progress":s.progress,"position":position,"interaction_reach":reach,"model_position":d.map(|d|d.model_position),"radius":d.map(|d|d.radius),"price":d.map(|d|d.price),"zones":d.map(|d|&d.zones)})
        }).collect();
        result["generators"] = json!({"sites":sites,"activations":g.activations,"completions":g.completions,"excess_retired":g.excess_retired,"reserve":g.reserve()});
    }
    if let Some(p) = world.get_resource::<crate::rounds::Population>() {
        result["population"] =
            json!({"ordinary":p.ordinary,"special":p.special,"defenders":p.defenders});
    }
    if let Some(r) = world.get_resource::<crate::rounds::RoundEvidence>() {
        result["round_evidence"] = json!({"retired":r.retired,"requeued":r.requeued,"replacement_admissions":r.replacement_admissions,"attempts":r.attempts,"admitted":r.admitted,"transitions":r.transitions});
    }
    if let Some(d) = world.get_resource::<crate::powerups::Drops>() {
        result["drops"] = json!({"spawned":d.spawned});
    }
    if let Some(a) = world.get_resource::<crate::audio::AudioEvidence>() {
        result["audio"] = json!({"events":a.events,"sinks":a.sinks});
    }
    result["controls"] = serde_json::to_value(world.resource::<crate::input::ControlSettings>())
        .map_err(|e| error(e.to_string()))?;
    if let Some(f) = world.get_resource::<crate::interactions::InteractionFocus>() {
        result["focus"] = json!({"target":f.target.map(Entity::to_bits),"prompt":f.prompt,"accepted":f.accepted,"rejection":f.rejection.map(|r|format!("{r:?}"))});
    }
    Ok(result)
}
fn setup(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Setup = parse(value)?;
    if *world.resource::<State<GameState>>().get() != GameState::Playing {
        return Err(error("Setup requires Playing"));
    }
    let owner = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .map_err(|_| error("Player unavailable"))?;
    if request.enemies.len() > 64
        || request
            .round_event
            .as_ref()
            .is_some_and(|r| !(1..=10000).contains(&r.round))
        || request.blast.as_ref().is_some_and(|b| {
            b.center.iter().any(|n| !n.is_finite())
                || !b.radius.is_finite()
                || !(0.01..=100.).contains(&b.radius)
                || b.damage > 1000000
        })
        || request.pending.is_some_and(|n| n > 10000)
        || request
            .pause_spawning_seconds
            .is_some_and(|n| !n.is_finite() || !(0.0..=300.).contains(&n))
        || request
            .position
            .is_some_and(|p| p.iter().any(|n| !n.is_finite()))
        || [request.yaw, request.pitch]
            .into_iter()
            .flatten()
            .any(|n| !n.is_finite())
    {
        return Err(error("Setup outside bounds"));
    }
    let game = world.resource::<GameDefinition>();
    let perks = request
        .owned_perks
        .as_ref()
        .map(|ids| {
            if ids.len() > 8
                || ids
                    .iter()
                    .any(|id| !game.content.perks.iter().any(|p| p.id.0 == *id))
            {
                return Err(error("Unknown perk or too many perks"));
            }
            let owned: std::collections::BTreeSet<_> =
                ids.iter().cloned().map(crate::origins::PerkId).collect();
            if owned.len() != ids.len() {
                return Err(error("Duplicate setup perks"));
            }
            Ok(crate::solo_revive::PerkInventory {
                solo_charges: u32::from(owned.contains(&game.solo_revive.perk)),
                owned,
                ..default()
            })
        })
        .transpose()?;
    let inventory = if let Some(ids) = &request.weapons {
        if ids.is_empty() || ids.len() > 2 {
            return Err(error("Expected 1..2 weapons"));
        }
        let mut inventory = WeaponInventory::default();
        for id in ids {
            let definition = game
                .content
                .weapons
                .iter()
                .find(|d| d.id.0 == *id)
                .ok_or_else(|| error("Unknown weapon"))?;
            inventory
                .grant(definition)
                .map_err(|e| error(format!("{e:?}")))?;
        }
        Some(inventory)
    } else {
        None
    };
    for actor in &request.enemies {
        if actor.health == 0
            || actor.health > 1000000
            || actor.position.iter().any(|n| !n.is_finite())
            || actor
                .defender
                .as_ref()
                .is_some_and(|id| !game.content.generators.iter().any(|g| g.id.0 == *id))
        {
            return Err(error("Invalid actor setup"));
        }
    }
    let actor_definition = game
        .content
        .enemies
        .iter()
        .find(|e| e.family == "german")
        .ok_or_else(|| error("Native German actor unavailable"))?
        .id
        .clone();
    let blast = request.blast.map(|b| crate::equipment::BlastRequest {
        session: crate::session::SessionId(b.session_id),
        owner,
        definition: world
            .get::<crate::equipment::GrenadeInventory>(owner)
            .unwrap()
            .definition
            .clone(),
        center: Vec3::from_array(b.center),
        radius: b.radius,
        inner_radius: 0.,
        inner_damage: b.damage,
        outer_damage: b.damage,
        direction: None,
    });
    if let Some(inventory) = inventory {
        world.entity_mut(owner).insert(inventory);
    }
    if let Some(perks) = perks {
        world.entity_mut(owner).insert(perks);
    }
    if let Some(position) = request.position {
        world.get_mut::<Transform>(owner).unwrap().translation = Vec3::from_array(position);
    }
    {
        let mut player = world.get_mut::<Player>(owner).unwrap();
        if let Some(yaw) = request.yaw {
            player.yaw = yaw;
        }
        if let Some(pitch) = request.pitch {
            player.pitch = pitch.clamp(-1.5, 1.5);
        }
        player.velocity = Vec3::ZERO;
    }
    if request.clear_enemies {
        let entities: Vec<_> = world
            .query_filtered::<Entity, With<Zombie>>()
            .iter(world)
            .collect();
        for entity in entities {
            world.despawn(entity);
        }
    }
    let mut spawned = Vec::new();
    for actor in request.enemies {
        let position = Vec3::from_array(actor.position);
        let mut entity = world.spawn((
            SessionEntity,
            Zombie::new(
                position,
                actor_definition.clone(),
                crate::rounds::Gait::Walk,
                1.,
            ),
            Vitality::ordinary_actor(actor.health, actor.health).unwrap(),
            crate::player::capsule(0.3, 1.7),
            Transform::from_translation(position),
            Visibility::Inherited,
        ));
        if let Some(id) = actor.defender {
            entity.insert((
                crate::rounds::PopulationKind::GeneratorDefender,
                crate::generators::GeneratorDefender {
                    generator: crate::origins::ContentId(id),
                    slot: 0,
                },
            ));
        } else {
            entity.insert(crate::rounds::PopulationKind::Ordinary);
        }
        spawned.push(entity.id().to_bits());
    }
    let mut session = world.resource_mut::<Session>();
    if let Some(points) = request.points {
        session.points = points;
    }
    if let Some(pending) = request.pending {
        session.pending = pending;
    }
    if let Some(seconds) = request.pause_spawning_seconds {
        session.spawn_timer = Timer::from_seconds(seconds, TimerMode::Once);
    }
    if let Some(blast) = blast {
        world
            .resource_mut::<crate::equipment::BlastQueue>()
            .0
            .push(blast);
    }
    if let Some(event) = request.round_event {
        world.write_message(crate::rounds::RoundStarted {
            session: crate::session::SessionId(event.session_id),
            round: event.round,
        });
    }
    Ok(json!({"setup_applied":true,"gameplay_evidence":false,"enemies":spawned}))
}
