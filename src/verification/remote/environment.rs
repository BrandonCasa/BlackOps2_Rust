use super::*;
use crate::environment::{
    digging::{DigTarget, Digger, Digging},
    parts::Parts,
    support::{DamageContact, DamageEmitter, EnvironmentOwner, ShootableEnvironment},
    weather::{Weather, WeatherKind},
};
use crate::origins::ContentId;
use avian3d::prelude::*;

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Setup {
    seed: Option<u64>,
    round: Option<u32>,
    weather: Option<WeatherKind>,
    intensity: Option<u32>,
    overrides: Option<BTreeMap<u32, WeatherKind>>,
    last_snow: Option<u32>,
    last_rain: Option<u32>,
    digger: Option<Digger>,
    active_mounds: Option<Vec<usize>>,
    clear_pickups: bool,
    parts: Vec<PartSetup>,
    contacts: Vec<ContactSetup>,
    shootables: Vec<[f32; 3]>,
    box_moved: Option<bool>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartSetup {
    id: String,
    position: [f32; 3],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ContactSetup {
    vehicle: bool,
    position: [f32; 3],
    size: [f32; 3],
    amount: u32,
}

pub(super) fn schema(_: In<Option<Value>>) -> BrpResult {
    Ok(json!({"version":1,"methods":{
        "origins/environment/setup":{"effect":"Direct session scenario setup, never gameplay proof","params":{"seed":"optional u64 PRNG state","round":"optional 1..255 ending-round index","weather":"optional clear/rain/snow","intensity":"optional 1..4","overrides":"optional ending-round -> weather map","last_snow":"optional u32 history","last_rain":"optional u32 history","digger":"optional serialized Digger","active_mounds":"optional unique indices in authored mound catalog; resets mound set","clear_pickups":"optional bool","parts":[{"id":"shared part catalog ID","position":"[x,y,z]"}],"contacts":[{"vehicle":"bool, false robot","position":"[x,y,z]","size":"positive [x,y,z]","amount":"1..1000000"}],"shootables":"optional target positions","box_moved":"optional bool"}},
        "origins/state":{"environment":"weather, digging, digger, parts, targets, authored sites, ambient room samples, environmental hits/contacts"}
    }}))
}

pub(super) fn snapshot(world: &mut World) -> Value {
    let game = world.resource::<GameDefinition>();
    let definition = game.environment.digging.as_ref();
    let location = |l: &crate::environment::digging::DigLocation| json!({"id":l.id,"position":l.position,"zone":l.zone,"group":l.group,"initial":l.initial});
    let mut value = json!({"weather":world.resource::<Weather>(),"digging":world.resource::<Digging>(),"parts":world.resource::<Parts>(),
        "locations":definition.map(|d|json!({"shovels":d.shovels.iter().map(location).collect::<Vec<_>>(),"mounds":d.mounds.iter().map(location).collect::<Vec<_>>(),"hidden":d.hidden.iter().map(location).collect::<Vec<_>>()})),
        "part_definitions":game.environment.parts.iter().map(|p|json!({"id":p.id.0,"zone":p.dig_zone,"initial_misses":p.initial_misses})).collect::<Vec<_>>(),
        "ambient_rooms":game.environment.triggers.iter().filter(|t|t.source.get("targetname").is_some_and(|n|n=="ambient_package")).map(|t|json!({"id":t.id,"room":t.source.get("script_ambientroom"),"positions":t.interior_points().iter().map(|p|p.to_array()).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "support":world.resource::<crate::environment::support::EnvironmentEvidence>()});
    value["digger"]=world.query_filtered::<(&Digger,&crate::solo_revive::PerkInventory),With<Player>>().iter(world).next().map_or(Value::Null,|(d,p)|json!({"state":d,"perk_limit":p.purchase_limit(),"earned_slots":p.earned_slots}));
    value["targets"]=json!(world.query::<(Entity,&DigTarget,&Transform,Option<&crate::environment::digging::RisingPickup>,Option<&crate::environment::digging::WeaponTimeout>,&Visibility)>().iter(world).map(|(e,t,p,r,w,v)|json!({"entity":e.to_bits(),"target":t,"position":p.translation.to_array(),"rising":r.map(|r|r.remaining),"expires":w.map(|w|w.0),"visible":*v!=Visibility::Hidden})).collect::<Vec<_>>());
    value["audio_active"] = json!(world.query::<(&crate::audio::PlayingSound,Option<&bevy::audio::AudioSink>)>().iter(world).map(|(sound,sink)|json!({"alias":sound.alias,"looping":sound.looping,"sink":sink.is_some()})).collect::<Vec<_>>());
    value
}

pub(super) fn setup(In(value): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Setup = super::parse(value)?;
    if *world.resource::<State<GameState>>().get() != GameState::Playing {
        return Err(super::error("Setup requires Playing"));
    }
    let owner = world
        .query_filtered::<Entity, With<Player>>()
        .single(world)
        .map_err(|_| super::error("Player unavailable"))?;
    let game = world.resource::<GameDefinition>().clone();
    let Some(definition) = &game.environment.digging else {
        return Err(super::error("Digging content unavailable"));
    };
    if request.round.is_some_and(|n| !(1..=255).contains(&n))
        || request.intensity.is_some_and(|n| !(1..=4).contains(&n))
        || request
            .overrides
            .as_ref()
            .is_some_and(|o| o.len() > 255 || o.keys().any(|n| !(1..=255).contains(n)))
        || request.active_mounds.as_ref().is_some_and(|v| {
            v.len() > definition.mounds.len()
                || v.iter().any(|i| *i >= definition.mounds.len())
                || v.iter().collect::<std::collections::BTreeSet<_>>().len() != v.len()
        })
        || request.digger.as_ref().is_some_and(|d| {
            d.qualifying_digs > 30
                || d.hidden_found > 4
                || d.losing_streak > 100000
                || d.hidden_used.iter().any(|i| *i >= definition.hidden.len())
        })
        || request.parts.len() > 32
        || request.contacts.len() > 8
        || request.shootables.len() > 8
        || request.parts.iter().any(|p| {
            p.position.iter().any(|n| !n.is_finite())
                || !game.environment.parts.iter().any(|d| d.id.0 == p.id)
        })
        || request.contacts.iter().any(|c| {
            c.position.iter().any(|n| !n.is_finite())
                || c.size
                    .iter()
                    .any(|n| !n.is_finite() || *n <= 0. || *n > 100.)
                || c.amount == 0
                || c.amount > 1000000
        })
        || request.shootables.iter().flatten().any(|n| !n.is_finite())
    {
        return Err(super::error("Environment setup outside bounds"));
    }
    if let Some(seed) = request.seed {
        world.resource_mut::<Session>().seed = seed;
    }
    if let Some(round) = request.round {
        let mut s = world.resource_mut::<Session>();
        s.round = round;
        s.intermission = None;
    }
    if let Some(digger) = request.digger {
        world.entity_mut(owner).insert(digger);
    }
    if request.weather.is_some()
        || request.overrides.is_some()
        || request.last_snow.is_some()
        || request.last_rain.is_some()
    {
        let mut weather = world.resource_mut::<Weather>();
        if let Some(kind) = request.weather {
            weather.delay = 0.;
            weather.kind = kind;
            weather.presented = kind;
            weather.intensity = if kind == WeatherKind::Clear {
                0
            } else {
                request.intensity.unwrap_or(4)
            };
            weather.presented_intensity = weather.intensity;
        }
        if let Some(overrides) = request.overrides {
            weather.overrides = overrides;
        }
        if let Some(n) = request.last_snow {
            weather.last_snow = n;
        }
        if let Some(n) = request.last_rain {
            weather.last_rain = n;
        }
    }
    if let Some(mounds) = request.active_mounds {
        let old: Vec<_> = world
            .query::<(Entity, &DigTarget)>()
            .iter(world)
            .filter(|(_, t)| matches!(t, DigTarget::Mound(_)))
            .map(|(e, _)| e)
            .collect();
        for e in old {
            world.despawn(e);
        }
        world.resource_mut::<Digging>().active = mounds.iter().copied().collect();
        let server = world.resource::<AssetServer>().clone();
        for i in mounds {
            crate::environment::digging::mound(&mut world.commands(), &server, &game, i, false);
        }
    }
    if request.clear_pickups {
        let old: Vec<_> = world
            .query_filtered::<Entity, Or<(
                With<crate::powerups::Pickup>,
                With<crate::environment::digging::WeaponTimeout>,
                With<crate::environment::digging::PerkSlotBottle>,
            )>>()
            .iter(world)
            .collect();
        for e in old {
            world.despawn(e);
        }
    }
    if let Some(moved) = request.box_moved {
        world.resource_mut::<Digging>().box_moved = moved;
    }
    let server = world.resource::<AssetServer>().clone();
    let mut revealed = Vec::new();
    world.resource_scope(|world, mut parts: Mut<Parts>| {
        for p in request.parts {
            revealed.push(
                crate::environment::digging::spawn_part(
                    &mut world.commands(),
                    &server,
                    &game,
                    &mut parts,
                    &ContentId(p.id),
                    Vec3::from_array(p.position),
                )
                .map(Entity::to_bits),
            );
        }
    });
    let session = *world.resource::<crate::session::SessionId>();
    for (i, c) in request.contacts.into_iter().enumerate() {
        world.spawn((
            SessionEntity,
            if c.vehicle {
                DamageEmitter::Vehicle(format!("vehicle.validation.{i}"))
            } else {
                DamageEmitter::Robot(format!("robot.validation.{i}"))
            },
            DamageContact {
                session,
                amount: c.amount,
            },
            Collider::cuboid(c.size[0], c.size[1], c.size[2]),
            Sensor,
            CollisionLayers::new(32u32, LayerMask::NONE),
            Transform::from_translation(Vec3::from_array(c.position)),
        ));
    }
    for p in request.shootables {
        world.spawn((
            SessionEntity,
            EnvironmentOwner::Quest,
            ShootableEnvironment,
            Collider::cuboid(0.5, 0.5, 0.5),
            Sensor,
            CollisionLayers::new(32u32, LayerMask::NONE),
            Transform::from_translation(Vec3::from_array(p)),
        ));
    }
    world.flush();
    Ok(json!({"setup_only":true,"revealed_parts":revealed}))
}
