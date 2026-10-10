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
    charge_level: Option<u32>,
    position: Option<[f32; 3]>,
    yaw: Option<f32>,
    pitch: Option<f32>,
    weapons: Option<Vec<String>>,
    equipment: Vec<String>,
    pickups: Vec<RemotePickup>,
    clear_enemies: bool,
    pending: Option<u32>,
    pause_spawning_seconds: Option<f32>,
    enemies: Vec<Actor>,
    blast: Option<RemoteBlast>,
    owned_perks: Option<Vec<String>>,
    round_event: Option<RemoteRound>,
    open_connections: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RemotePickup {
    id: String,
    position: [f32; 3],
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
    super::pipelines::install(app);
    app.add_observer(preview_ready)
        .init_resource::<RemoteState>()
        .init_resource::<crate::audio::RequestedAliases>()
        .configure_sets(PreUpdate, InputSystems.run_if(|| false))
        .add_plugins((
            RemotePlugin::default()
                .with_method_main("origins/schema", schema)
                .with_method_main("origins/state", snapshot)
                .with_method_main("origins/materials", materials)
                .with_method_main("origins/capture", capture)
                .with_method_main("origins/preview", preview)
                .with_method_main("origins/animations", animations)
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
        "bevy_version":"0.20.0",
        "endpoints":{"main":format!("http://127.0.0.1:{}",port.0),"render":"http://127.0.0.1:15703"},
        "discovery":{"methods":"rpc.discover","schemas":"registry.schema"},
        "methods":{
            "origins/animations":{"params":{},"result":"fixed_tick, weapons (ready, action, clip clocks, local bone poses), enemies (action, gait, native combat state, sequence, elapsed, notetracks, rig poses and clocks)"},
            "origins/state":{"params":{},"result":"fixed_tick, fixed_delta_seconds, epoch, session_id, state, queued_inputs, held_inputs, session, player (life, ads, prone, crouched, velocity, lethal, tactical, mine), firing (shots with stance, speed, spread, ADS and perk scales; first 128), enemies (nuked, ready, hit_regions with world endpoints), damage (recent with base_amount and insta_kill promotion, awarded_points, headshot_kills, melee_kills), generators, population, round_evidence, equipment (events, active_grenades, mine_placed), drops (spawned, collected, expired, active), effects, audio (requests before mixer rejection, events started, sinks), controls, focus"},
            "origins/materials":{"params":{},"result":"pending, first_person, world, inventory; native material, texture, tangent readiness"},
            "origins/preview":{"params":{"weapon":"optional native ID; omitted clears preview"},"effect":"World-model scenario setup, attached to camera for inspection"},
            "origins/capture":{"params":{"path":"absolute PNG path"},"result":"asynchronous screenshot request"},
            "origins/reset":{"params":{},"effect":"Release inputs, transition to GameOver, send normal RestartGame"},
            "origins/input":{"params":{"events":[{"tick":"relative integer 1..3600","key":"Bevy KeyCode name or MouseLeft/MouseRight/MouseMiddle","pressed":"boolean"}]},"max_events":512,"effect":"Queue native input messages; automatic release after final scheduled tick"},
            "origins/release":{"params":{},"effect":"Cancel queued input and release remotely held buttons"},
            "origins/quit":{"params":{},"effect":"Release remote inputs and emit normal AppExit"},
            "origins/setup":{"params":{"points":"optional u32","position":"optional [x,y,z] player center","yaw":"optional radians","pitch":"optional radians","weapons":"optional 1..2 native weapon IDs","charge_level":"optional 0..3 source staff charge level, scenario setup","equipment":"optional up to 3 native IDs, one per lethal/tactical/mine slot","pickups":[{"id":"supported native powerup ID","position":"[x,y,z]"}],"owned_perks":"optional 0..8 unique native perk IDs","open_connections":"optional native door connection IDs","blast":{"session_id":"u64","center":"[x,y,z]","damage":"0..1000000","radius":"0.01..100"},"round_event":{"session_id":"u64","round":"1..10000"},"clear_enemies":"optional bool","pending":"optional 0..10000","pause_spawning_seconds":"optional 0..300","enemies":[{"position":"[x,y,z]","health":"1..1000000, default100","defender":"optional generator ID"}]},"max_enemies":64,"max_pickups":64,"effect":"Direct scenario setup only; not evidence of gameplay"}
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
fn animations(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    Ok(json!({"fixed_tick":world.resource::<RemoteState>().tick,
        "weapons":crate::presentation::animation_observation(world),
        "enemies":crate::enemies::animation_observation(world)}))
}
fn snapshot(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    let remote = world.resource::<RemoteState>();
    let mut result = json!({"fixed_tick":remote.tick,"epoch":remote.epoch,"reset_pending":remote.reset,"state":format!("{:?}",world.resource::<State<GameState>>().get()),"queued_inputs":remote.queue.values().map(Vec::len).sum::<usize>(),"held_inputs":remote.held.keys().collect::<Vec<_>>(),"player":null});
    result["fixed_delta_seconds"] = json!(world.resource::<Time<Fixed>>().timestep().as_secs_f64());
    let session = world.resource::<Session>();
    result["session_id"] = json!(world.resource::<crate::session::SessionId>().0);
    let equipment = world.resource::<crate::equipment::EquipmentEvidence>();
    result["equipment"] = json!({"retired_blasts":equipment.retired_blasts,"self_impacts":equipment.self_impacts,"enemy_impacts":equipment.enemy_impacts,"round_awards":equipment.round_awards,
        "primed":equipment.primed,"thrown":equipment.thrown,"cooked_off":equipment.cooked_off,"detonations":equipment.detonations,"events":equipment.events});
    result["session"] = json!({"round":session.round,"pending":session.pending,"points":session.points,"score":session.score,"kills":session.kills,"shots":session.shots,"downs":session.downs,"spawn_remaining":session.spawn_timer.remaining_secs()});
    let history = world.resource::<crate::damage::CombatHistory>();
    result["damage"] = json!({"awarded_points":history.awarded_points,"headshot_kills":history.headshot_kills,"melee_kills":history.melee_kills,
        "recent":history.recent.iter().map(|receipt|json!({"attacker":receipt.impact.attacker.to_bits(),"target":receipt.impact.target.to_bits(),"source":receipt.impact.source.id(),"category":format!("{:?}",receipt.impact.category),"region":format!("{:?}",receipt.impact.region),"amount":receipt.impact.amount,"base_amount":receipt.base_amount,"lethal":receipt.lethal,"points":receipt.points,"aimed_bullet":receipt.aimed_bullet,"insta_kill":receipt.insta_kill})).collect::<Vec<_>>()});
    result["equipment"]["active_grenades"] = json!(world.query::<(Entity, &crate::equipment::Grenade, &Transform)>().iter(world)
        .map(|(entity, grenade, pose)| json!({"entity":entity.to_bits(),"id":grenade.definition.0,"remaining":grenade.remaining,"position":pose.translation.to_array()})).collect::<Vec<_>>());
    result["equipment"]["mine_placed"] =
        json!(world.resource::<crate::mines::MineEvidence>().placed);
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
        result["player"] = json!({"entity":entity.to_bits(),"position":pose.translation.to_array(),"yaw":player.yaw,"pitch":player.pitch,"height":player.height,"eye_height":player.eye_height,"prone":player.prone,"crouched":player.crouched,"velocity":player.velocity.to_array(),"health":world.get::<Vitality>(entity).and_then(|v| v.current()),"active_slot":inventory.active_slot(),"action":format!("{:?}",inventory.action),"weapons":weapons,"ads":inventory.ads,"right_reload":inventory.hand_reload_timing(false),"left_reload":inventory.hand_reload_timing(true)});
    }
    if let Some(&entity) = entities.first() {
        result["player"]["life"] = json!(format!(
            "{:?}",
            world.get::<Vitality>(entity).unwrap().state()
        ));
        let lethal = world
            .get::<crate::equipment::GrenadeInventory>(entity)
            .unwrap();
        result["player"]["lethal"] = json!({"id":lethal.definition.0,"count":lethal.count,"state":format!("{:?}",lethal.state)});
        result["player"]["tactical"] = json!(world.get::<crate::equipment::TacticalInventory>(entity).unwrap().0.as_ref()
            .map(|inventory| json!({"id":inventory.definition.0,"count":inventory.count,"state":format!("{:?}",inventory.state)})));
        let mine = world.get::<crate::mines::MineInventory>(entity).unwrap();
        result["player"]["mine"] = json!({"id":mine.definition.as_ref().map(|id|&id.0),"count":mine.count,"busy":mine.busy()});
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
    result["enemies"] = json!(world.query::<(Entity,&Zombie,&Transform,Option<&Vitality>,Option<&crate::generators::GeneratorDefender>,Has<avian3d::prelude::ColliderDisabled>,Has<crate::powerups::NukeTarget>,Has<crate::hit_regions::SkeletalReady>,Option<&crate::hit_regions::Hitboxes>)>().iter(world).map(|(e,_,p,v,d,c,n,r,h)|json!({"entity":e.to_bits(),"position":p.translation.to_array(),"health":v.and_then(|v| v.current()),"defender":d.map(|d|&d.generator.0),"collider_disabled":c,"nuked":n,"ready":r,"hit_regions":h.map(|h|h.volumes.iter().map(|v|json!({"id":v.id,"region":format!("{:?}",v.region),"start":p.compute_affine().transform_point3(v.start).to_array(),"end":p.compute_affine().transform_point3(v.end).to_array()})).collect::<Vec<_>>()).unwrap_or_default()})).collect::<Vec<_>>());
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
        result["drops"] = json!({"spawned":d.spawned,"collected":d.collected,"expired":d.expired});
    }
    result["drops"]["active"] = json!(world.query::<(Entity, &crate::powerups::Pickup, &Transform)>().iter(world)
        .map(|(entity,pickup,pose)|json!({"entity":entity.to_bits(),"id":world.resource::<GameDefinition>().content.powerups[pickup.definition].id.0,"position":pose.translation.to_array(),"remaining":pickup.remaining})).collect::<Vec<_>>());
    if let Some(a) = world.get_resource::<crate::audio::AudioEvidence>() {
        result["audio"] = json!({"events":a.events,"sinks":a.sinks});
    }
    result["audio"]["requests"] = json!(world.resource::<crate::audio::RequestedAliases>().0);
    let effects = world.resource::<crate::powerups::PowerupEffects>();
    result["firing"] = json!({"shots":world.resource::<crate::perks::FiringEvidence>().shots});
    result["effects"] = json!({"double_points":effects.double_points,"insta_kill":effects.insta_kill,"zombie_blood":effects.zombie_blood});
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
        || request.pickups.len() > 64
        || request.equipment.len() > 3
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
    let mut lethal = None;
    let mut tactical = None;
    let mut mine = None;
    for id in &request.equipment {
        let weapon = game
            .content
            .weapons
            .iter()
            .find(|w| w.id.0 == *id)
            .ok_or_else(|| error("Unknown equipment"))?;
        if let Some(profile) = &weapon.throwable {
            let inventory = crate::equipment::GrenadeInventory::new(weapon.id.clone(), profile);
            let previous = match profile.slot {
                crate::equipment::ThrowableSlot::Lethal => lethal.replace(inventory),
                crate::equipment::ThrowableSlot::Tactical => tactical.replace(inventory),
            };
            if previous.is_some() {
                return Err(error("Duplicate equipment slot"));
            }
        } else if weapon.mine.is_some() {
            let mut inventory = crate::mines::MineInventory::default();
            inventory
                .receive(weapon)
                .map_err(|e| error(format!("{e:?}")))?;
            if mine.replace(inventory).is_some() {
                return Err(error("Duplicate equipment slot"));
            }
        } else {
            return Err(error("Weapon is not equipment"));
        }
    }
    let pickup_setup = request
        .pickups
        .iter()
        .map(|pickup| {
            if pickup.position.iter().any(|n| !n.is_finite()) {
                return Err(error("Invalid pickup position"));
            }
            if !matches!(
                pickup.id.as_str(),
                "powerup.nuke"
                    | "powerup.full_ammo"
                    | "powerup.insta_kill"
                    | "powerup.double_points"
                    | "powerup.zombie_blood"
            ) {
                return Err(error("Unsupported powerup"));
            }
            let (index, definition) = game
                .content
                .powerups
                .iter()
                .enumerate()
                .find(|(_, d)| d.id.0 == pickup.id)
                .ok_or_else(|| error("Unknown powerup"))?;
            let scene = game
                .content
                .presentations
                .iter()
                .find(|p| Some(&p.id) == definition.presentation.as_ref())
                .and_then(|p| p.scene.clone())
                .ok_or_else(|| error("Powerup scene unavailable"))?;
            Ok((index, pickup.position, scene))
        })
        .collect::<Result<Vec<_>, BrpError>>()?;
    if request
        .open_connections
        .iter()
        .any(|id| !game.access.doors.iter().any(|d| d.connection.0 == *id))
    {
        return Err(error("Unknown door connection"));
    }
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
    if request.charge_level.is_some_and(|v| v > 3) {
        return Err(error("Charge level must be 0..3"));
    }
    if let Some(inventory) = inventory {
        world.entity_mut(owner).insert(inventory);
    }
    if let Some(level) = request.charge_level {
        world
            .get_mut::<WeaponInventory>(owner)
            .unwrap()
            .charge_level = level;
    }
    if let Some(inventory) = lethal {
        world.entity_mut(owner).insert(inventory);
    }
    if let Some(inventory) = tactical {
        world
            .entity_mut(owner)
            .insert(crate::equipment::TacticalInventory(Some(inventory)));
    }
    if let Some(inventory) = mine {
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
    let mut spawned_pickups = Vec::new();
    for (definition, position, scene) in pickup_setup {
        let scene = world.resource::<AssetServer>().load(scene);
        spawned_pickups.push(
            world
                .spawn((
                    SessionEntity,
                    crate::powerups::Pickup {
                        definition,
                        remaining: 26.5,
                    },
                    bevy::world_serialization::WorldAssetRoot(scene),
                    Transform::from_translation(Vec3::from_array(position)),
                    Visibility::Inherited,
                ))
                .id()
                .to_bits(),
        );
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
    world.resource_scope(|world, mut access: Mut<crate::access::ZoneAccess>| {
        let game = world.resource::<GameDefinition>();
        for id in request.open_connections {
            let _ = access.unlock(game, &crate::origins::ContentId(id));
        }
    });
    Ok(
        json!({"setup_applied":true,"gameplay_evidence":false,"enemies":spawned,"pickups":spawned_pickups}),
    )
}

fn materials(_: In<Option<Value>>, world: &mut World) -> BrpResult {
    let mut result = crate::rendering::weapon_material_snapshot(world);
    result["pipelines"] = super::pipelines::snapshot(world);
    Ok(result)
}
fn capture(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    use bevy::render::view::screenshot::{Screenshot, save_to_disk};
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Capture {
        path: std::path::PathBuf,
    }
    let request: Capture = parse(value)?;
    if !request.path.is_absolute()
        || request.path.extension().and_then(|s| s.to_str()) != Some("png")
    {
        return Err(error("Capture requires an absolute PNG path"));
    }
    if request.path.exists() {
        return Err(error("Capture target already exists"));
    }
    if let Some(parent) = request.path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| error(&e.to_string()))?;
    }
    window(world)?;
    world
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(request.path.clone()));
    Ok(json!({"requested":true,"path":request.path}))
}

#[derive(Component)]
struct MaterialPreview;
fn preview(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Preview {
        weapon: Option<String>,
    }
    let request: Preview = parse(value)?;
    let scene = request
        .weapon
        .as_ref()
        .map(|id| {
            let game = world.resource::<GameDefinition>();
            let weapon = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == *id)
                .ok_or_else(|| error("Unknown preview weapon"))?;
            let scene = weapon
                .pickup
                .as_ref()
                .and_then(|id| game.content.presentations.iter().find(|p| p.id == *id))
                .and_then(|p| p.scene.clone())
                .ok_or_else(|| error("Weapon has no world model"))?;
            Ok::<_, BrpError>((weapon.id.clone(), scene))
        })
        .transpose()?;
    let previous: Vec<_> = world
        .query_filtered::<Entity, With<MaterialPreview>>()
        .iter(world)
        .collect();
    for entity in previous {
        world.despawn(entity);
    }
    if let Some((weapon, scene)) = scene {
        let camera = world
            .query_filtered::<Entity, With<crate::rendering::GameCamera>>()
            .single(world)
            .map_err(|e| error(e.to_string()))?;
        let handle = world.resource::<AssetServer>().load(scene);
        world.spawn((
            bevy::world_serialization::WorldAssetRoot(handle),
            crate::presentation::camo::WeaponMaterialContext(weapon),
            MaterialPreview,
            SessionEntity,
            ChildOf(camera),
            Transform::from_xyz(0., -0.15, -0.9).with_rotation(Quat::from_rotation_y(0.8)),
            Visibility::Inherited,
        ));
    }
    Ok(json!({"requested":true,"weapon":request.weapon}))
}
fn preview_ready(
    event: On<bevy::world_serialization::WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<(), With<MaterialPreview>>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
) {
    if !roots.contains(event.entity) {
        return;
    }
    for entity in children
        .iter_descendants(event.entity)
        .filter(|e| meshes.contains(*e))
    {
        commands.entity(entity).insert((
            bevy::camera::visibility::RenderLayers::layer(1),
            bevy::light::NotShadowCaster,
        ));
    }
}
