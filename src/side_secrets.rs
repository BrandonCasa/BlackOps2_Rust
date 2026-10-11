use crate::{assets::GameDefinition,gameplay::{GameStep,SessionEntity},player::Player,GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Component,Clone,Copy)]
struct Secret(Target);
#[derive(Clone,Copy)]
enum Target { Rock(usize), Radio(usize), Log(usize), Floor(usize) }
#[derive(Component)]
struct Scare;
#[derive(Resource,Default)]
pub struct Secrets {
    pub rocks: [bool;3], pub radios: [bool;3], pub logs: [bool;6], pub floors: [bool;4],
    pub poster_removed: bool, pub scared: bool, poster_damage: u32, aiming: f32, scare_seconds: f32,
    music: f32, latched: bool, voice: std::collections::VecDeque<String>, voice_seconds: f32, voice_origin: Vec3,
}
pub struct SecretPlugin;
impl Plugin for SecretPlugin {
    fn build(&self,app:&mut App){app.init_resource::<Secrets>()
        .add_systems(OnEnter(GameState::PreparingShaders),setup.in_set(crate::session::SessionSetup::World))
        .add_systems(FixedUpdate,(tick,poster).in_set(GameStep::Rewards).run_if(in_state(GameState::Playing)))
        .add_systems(FixedUpdate,interact.in_set(GameStep::Interactions).after(crate::side_rewards::collect).run_if(in_state(GameState::Playing)));}
}
fn setup(mut commands:Commands,mut state:ResMut<Secrets>){*state=default();
    commands.spawn((SessionEntity,Secret(Target::Log(4)),Transform::from_translation(Vec3::from_array([-162.840162, 4.0132, 167.189658]))));
    commands.spawn((SessionEntity,Secret(Target::Log(3)),Transform::from_translation(Vec3::from_array([-166.955724, 4.0005, 166.742872]))));
    commands.spawn((SessionEntity,Secret(Target::Log(5)),Transform::from_translation(Vec3::from_array([-149.065234, 4.0132, 163.519612]))));
    commands.spawn((SessionEntity,Secret(Target::Log(1)),Transform::from_translation(Vec3::from_array([62.516766, -8.636, -131.839716]))));
    commands.spawn((SessionEntity,Secret(Target::Log(0)),Transform::from_translation(Vec3::from_array([55.78475, -6.81355, -115.539266]))));
    commands.spawn((SessionEntity,Secret(Target::Log(2)),Transform::from_translation(Vec3::from_array([-17.872202, -6.2992, -59.365388]))));
    commands.spawn((SessionEntity,Secret(Target::Radio(0)),Transform::from_translation(Vec3::from_array([-140.779246, 3.8227, 163.552632]))));
    commands.spawn((SessionEntity,Secret(Target::Radio(1)),Transform::from_translation(Vec3::from_array([243.573808, -11.7729, 206.118968]))));
    commands.spawn((SessionEntity,Secret(Target::Radio(2)),Transform::from_translation(Vec3::from_array([3.340608, -12.7127, -4.72313]))));
    commands.spawn((SessionEntity,Secret(Target::Rock(0)),Transform::from_translation(Vec3::from_array([68.128642, -7.679741, -113.18621]))));
    commands.spawn((SessionEntity,Secret(Target::Rock(1)),Transform::from_translation(Vec3::from_array([18.314492, 7.264044, 2.227753]))));
    commands.spawn((SessionEntity,Secret(Target::Rock(2)),Transform::from_translation(Vec3::from_array([-17.120819, -2.857068, -64.431418]))));
    commands.spawn((SessionEntity,Secret(Target::Floor(0)),Transform::from_translation(Vec3::from_array([55.0672, -7.3406, -117.2718]))));
    commands.spawn((SessionEntity,Secret(Target::Floor(1)),Transform::from_translation(Vec3::from_array([55.118, -7.3406, -125.8062]))));
    commands.spawn((SessionEntity,Secret(Target::Floor(2)),Transform::from_translation(Vec3::from_array([-62.4586, 6.1722, -4.4704]))));
    commands.spawn((SessionEntity,Secret(Target::Floor(3)),Transform::from_translation(Vec3::from_array([-70.9168, 6.1722, -4.445]))));
}
fn interact(mut commands:Commands,input:Res<crate::input::GameInput>,settings:Res<crate::input::ControlSettings>,mut state:ResMut<Secrets>,mut focus:ResMut<crate::interactions::InteractionFocus>,spatial:SpatialQuery,sites:Query<(Entity,&Secret,&Transform),Without<Player>>,players:Query<(Entity,&Player,&Transform,&crate::life::Vitality)>){
    if !input.use_held{state.latched=false;}if focus.target.is_some(){return;}
    let Ok((owner,player,pose,health))=players.single()else{return;};if health.state()!=crate::life::LifeState::Standing{return;}
    let eye=pose.translation+Vec3::Y*(player.eye_height-player.height*0.5);let forward=Quat::from_euler(EulerRot::YXZ,player.yaw,player.pitch,0.)*Vec3::NEG_Z;
    let candidate=sites.iter().filter(|(_,secret,_)|match secret.0{Target::Rock(i)=>!state.rocks[i] && state.music<=0.,Target::Radio(i)=>!state.radios[i] && state.music<=0.,Target::Log(i)=>!state.logs[i],Target::Floor(i)=>!state.floors[i] && state.music<=0. && player.prone && (i<2 || state.floors[..2].iter().all(|v|*v))})
        .filter_map(|(entity,secret,location)|{let delta=location.translation-eye;let direction=Dir3::new(delta).ok()?;(delta.length()<2. && forward.dot(*direction)>0.8 && spatial.cast_ray(eye,direction,(delta.length()-0.1).max(0.),true,&crate::environment::solids([owner])).is_none()).then_some((entity,secret.0,location.translation,delta.length()))}).min_by(|a,b|a.3.total_cmp(&b.3));
    let Some((entity,target,position,_))=candidate else{return;};focus.target=Some(entity);focus.prompt=format!("Press {} to investigate",settings.label(crate::input::Action::Interact));
    if !input.use_held || state.latched{return;}state.latched=true;focus.accepted+=1;
    let mut song=None;
    match target {
        Target::Rock(i)=>{state.rocks[i]=true;if state.rocks.iter().all(|v|*v){song=Some(("mus_zmb_secret_song",310.));}}
        Target::Radio(i)=>{state.radios[i]=true;if state.radios.iter().all(|v|*v){song=Some(("mus_zmb_secret_song_a7x",352.));}}
        Target::Floor(i)=>{state.floors[i]=true;if state.floors.iter().filter(|v|**v).count()==3{song=Some(("mus_zmb_secret_song_aether",135.));}}
        Target::Log(i)=>{state.logs[i]=true;state.voice_origin=position;state.voice.clear();for part in 1..=if i==1 || i==5{2}else{3}{state.voice.push_back(format!("vox_maxi_audio_log_{}_{}_0",i+1,part));}state.voice_seconds=0.;}
    }
    crate::audio::emit(&mut commands,"zmb_meteor_activate",crate::audio::SoundSource::World(position));
    if let Some((alias,seconds))=song{state.music=seconds;crate::audio::emit(&mut commands,alias,crate::audio::SoundSource::World(position));}
}
fn tick(mut commands:Commands,time:Res<Time<Fixed>>,game:Res<GameDefinition>,server:Res<AssetServer>,zone:Res<crate::access::CurrentZone>,mut state:ResMut<Secrets>,players:Query<(&Player,&Transform,&crate::weapons::WeaponInventory)>,cameras:Query<Entity,With<crate::presentation::ViewModelCamera>>,scares:Query<Entity,With<Scare>>){
    let dt=time.delta_secs();state.music=(state.music-dt).max(0.);state.voice_seconds=(state.voice_seconds-dt).max(0.);
    if state.voice_seconds==0. && let Some(alias)=state.voice.pop_front(){state.voice_seconds=game.audio.aliases.get(&alias).map_or(4.,|a|a.seconds);crate::audio::emit(&mut commands,alias,crate::audio::SoundSource::World(state.voice_origin));}
    let Ok((player,pose,inventory))=players.single()else{return;};
    if state.scare_seconds>0.{state.scare_seconds-=dt;if state.scare_seconds<=0. || inventory.ads<0.9{state.scare_seconds=0.;for entity in &scares{commands.entity(entity).despawn();}}}
    if state.scared{return;}
    let eye=pose.translation+Vec3::Y*(player.eye_height-player.height*0.5);let forward=Quat::from_euler(EulerRot::YXZ,player.yaw,player.pitch,0.)*Vec3::NEG_Z;
    let valid=inventory.ads>0.9 && inventory.active().is_some_and(|w|w.definition.0.contains("dsr50")) && inventory.attachment()!="is" && zone.0.as_ref().is_some_and(|z|z.0.ends_with("nml_18")) && forward.dot((Vec3::new(30.0228,25.4,59.5884)-eye).normalize_or_zero())>0.998;
    state.aiming=if valid{state.aiming+dt}else{0.};if state.aiming<1.25{return;}
    state.scared=true;state.scare_seconds=0.25;crate::audio::emit(&mut commands,"zmb_easteregg_scarydog",crate::audio::SoundSource::World(eye));
    if let Ok(camera)=cameras.single() && let Some(image)=game.content.ui_assets.iter().find(|a|a.id.0.contains("zm_tm_wth_dog") || a.path.contains("zm_tm_wth_dog")){commands.spawn((SessionEntity,Scare,UiTargetCamera(camera),GlobalZIndex(1000),ImageNode::new(server.load(image.path.clone())),Node{position_type:PositionType::Absolute,width:percent(100.),height:percent(100.),..default()}));}
}
fn poster(mut commands:Commands,mut state:ResMut<Secrets>,mut hits:MessageReader<crate::environment::support::EnvironmentalHit>,brushes:Query<(Entity,&crate::environment::brushes::MapBrush)>){
    if state.poster_removed{return;}
    for (entity,brush) in &brushes{if brush.id=="entity.4423f73c.58b0fe1348ff1365bedbfb88.0"{commands.entity(entity).insert((crate::environment::support::ShootableEnvironment,crate::environment::support::EnvironmentOwner::Quest));}}
    for hit in hits.read(){if brushes.get(hit.target).is_ok_and(|(_,brush)|brush.id=="entity.4423f73c.58b0fe1348ff1365bedbfb88.0"){state.poster_damage=state.poster_damage.saturating_add(hit.amount);if state.poster_damage>=1000{state.poster_removed=true;commands.entity(hit.target).insert((Visibility::Hidden,ColliderDisabled));}}}
}
