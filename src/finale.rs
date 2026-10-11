use crate::{gameplay::{GameStep,SessionEntity},life::{LifeState,Vitality},quests::{MainQuest,QuestStage},GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource,Default)]
pub struct Finale { pub fists: u32, pub souls: u32, pub portal_open: bool, pub drone_entered: bool, drone_ascending: bool }
#[derive(Component)]
struct FistSoul { spent: bool, lights: [Entity;2] }
#[derive(Component)]
struct FistReward;
#[derive(Component)]
struct FinalPortal;
pub struct FinalePlugin;
impl Plugin for FinalePlugin {
    fn build(&self,app:&mut App){app.init_resource::<Finale>()
        .add_systems(OnEnter(GameState::PreparingShaders),setup.in_set(crate::session::SessionSetup::World))
        .add_systems(FixedUpdate,fists.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)))
        .add_systems(FixedUpdate,portal.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)))
        .add_systems(FixedUpdate,enter.in_set(GameStep::Interactions).after(crate::quest_recovery::recover).run_if(in_state(GameState::Playing)));}
}
fn setup(mut commands:Commands,mut finale:ResMut<Finale>){*finale=default();commands.spawn((SessionEntity,FinalPortal,Transform::from_translation(Vec3::new(262.636,-8.5344,200.8124)),Visibility::Hidden));}
fn fists(mut commands:Commands,mut finale:ResMut<Finale>,mut quest:ResMut<MainQuest>,mut applied:MessageReader<crate::damage::DamageApplied>,game:Res<crate::assets::GameDefinition>,server:Res<AssetServer>,
    unmarked:Query<(Entity,&crate::rounds::SpawnZone,&Vitality),(With<crate::enemies::Zombie>,Without<FistSoul>)>,mut souls:Query<(&mut FistSoul,&Transform)>,
    mut players:Query<(&Transform,&mut crate::rituals::IronFists),With<crate::player::Player>>,rewards:Query<(Entity,&Transform),With<FistReward>>){
    if !quest.enabled || quest.stage!=QuestStage::UpgradeFists {applied.clear();return;}
    for (entity,zone,health) in &unmarked{if zone.0.0!="zone.ug_bottom_zone" || health.state()!=LifeState::ActorAlive {continue;}
        let lights=[-0.35,0.35].map(|x|commands.spawn((PointLight{color:Color::srgb(0.2,0.8,1.),intensity:18000.,range:1.5,shadows_enabled:false,..default()},Transform::from_xyz(x,0.1,0.),ChildOf(entity))).id());commands.entity(entity).insert(FistSoul{spent:false,lights});
    }
    for hit in applied.read(){if hit.impact.source.id()!="weapon.one_inch_punch_zm" {continue;}let Ok((mut soul,pose))=souls.get_mut(hit.impact.target)else{continue;};if soul.spent || finale.fists>=20{continue;}soul.spent=true;for light in soul.lights{commands.entity(light).despawn();}finale.fists+=1;
        if finale.fists==20{let reward=commands.spawn((SessionEntity,FistReward,Transform::from_translation(pose.translation+Vec3::Y*1.27),Visibility::Inherited)).id();if let Some(scene)=game.content.presentations.iter().filter_map(|p|p.scene.as_ref()).find(|s|s.contains("p6_zm_tm_tablet_lod0")){commands.entity(reward).insert(WorldAssetRoot(server.load(scene.clone())));}}
    }
    for (entity,pose) in &rewards{for (player,mut fists) in &mut players{if player.translation.distance(pose.translation)<1.5{fists.unlocked=true;fists.upgraded=true;commands.entity(entity).despawn();quest.transition(QuestStage::UpgradeFists,QuestStage::Finale);}}}
}
fn portal(mut commands:Commands,mut finale:ResMut<Finale>,mut quest:ResMut<MainQuest>,upgrades:Res<crate::staff_upgrades::StaffUpgrades>,mut deaths:MessageReader<crate::damage::EnemyDeath>,enemies:Query<&Transform,With<crate::enemies::Zombie>>,
    mut drones:Query<(Entity,&mut crate::buildables::Drone,&Transform)>,mut buildables:ResMut<crate::buildables::BuildableState>){
    if !quest.enabled {deaths.clear();return;}
    if quest.stage==QuestStage::Finale && upgrades.inserted.into_iter().all(|inserted|inserted){for death in deaths.read(){if death.0.can_supply_soul() && enemies.get(death.0.impact.target).is_ok_and(|p|(p.translation-Vec3::new(262.636,-9.2456,200.8124)).with_y(0.).length_squared()<45.72_f32.powi(2)){finale.souls=(finale.souls+1).min(100);}}}else{deaths.clear();}
    if finale.souls>=100 && !finale.portal_open{finale.portal_open=true;quest.transition(QuestStage::Finale,QuestStage::Complete);crate::audio::emit(&mut commands,"vox_sam_generic_encourage_0",crate::audio::SoundSource::World(Vec3::new(262.636,-8.5344,200.8124)));}
    if finale.portal_open && !finale.drone_entered {for (entity,mut drone,pose) in &mut drones{let entrance=Vec3::new(262.636,-6.2992,200.8124);if !finale.drone_ascending && pose.translation.distance(entrance)>=12.7{continue;}drone.returning=false;drone.quest_goal=Some(if finale.drone_ascending{Vec3::new(262.636,7.9248,200.8124)}else{entrance});if pose.translation.distance(drone.quest_goal.unwrap())<0.5{if finale.drone_ascending{commands.entity(entity).despawn();buildables.drone=None;finale.drone_entered=true;}else{finale.drone_ascending=true;}}}}
}
fn enter(input:Res<crate::input::GameInput>,settings:Res<crate::input::ControlSettings>,finale:Res<Finale>,mut focus:ResMut<crate::interactions::InteractionFocus>,mut next:ResMut<NextState<GameState>>,
    targets:Query<(Entity,&Transform),With<FinalPortal>>,players:Query<(&crate::player::Player,&Transform,&Vitality),Without<FinalPortal>>){
    if !finale.drone_entered || focus.target.is_some(){return;}let Ok((player,pose,health))=players.single()else{return;};if health.state()!=LifeState::Standing{return;}
    for (entity,target) in &targets{let delta=target.translation-pose.translation;let forward=Quat::from_euler(EulerRot::YXZ,player.yaw,player.pitch,0.)*Vec3::NEG_Z;if delta.length()<2.54 && forward.dot(delta.normalize_or_zero())>0.5{focus.target=Some(entity);focus.hold_seconds=0.;focus.prompt=format!("Press {} to enter the final portal",settings.label(crate::input::Action::Interact));if input.use_held{next.set(GameState::Ending);focus.accepted+=1;}}}
}
