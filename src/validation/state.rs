use super::Controller;
use crate::{GameState, life::Vitality, player::Player};
use bevy::prelude::*;
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::VecDeque;

#[derive(Default, Serialize)]
pub(super) struct Evidence {
    pub hits: u64,
    pub kills: u64,
    pub landings: u64,
    pub interactions: u32,
    pub sequence: u64,
    pub events: VecDeque<Value>,
}
impl Evidence {
    fn push(&mut self, session: u64, tick: u64, kind: &str, data: Value) {
        self.sequence += 1;
        if self.events.len() == 512 {
            self.events.pop_front();
        }
        self.events.push_back(
            json!({"sequence":self.sequence,"session":session,"tick":tick,"kind":kind,"data":data}),
        );
    }
}
pub(super) fn observe(
    mut controller: ResMut<Controller>,
    mut hits: MessageReader<crate::damage::DamageApplied>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut landings: MessageReader<crate::movement::Landing>,
    players: Query<(), With<Player>>,
    focus: Res<crate::interactions::InteractionFocus>,
) {
    let Some(session) = controller.session else {
        hits.clear();
        deaths.clear();
        landings.clear();
        return;
    };
    let tick = controller.tick;
    for hit in hits.read().filter(|h| players.contains(h.impact.attacker)) {
        controller.evidence.hits += 1;
        controller.evidence.push(session,tick,"hit",json!({"attacker":hit.impact.attacker.to_bits(),"target":hit.impact.target.to_bits(),"source":hit.impact.source.id(),"before":hit.before,"lethal":hit.lethal}));
    }
    for death in deaths
        .read()
        .filter(|d| players.contains(d.0.impact.attacker))
    {
        controller.evidence.kills += 1;
        controller.evidence.push(session,tick,"kill",json!({"attacker":death.0.impact.attacker.to_bits(),"target":death.0.impact.target.to_bits(),"source":death.0.impact.source.id(),"point":death.0.impact.point.map(|p|p.to_array())}));
    }
    for landing in landings.read() {
        controller.evidence.landings += 1;
        controller.evidence.push(session,tick,"landing",json!({"owner":landing.owner.to_bits(),"position":landing.position.to_array(),"height":landing.height}));
    }
    if focus.accepted != controller.evidence.interactions {
        controller.evidence.interactions = focus.accepted;
        controller.evidence.push(session,tick,"interaction",json!({"target":focus.target.map(Entity::to_bits),"accepted":focus.accepted,"prompt":focus.prompt}));
    }
}

pub(super) fn snapshot(world: &mut World) -> Value {
    let player=world.query_filtered::<(Entity,&Player,&Transform,&Vitality,&crate::weapons::WeaponInventory,Option<&crate::environment::digging::Digger>,Option<&crate::movement::platforms::PlatformSupport>),With<Player>>().single(world).ok().map(|(entity,p,t,h,w,d,s)| {
        json!({"entity":entity.to_bits(),"position":t.translation.to_array(),"feet":(t.translation-Vec3::Y*p.height*0.5).to_array(),"eye":(t.translation+Vec3::Y*(p.eye_height-p.height*0.5)).to_array(),"yaw":p.yaw,"pitch":p.pitch,"grounded":p.grounded,"velocity":p.velocity.to_array(),"health":h.current(),"life":format!("{:?}",h.state()),"platform":s.and_then(|s|s.entity).map(Entity::to_bits),"shovel":d.is_some_and(|d|d.shovel),"inventory":w.owned().map(|(slot,weapon)|json!({"slot":slot,"id":weapon.definition.0,"magazine":weapon.magazine})).collect::<Vec<_>>(),"weapon":w.active().map(|w|w.definition.0.clone())})
    });
    let enemies:Vec<_>=world.query::<(Entity,&crate::enemies::Zombie,&Transform,&Vitality,Option<&crate::rounds::SpawnZone>)>().iter(world).map(|(e,z,t,h,zone)|json!({"entity":e.to_bits(),"position":t.translation.to_array(),"health":h.current(),"life":format!("{:?}",h.state()),"emerged":z.completed_emerging(),"zone":zone.map(|z|z.0.0.clone())})).collect();
    let mut targets:Vec<_>=world.query::<(Entity,&crate::environment::digging::DigTarget,&Transform)>().iter(world).map(|(e,target,t)|json!({"entity":e.to_bits(),"target":target,"position":t.translation.to_array()})).collect();
    targets.extend(
        world
            .query::<(Entity, &crate::crafting::CraftTarget, &Transform)>()
            .iter(world)
            .map(|(e, target, t)| {
                let target = match target {
                    crate::crafting::CraftTarget::Part(id) => {
                        json!({"kind":"craft_part","value":id})
                    }
                    crate::crafting::CraftTarget::Bench(index, recipe) => {
                        json!({"kind":"bench","index":index,"recipe":recipe})
                    }
                };
                json!({"entity":e.to_bits(),"target":target,"position":t.translation.to_array()})
            }),
    );
    let c = world.resource::<Controller>();
    let session = world.resource::<crate::gameplay::Session>();
    let focus = world.resource::<crate::interactions::InteractionFocus>();
    let mut result = json!({"session":world.resource::<crate::session::SessionId>().0,"seed":c.seed,"game_state":format!("{:?}",world.resource::<State<GameState>>().get()),"ready":matches!(world.resource::<State<GameState>>().get(),GameState::Playing|GameState::Paused)&&player.is_some(),"control":{"owned":c.owns_input(),"paused":c.paused,"aborted":c.aborted,"failure":c.failure,"tick":c.tick,"simulation_seconds":c.simulation_seconds,"label":c.label,"queued":c.queue.len(),"active":c.active.as_ref().map(|(r,n)|json!({"id":r.id,"remaining":n})),"god":c.god},"player":player,"round":{"number":session.round,"pending":session.pending,"points":session.points},"interaction":{"target":focus.target.map(Entity::to_bits),"accepted":focus.accepted,"prompt":focus.prompt,"rejection":focus.rejection.map(|r|format!("{r:?}"))},"enemies":enemies,"targets":targets,"evidence":c.evidence});
    result["process_id"] = json!(std::process::id());
    if let Some(physics) = world.get_resource::<Time<avian3d::prelude::Physics>>() {
        result["physics_seconds"] = json!(physics.elapsed_secs_f64());
    }
    if let Some(quest) = world.get_resource::<crate::quests::MainQuest>() {
        result["quest"] = json!({"stage":quest.stage,"upgraded":quest.upgraded,"placed":quest.placed,"authorized":quest.strike_authorized()});
    }
    if let Some(parts) = world.get_resource::<crate::environment::parts::Parts>() {
        result["parts"] = serde_json::to_value(parts).unwrap();
    }
    if let Some(generators) = world.get_resource::<crate::generators::Generators>() {
        result["generators"] = json!({"activations":generators.activations,"completions":generators.completions,"sites":generators.sites.iter().map(|(id,s)|(&id.0,json!({"state":format!("{:?}",s.state),"progress":s.progress}))).collect::<std::collections::BTreeMap<_,_>>()});
    }
    result
}
#[derive(Component)]
pub(super) struct Overlay;
pub(super) fn overlay(
    mut commands: Commands,
    controller: Res<Controller>,
    cameras: Query<Entity, With<crate::rendering::display::GameUiCamera>>,
    mut text: Query<&mut Text, With<Overlay>>,
    mut previous: Local<String>,
) {
    let label = format!(
        "ORIGINS PLAYTHROUGH  |  session {:?}\n{}{}",
        controller.session,
        controller.label,
        if controller.paused {
            "\nPAUSED  |  BRP resume / step / abort"
        } else {
            "\nESC pause  |  F12 abort"
        }
    );
    if *previous == label && !text.is_empty() {
        return;
    }
    if let Ok(mut text) = text.single_mut() {
        text.0 = label.clone();
    } else if let Ok(camera) = cameras.single() {
        commands.spawn((
            Overlay,
            Text::new(label.clone()),
            TextFont {
                font_size: 18.0.into(),
                ..default()
            },
            TextColor(Color::WHITE),
            UiTargetCamera(camera),
            GlobalZIndex(120),
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                top: px(12.),
                left: px(12.),
                padding: UiRect::all(px(8.)),
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.8)),
        ));
    }
    *previous = label;
}
