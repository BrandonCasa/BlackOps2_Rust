use crate::{gameplay::{GameStep, SessionEntity}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, quests::{MainQuest, QuestStage}, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource, Default)]
pub struct QuestProgress { pub seal_open: bool, pub drone_consumed: bool, pub encounter_spawned: u32, pub encounter_killed: u32, drone_step: Option<usize>, spawn_timer: f32, latched: bool }
#[derive(Component)]
pub(crate) enum Target { Staff { index: usize, robot: Option<usize> }, Button(usize) }
pub struct MainQuestPlugin;
impl Plugin for MainQuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestProgress>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, interact.in_set(GameStep::Interactions).after(crate::tablets::interact).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, progress.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut progress: ResMut<QuestProgress>, game: Res<crate::assets::GameDefinition>, server: Res<AssetServer>) {
    *progress = default();
    let pedestals = [(0,None,Vec3::new(-0.015748,-19.0373,-4.283202)),(1,Some(2),Vec3::new(-144.878552,4.0894,170.043602)),(2,Some(0),Vec3::new(-171.700952,4.0767,170.056302)),(3,Some(1),Vec3::new(-158.086552,4.0894,170.056302))];
    for (index,robot,point) in pedestals {
        let mut entity=commands.spawn((SessionEntity,Target::Staff{index,robot},Transform::from_translation(point),Visibility::Hidden));
        if let Some(scene)=game.content.presentations.iter().filter_map(|p|p.scene.as_ref()).find(|s|s.contains("p6_zm_tm_staff_holder_lod0")){entity.insert(WorldAssetRoot(server.load(scene.clone())));}
    }
    for (robot,point) in [Vec3::new(-171.704,4.8768,170.7261),Vec3::new(-158.0896,4.8768,170.688),Vec3::new(-144.8816,4.8768,170.7007)].into_iter().enumerate(){
        let mut entity=commands.spawn((SessionEntity,Target::Button(robot),Transform::from_translation(point),Visibility::Hidden));
        if let Some(scene)=game.content.presentations.iter().filter_map(|p|p.scene.as_ref()).find(|s|s.contains("p_rus_alarm_button_lod0")){entity.insert(WorldAssetRoot(server.load(scene.clone())));}
    }
}
fn progress(mut commands: Commands,time: Res<Time<Fixed>>,mut state: ResMut<QuestProgress>,mut quest: ResMut<MainQuest>, mut strikes: MessageReader<crate::tablets::BeaconStrike>,mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut drones: Query<(Entity,&mut crate::buildables::Drone,&Transform)>, panzers: Query<(&crate::panzer::Panzer,&Vitality)>, mut buildables: ResMut<crate::buildables::BuildableState>,
    game: Res<crate::assets::GameDefinition>,server: Res<AssetServer>,brushes: Query<(Entity,&crate::environment::brushes::MapBrush)>,mut targets: Query<(&Target,&mut Visibility)>) {
    for (target,mut visible) in &mut targets { *visible=if quest.enabled && match target {Target::Staff{index,..}=>quest.stage==QuestStage::PlaceStaffs && !quest.placed.contains(&crate::staff_upgrades::ELEMENTS[*index]),Target::Button(_)=>quest.stage==QuestStage::BreakSeal}{Visibility::Inherited}else{Visibility::Hidden}; }
    if !quest.enabled { strikes.clear(); deaths.clear(); return; }
    if quest.stage==QuestStage::PlaceStaffs && quest.placed.len()==4 { quest.transition(QuestStage::PlaceStaffs,QuestStage::BreakSeal);quest.placed.clear(); }
    for strike in strikes.read() {
        if !quest.strike_authorized() || strike.point.distance_squared(Vec3::new(-79.248,4.5974,-28.3464))>5.08_f32.powi(2){continue;}
        state.seal_open=true;quest.transition(QuestStage::BreakSeal,QuestStage::PanzerEncounter);quest.hold_round(QuestStage::PanzerEncounter);
        for (entity,brush) in &brushes {if game.environment.brush_entities.iter().find(|b|b.id==brush.id).is_some_and(|b|b.source.get("targetname").is_some_and(|n|n=="easter_mechzombie_spawn")){commands.entity(entity).insert((Visibility::Hidden,ColliderDisabled));}}
        crate::audio::emit(&mut commands,"zmb_squest_robot_floor_collapse",crate::audio::SoundSource::World(strike.point));
    }
    if quest.stage!=QuestStage::PanzerEncounter { deaths.clear(); return; }
    if !state.drone_consumed {
        let path=[Vec3::new(-70.7136,8.9408,-15.6464),Vec3::new(-79.4512,6.9088,-28.2448),Vec3::new(-79.4512,0.2032,-28.2448)];
        for (entity,mut drone,pose) in &mut drones {
            if state.drone_step.is_none() && pose.translation.distance(path[0])<12.7 {state.drone_step=Some(0);}
            if let Some(step)=state.drone_step {
                drone.returning=false;drone.quest_goal=Some(path[step]);
                if pose.translation.distance(path[step])<0.5 {
                    if step<2 {state.drone_step=Some(step+1);}else{commands.entity(entity).despawn();buildables.drone=None;buildables.drone_cooldown=f32::MAX;state.drone_consumed=true;state.drone_step=None;}
                }
            }
        }
    }
    for death in deaths.read(){if panzers.get(death.0.impact.target).is_ok_and(|(panzer,_)|panzer.quest){state.encounter_killed+=1;}}
    if state.drone_consumed && state.encounter_spawned<8 {
        state.spawn_timer-=time.delta_secs();
        let alive=panzers.iter().filter(|(panzer,health)|panzer.quest && health.state()==LifeState::ActorAlive).count();
        if state.spawn_timer<=0. && alive<4 {
            let offsets=[Vec3::X,Vec3::Z,Vec3::NEG_X,Vec3::NEG_Z];let position=Vec3::new(-79.4512,3.8354,-28.2448)+offsets[state.encounter_spawned as usize%4]*3.;
            crate::panzer::spawn(&mut commands,&game,&server,position,22500,true);state.encounter_spawned+=1;state.spawn_timer=0.75;
        }
    }
    if state.encounter_killed>=8 {quest.transition(QuestStage::PanzerEncounter,QuestStage::RecoverDrone);}
}
pub(crate) fn interact(mut commands: Commands,input: Res<GameInput>,settings: Res<crate::input::ControlSettings>,mut state: ResMut<QuestProgress>,mut quest: ResMut<MainQuest>,robots: Res<crate::robots::Robots>,mut focus: ResMut<InteractionFocus>,game: Res<crate::assets::GameDefinition>,spatial: SpatialQuery,
    targets: Query<(Entity,&Target,&Transform),Without<Player>>,mut players: Query<(Entity,&Player,&Transform,&Vitality,&mut WeaponInventory,Has<crate::perks::PerkDrinking>)>) {
    if !input.use_held {state.latched=false;}
    if !quest.enabled || focus.target.is_some(){return;}
    let Ok((owner,player,pose,health,mut inventory,drinking))=players.single_mut()else{return;};if health.state()!=LifeState::Standing{return;}
    let eye=pose.translation+Vec3::Y*(player.eye_height-player.height*0.5);let forward=Quat::from_euler(EulerRot::YXZ,player.yaw,player.pitch,0.)*Vec3::NEG_Z;
    let selected=targets.iter().filter(|(_,target,_)|match target{Target::Staff{index,robot}=>quest.stage==QuestStage::PlaceStaffs && !quest.placed.contains(&crate::staff_upgrades::ELEMENTS[*index]) && robot.is_none_or(|i|robots.occupancy.is_some_and(|o|o.robot==i && o.player==owner)),Target::Button(i)=>quest.stage==QuestStage::BreakSeal && robots.occupancy.is_some_and(|o|o.robot==*i && o.player==owner)})
        .filter_map(|(e,t,p)|{let delta=p.translation+Vec3::Y*0.6-eye;let distance=delta.length();let direction=Dir3::new(delta).ok()?;(distance<2.54 && forward.dot(*direction)>0.7 && spatial.cast_ray(eye,direction,(distance-0.1).max(0.),true,&crate::environment::solids([owner])).is_none()).then_some((e,t,distance))}).min_by(|a,b|a.2.total_cmp(&b.2));
    let Some((entity,target,_))=selected else{return;};focus.target=Some(entity);focus.hold_seconds=0.;focus.progress=0.;
    let key=settings.label(crate::input::Action::Interact);focus.prompt=match target{Target::Staff{..}=>format!("Press {key} to place upgraded staff"),Target::Button(_)=>format!("Press {key} to authorize G-Strike")};
    if !input.use_held || state.latched || drinking || inventory.action!=WeaponAction::Ready{return;}
    match target {
        Target::Staff{index,..}=>{let id=crate::staff_upgrades::WEAPONS[*index].replace("_zm","_upgraded_zm");if !inventory.active().is_some_and(|w|w.definition.0==id) || inventory.deposit_active().is_err(){return;}quest.placed.insert(crate::staff_upgrades::ELEMENTS[*index]);if inventory.active().is_none() && let Some(fists)=game.content.weapons.iter().find(|w|w.id.0=="weapon.zombie_fists_zm"){let _=inventory.grant(fists);}},
        Target::Button(_)=>{if !robots.triple_active || !quest.authorize_strike(){focus.prompt="Requires synchronized robot pass".into();return;}},
    }
    state.latched=true;focus.accepted+=1;crate::audio::emit(&mut commands,"zmb_squest_robot_button_activate",crate::audio::SoundSource::attached(entity));
}
