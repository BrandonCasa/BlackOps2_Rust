use super::Controller;
use crate::{GameState, life::Vitality, player::Player};
use avian3d::prelude::SimpleCollider;
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
    #[serde(skip)]
    support: Option<u64>,
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
    mut jumps: MessageReader<crate::movement::Jump>,
    players: Query<Option<&crate::movement::platforms::PlatformSupport>, With<Player>>,
    focus: Res<crate::interactions::InteractionFocus>,
) {
    let Some(session) = controller.session else {
        hits.clear();
        deaths.clear();
        landings.clear();
        jumps.clear();
        return;
    };
    let tick = controller.tick;
    let support = controller.evidence.support;
    if let Ok(platform) = players.single() {
        controller.evidence.support = platform.and_then(|p| p.entity).map(Entity::to_bits);
    }
    for jump in jumps.read() {
        controller.evidence.push(session, tick, "jump", json!({"owner":jump.owner.to_bits(),
            "platform":support,"position":jump.position.to_array(),"velocity":jump.velocity.to_array()}));
    }
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
    let firing = world.query_filtered::<&crate::weapons::WeaponInventory, With<Player>>().single(world).ok().and_then(|inventory| {
        let active = inventory.active()?;
        let game = world.get_resource::<crate::assets::GameDefinition>()?;
        let definition = game.content.weapons.iter().find(|weapon| weapon.id == active.definition)?;
        Some(json!({"interval":definition.fire_interval * inventory.firing.interval_scale,
            "automatic":matches!(definition.mechanics.fire, crate::origins::content::FirePattern::Automatic),
            "aim_seconds":definition.mechanics.ads_seconds[0],
            "ready":inventory.action == crate::weapons::WeaponAction::Ready,
            "magazine":active.magazine}))
    });
    let player=world.query_filtered::<(Entity,&Player,&Transform,&Vitality,&crate::weapons::WeaponInventory,Option<&crate::environment::digging::Digger>,Option<&crate::movement::platforms::PlatformSupport>),With<Player>>().single(world).ok().map(|(entity,p,t,h,w,d,s)| {
        json!({"entity":entity.to_bits(),"position":t.translation.to_array(),"feet":(t.translation-Vec3::Y*p.height*0.5).to_array(),"eye":(t.translation+Vec3::Y*(p.eye_height-p.height*0.5)).to_array(),"yaw":p.yaw,"pitch":p.pitch,"grounded":p.grounded,"velocity":p.velocity.to_array(),"health":h.current(),"life":format!("{:?}",h.state()),"platform":s.and_then(|s|s.entity).map(Entity::to_bits),"shovel":d.is_some_and(|d|d.shovel),"inventory":w.owned().map(|(slot,weapon)|json!({"slot":slot,"id":weapon.definition.0,"magazine":weapon.magazine})).collect::<Vec<_>>(),"weapon":w.active().map(|w|w.definition.0.clone())})
    });
    let enemies:Vec<_>=world.query::<(Entity,&crate::enemies::Zombie,&Transform,&Vitality,Option<&crate::rounds::SpawnZone>,Option<&crate::generators::GeneratorDefender>,Option<&crate::hit_regions::Hitboxes>)>().iter(world).map(|(e,z,t,h,zone,defender,boxes)| {
        let head_offset=boxes.and_then(|boxes|boxes.volumes.iter().find(|v|v.region.head())).map(|v|(v.start+v.end)*0.5);
        let head=head_offset.map(|p|t.transform_point(p).to_array());
        let aim=t.transform_point(boxes.and_then(aim_point).unwrap_or(Vec3::ZERO));
        json!({"entity":e.to_bits(),"position":t.translation.to_array(),"aim":aim.to_array(),"head":head,"head_offset":head_offset.map(|p|p.to_array()),"definition":z.definition.0,"generator":defender.map(|d|&d.generator.0),"health":h.current(),"life":format!("{:?}",h.state()),"emerged":z.completed_emerging(),"zone":zone.map(|z|z.0.0.clone())})
    }).collect();
    let mut targets:Vec<_>=world.query::<(Entity,&crate::environment::digging::DigTarget,&Transform)>().iter(world).map(|(e,target,t)|json!({"entity":e.to_bits(),"target":target,"position":t.translation.to_array()})).collect();
    targets.extend(world.query::<(Entity, &crate::generators::rewards::Reward, &Transform)>().iter(world)
        .filter_map(|(e, reward, _)| reward.part().map(|(id, position)| json!({"entity":e.to_bits(),"target":{"kind":"capture_reward","value":id},"position":position.to_array()}))));
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
    targets.extend(world.query::<(Entity,&crate::interactions::Usable,&Transform)>().iter(world).filter_map(|(e,u,t)| {
        let target = match &u.source.action {
            crate::origins::content::InteractionAction::Generator { generator } => json!({"kind":"generator","value":generator.0}),
            crate::origins::content::InteractionAction::Weapon { weapon, .. } => json!({"kind":"wall_weapon","value":weapon.0}),
            _ => return None,
        };
        Some(json!({"entity":e.to_bits(),"target":target,"position":t.translation.to_array()}))
    }));
    targets.extend(
        world
            .query::<(Entity, &crate::portals::Target, &Transform)>()
            .iter(world)
            .map(|(e, t, pose)| {
                let target = match t {
                    crate::portals::Target::Gramophone(i) => json!({"kind":"gramophone","value":i}),
                    crate::portals::Target::Entrance => json!({"kind":"portal_entrance"}),
                    crate::portals::Target::Return(i) => json!({"kind":"portal_return","value":i}),
                };
                json!({"entity":e.to_bits(),"target":target,"position":pose.translation.to_array()})
            }),
    );
    let tank_uses: std::collections::BTreeMap<_, _> = world
        .get_resource::<crate::assets::GameDefinition>()
        .map(|game| {
            game.environment
                .triggers
                .iter()
                .filter_map(|t| {
                    let name = t.source.get("targetname")?;
                    matches!(name.as_str(), "trig_use_tank" | "trig_tank_station_call")
                        .then(|| (t.id.clone(), name.clone()))
                })
                .collect()
        })
        .unwrap_or_default();
    targets.extend(world.query::<(Entity,&crate::environment::triggers::MapTrigger,&Transform,&avian3d::prelude::Collider)>().iter(world).filter_map(|(e,t,pose,shape)| {
        let name=tank_uses.get(&t.0)?;
        let bounds=shape.aabb(pose.translation,pose.rotation,0.);
        Some(json!({"entity":e.to_bits(),"target":{"kind":name,"value":t.0},"position":((bounds.min+bounds.max)*0.5).to_array()}))
    }));
    let c = world.resource::<Controller>();
    let session = world.resource::<crate::gameplay::Session>();
    let focus = world.resource::<crate::interactions::InteractionFocus>();
    let mut result = json!({"session":world.resource::<crate::session::SessionId>().0,"seed":c.seed,"game_state":format!("{:?}",world.resource::<State<GameState>>().get()),"ready":matches!(world.resource::<State<GameState>>().get(),GameState::Playing|GameState::Paused)&&player.is_some(),"control":{"owned":c.owns_input(),"paused":c.paused,"aborted":c.aborted,"failure":c.failure,"tick":c.tick,"simulation_seconds":c.simulation_seconds,"label":c.label,"queued":c.queue.len(),"active":c.active.as_ref().map(|(r,n)|json!({"id":r.id,"remaining":n})),"god":c.god},"player":player,"round":{"number":session.round,"pending":session.pending,"points":session.points},"interaction":{"target":focus.target.map(Entity::to_bits),"accepted":focus.accepted,"prompt":focus.prompt,"rejection":focus.rejection.map(|r|format!("{r:?}"))},"enemies":enemies,"targets":targets,"evidence":c.evidence});
    result["process_id"] = json!(std::process::id());
    result["firing"] = firing.unwrap_or(Value::Null);
    result["buildables"] = world
        .query_filtered::<&crate::crafting::BuildableInventory, With<Player>>()
        .single(world)
        .ok()
        .map_or(
            Value::Null,
            |b| json!({"owned":b.owned,"equipped":b.equipped}),
        );
    if let Some(crafting) = world.get_resource::<crate::crafting::Crafting>() {
        result["crafting"] = json!({"crafted":crafting.crafted,"consumed":crafting.consumed,
            "records":crafting.records,"gramophone":crafting.gramophone});
    }
    if let Some(portals) = world.get_resource::<crate::portals::Portals>() {
        result["portals"] = json!({"gramophone":portals.gramophone,"entrances":portals.entrances.map(|p|format!("{p:?}")),
            "returns":portals.returns,"excavation_open":portals.excavation_open,"trips":portals.trips,
            "visited":portals.visited,"return_unlocked":portals.return_unlocked,"crystals_ready":portals.crystals_ready});
    }
    if let Some(weather) = world.get_resource::<crate::environment::weather::Weather>() {
        result["weather"] = json!({"kind":weather.kind,"presented":weather.presented,"intensity":weather.intensity,
            "selection":weather.selection});
    }
    if let Some(navigation) = world.get_resource::<crate::navigation::Navigation>() {
        result["spawn_candidates"] = json!(navigation.spawn_candidates().map(|(i,s)|
            json!({"candidate":i,"position":s.position.to_array(),"zone":s.zone.0,"source":s.source})).collect::<Vec<_>>());
    }
    if let Some(robots) = world.get_resource::<crate::robots::Robots>() {
        result["robots"] = json!({"selection":robots.selection,"entries":robots.entries,"evacuations":robots.evacuations,
            "passes":robots.passes.map(|p|p.map(|p|json!({"delay":p.delay,"step":p.step,"clock":p.clock,
                "eligible":p.eligible,"hatch":format!("{:?}",p.hatch)}))),
            "occupancy":robots.occupancy.map(|o|json!({"robot":o.robot,"remaining":o.remaining,"exit":o.exit.to_array()}))});
    }
    result["robot_feet"] = json!(world.query::<(Entity,&crate::robots::Foot,&GlobalTransform)>().iter(world)
        .map(|(e,f,t)|json!({"entity":e.to_bits(),"robot":f.robot,"side":f.side,"position":t.translation().to_array(),
            "openable":world.get_resource::<crate::robots::Robots>().is_some_and(|r|r.passes[f.robot].is_some_and(|p|p.openable(f.side)))
        })).collect::<Vec<_>>());
    let blood = world
        .get_resource::<crate::powerups::PowerupEffects>()
        .is_some_and(|p| p.zombie_blood > 0.);
    result["planes"] = json!(world.query::<(Entity, &crate::quest_recovery::Plane, &Transform)>().iter(world)
        .filter(|(_,p,_)| !p.quest || blood)
        .map(|(e,p,t)|json!({"entity":e.to_bits(),"quest":p.quest,"position":t.translation.to_array()})).collect::<Vec<_>>());
    if let Some(game) = world.get_resource::<crate::assets::GameDefinition>() {
        if let Some(digging) = &game.environment.digging {
            for target in result["targets"].as_array_mut().unwrap() {
                if target["target"]["kind"] == "mound" {
                    if let Some(site) = target["target"]["value"]
                        .as_u64()
                        .and_then(|i| digging.mounds.get(i as usize))
                    {
                        target["zone"] = json!(site.zone);
                    }
                }
            }
        }
        let tuning = &game.tuning;
        result["movement"] = json!({"walk_speed":tuning.walk_speed,"sprint_speed":tuning.sprint_speed,
            "gravity":tuning.gravity,"jump_speed":tuning.jump_speed,"radius":tuning.radius,
            "height":tuning.standing_height,"friction":tuning.physics.friction,
            "stop_speed":tuning.physics.stop_speed,"acceleration":tuning.physics.ground_acceleration,
            "step_down":tuning.physics.step_down,"step_height":tuning.step_height,
            "sprint_toggle":world.resource::<crate::input::ControlSettings>().sprint_mode == crate::input::PressMode::Toggle});
    }
    result["speed"] = json!(world.resource::<Time<Virtual>>().relative_speed_f64());
    if let Some(movie) = world.get_resource::<super::cinematic::Playback>() {
        result["cinematic"] = serde_json::to_value(movie).unwrap();
    }
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
        result["generators"] = json!({"activations":generators.activations,"completions":generators.completions,"defenders_admitted":generators.defenders_admitted,"sites":generators.sites.iter().map(|(id,s)|(&id.0,json!({"state":format!("{:?}",s.state),"progress":s.progress}))).collect::<std::collections::BTreeMap<_,_>>()});
    }
    if let Some(tank) = world.get_resource::<crate::tank::Tank>() {
        let pose = tank.entity.and_then(|e| world.get::<Transform>(e));
        let model_scale = tank
            .entity
            .and_then(|e| world.get::<Children>(e))
            .and_then(|children| {
                children.iter().find_map(|child| {
                    world.get::<WorldAssetRoot>(child)?;
                    let imported_root = *world.get::<Children>(child)?.first()?;
                    world
                        .get::<GlobalTransform>(imported_root)
                        .map(|pose| pose.to_scale_rotation_translation().0.to_array())
                })
            });
        result["tank"] = json!({"entity":tank.entity.map(Entity::to_bits),"phase":format!("{:?}",tank.phase),"station":tank.station,"trips":tank.trips,"timer":tank.timer,"position":pose.map(|p|p.translation.to_array()),"rotation":pose.map(|p|p.rotation.to_array()),"model_scale":model_scale});
    }
    result
}

pub(super) fn aim_point(boxes: &crate::hit_regions::Hitboxes) -> Option<Vec3> {
    use crate::weapons::HitRegion;
    [HitRegion::UpperTorso, HitRegion::MidTorso, HitRegion::Torso]
        .into_iter()
        .find_map(|region| {
            boxes
                .volumes
                .iter()
                .find(|v| v.region == region)
                .map(|v| (v.start + v.end) * 0.5)
        })
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
