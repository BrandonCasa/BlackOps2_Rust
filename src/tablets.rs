use crate::{gameplay::{GameStep, SessionEntity}, input::GameInput, interactions::InteractionFocus, life::{LifeState, Vitality}, player::Player, weapons::{WeaponAction, WeaponInventory}, GameState};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Component, Default)]
pub struct Tablet { pub stage: u8, pub souls: u32, pub reclean: bool, reward_delay: f32 }
#[derive(Component)]
pub(crate) enum Target { Bunker, Church }
#[derive(Component)]
struct TabletModel { index: usize, church: bool }
#[derive(Component, Default)]
struct Barrage { elapsed: f32, waiting: f32, shots: usize, completed: bool }
#[derive(Message, Clone, Copy)]
pub struct BeaconStrike { pub owner: Entity, pub point: Vec3 }
#[derive(Resource, Default)]
pub(crate) struct TabletInput { held: bool }
pub struct TabletPlugin;
impl Plugin for TabletPlugin {
    fn build(&self, app: &mut App) {
        app.register_required_components::<Player, Tablet>().init_resource::<TabletInput>().add_message::<BeaconStrike>()
            .add_systems(OnEnter(GameState::PreparingShaders), setup.in_set(crate::session::SessionSetup::World))
            .add_systems(FixedUpdate, track.in_set(GameStep::Rewards).after(crate::damage::reward_deaths).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, interact.in_set(GameStep::Interactions).after(crate::staff_upgrades::interact).run_if(in_state(GameState::Playing)))
            .add_systems(FixedUpdate, bombard.in_set(GameStep::Combat).after(crate::equipment::flight).before(crate::equipment::resolve_blasts).run_if(in_state(GameState::Playing)));
    }
}
fn setup(mut commands: Commands, mut game: ResMut<crate::assets::GameDefinition>, server: Res<AssetServer>, mut input: ResMut<TabletInput>) {
    *input = default();
    if let Some(mut tuning) = game.content.weapons.iter().find_map(|w| w.throwable.clone()) && let Some(beacon) = game.content.weapons.iter_mut().find(|w| w.id.0 == "weapon.beacon_zm") {
        tuning.slot = crate::equipment::ThrowableSlot::Tactical; tuning.initial = 2; tuning.capacity = 2; tuning.round_award = 0;
        tuning.speed = 20.32; tuning.fuse_seconds = 10.; tuning.cook_in_hand = false; tuning.sticky = false; beacon.throwable = Some(tuning);
    }
    commands.spawn((SessionEntity, Target::Bunker, Transform::from_translation(Vec3::new(-3.5814,-8.2296,-113.4618))));
    commands.spawn((SessionEntity, Target::Church, Transform::from_translation(Vec3::new(23.368,9.0678,67.056))));
    let scene = game.content.presentations.iter().filter_map(|p| p.scene.as_ref()).find(|p| p.contains("p6_zm_tm_tablet_muddy_lod0"));
    for (index, x) in [-3.6957,-3.4417,-4.0005,-3.1369].into_iter().enumerate() {
        let mut entity = commands.spawn((SessionEntity, TabletModel { index, church: false }, Transform::from_translation(Vec3::new(x,-8.1915,-113.3856)), Visibility::Inherited));
        if let Some(scene) = scene { entity.insert(WorldAssetRoot(server.load(scene.clone()))); }
    }
    let mut entity = commands.spawn((SessionEntity, TabletModel { index: 3, church: true }, Transform::from_translation(Vec3::new(23.3045,8.8773,67.3481)), Visibility::Hidden));
    if let Some(scene) = scene { entity.insert(WorldAssetRoot(server.load(scene.clone()))); }
}
fn track(mut commands: Commands, time: Res<Time<Fixed>>, mut deaths: MessageReader<crate::damage::EnemyDeath>, game: Res<crate::assets::GameDefinition>, spatial: SpatialQuery,
    volumes: Query<&crate::environment::triggers::MapTrigger>, enemies: Query<&Transform, With<crate::enemies::Zombie>>,
    mut players: Query<(&mut Tablet, &Vitality, &crate::environment::TerrainMotion), With<Player>>, mut models: Query<(&TabletModel, &mut Visibility)>) {
    let Ok((mut tablet, health, motion)) = players.single_mut() else { return; };
    if health.state() == LifeState::HumanTerminal && tablet.stage < 7 { *tablet = default(); }
    if tablet.stage == 4 && motion.muddy { tablet.stage = 1; tablet.reclean = true; crate::audio::emit(&mut commands, "zmb_squest_oiptablet_dirtied", crate::audio::SoundSource::World(Vec3::ZERO)); }
    tablet.reward_delay = (tablet.reward_delay - time.delta_secs()).max(0.);
    for death in deaths.read() {
        if !matches!(tablet.stage, 2 | 5) || !death.0.can_supply_soul() || !matches!(death.0.impact.category, crate::origins::DamageCategory::Melee) { continue; }
        let Ok(pose) = enemies.get(death.0.impact.target) else { continue; };
        let name = if tablet.stage == 2 { "oneinchpunch_church_volume" } else { "oneinchpunch_bunker_volume" };
        let inside = spatial.shape_intersections(&Collider::sphere(0.3), pose.translation, Quat::IDENTITY, &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER)).iter().filter_map(|e| volumes.get(*e).ok()).any(|v| game.environment.triggers.iter().find(|t| t.id == v.0).is_some_and(|t| t.source.get("targetname").is_some_and(|n| n == name)));
        if inside { tablet.souls += 1; if tablet.souls >= 20 { tablet.stage += 1; tablet.souls = 0; if tablet.stage == 6 { tablet.reward_delay = 4.; } } }
    }
    for (model, mut visibility) in &mut models { *visibility = if model.church && matches!(tablet.stage,2|3) || !model.church && (model.index != 3 || matches!(tablet.stage,0|5|6)) { Visibility::Inherited } else { Visibility::Hidden }; }
}
pub(crate) fn interact(mut commands: Commands, input: Res<GameInput>, settings: Res<crate::input::ControlSettings>, mut latch: ResMut<TabletInput>, mut focus: ResMut<InteractionFocus>, game: Res<crate::assets::GameDefinition>, spatial: SpatialQuery,
    targets: Query<(Entity, &Target, &Transform), Without<Player>>, mut players: Query<(Entity, &Player, &Transform, &Vitality, &WeaponInventory, &mut Tablet, &mut crate::equipment::TacticalInventory, Has<crate::perks::PerkDrinking>)>) {
    if !input.use_held { latch.held = false; }
    let Ok((owner, player, pose, health, inventory, mut tablet, mut tactical, drinking)) = players.single_mut() else { return; };
    if focus.target.is_some() || health.state() != LifeState::Standing { return; }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5); let forward = Quat::from_euler(EulerRot::YXZ,player.yaw,player.pitch,0.) * Vec3::NEG_Z;
    let selected = targets.iter().filter_map(|(e,t,p)| { let delta = p.translation-eye; let distance=delta.length(); let direction=Dir3::new(delta).ok()?;
        (distance<2.54 && forward.dot(*direction)>0.7 && spatial.cast_ray(eye,direction,(distance-0.1).max(0.),true,&crate::environment::solids([owner])).is_none()).then_some((e,t,distance)) }).min_by(|a,b|a.2.total_cmp(&b.2));
    let Some((entity,target,_))=selected else{return;};
    focus.target=Some(entity);focus.hold_seconds=0.;focus.progress=0.;
    let action=match (target,tablet.stage) { (Target::Bunker,0)=>"take muddy tablet",(Target::Church,1)=>"place tablet in basin",(Target::Church,3)=>"take cleansed tablet",(Target::Bunker,4)=>"return cleansed tablet",(Target::Bunker,6)=>"claim G-Strike",_=>"tablet ritual in progress" };
    focus.prompt=format!("Press {} to {action}",settings.label(crate::input::Action::Interact));
    if !input.use_held || latch.held || drinking || inventory.action!=WeaponAction::Ready {return;}
    match (target,tablet.stage) {
        (Target::Bunker,0)=>tablet.stage=1,
        (Target::Church,1)=>{tablet.stage=if tablet.reclean {3}else{2};tablet.reclean=false;},
        (Target::Church,3)=>tablet.stage=4,
        (Target::Bunker,4)=>tablet.stage=5,
        (Target::Bunker,6) if tablet.reward_delay<=0.=>{let Some(weapon)=game.content.weapons.iter().find(|w|w.id.0=="weapon.beacon_zm")else{return;};if tactical.receive(weapon).is_err(){return;}tablet.stage=7;},
        _=>return,
    }
    latch.held=true;focus.accepted+=1;crate::audio::emit(&mut commands,"zmb_squest_oiptablet_pickup",crate::audio::SoundSource::attached(owner));
}
fn bombard(mut commands: Commands, time: Res<Time<Fixed>>, robots: Res<crate::robots::Robots>, mut grenades: Query<(Entity, &mut crate::equipment::Grenade, &Transform, Option<&mut Barrage>)>,
    enemies: Query<(Entity,&Transform,&Vitality),With<crate::enemies::Zombie>>, mut damage: ResMut<crate::damage::DamageQueue>, mut strikes: MessageWriter<BeaconStrike>) {
    for (entity,mut grenade,pose,barrage) in &mut grenades {
        if grenade.definition.0!="weapon.beacon_zm" || !grenade.settled() {continue;}
        let Some(mut barrage)=barrage else{grenade.remaining=100.;commands.entity(entity).insert(Barrage::default());continue;};
        grenade.remaining=100.;
        if barrage.elapsed==0. && robots.passes.iter().all(Option::is_none) { barrage.waiting += time.delta_secs(); if barrage.waiting >= 10. { commands.entity(entity).despawn(); } continue; }
        barrage.elapsed+=time.delta_secs();
        let offsets=[Vec3::ZERO,Vec3::new(-1.8288,0.,-1.8288),Vec3::new(1.8288,0.,-1.8288),Vec3::new(1.8288,0.,1.8288),Vec3::new(-1.8288,0.,1.8288)];
        let due=if barrage.shots==0{0.5}else{1.5+(barrage.shots-1)as f32*0.25};
        if barrage.shots<5 && barrage.elapsed>=due {
            let center=pose.translation+offsets[barrage.shots]; if barrage.shots == 0 { strikes.write(BeaconStrike{owner:grenade.owner,point:pose.translation}); } barrage.shots+=1;
            for (enemy,p,health) in &enemies {let distance=p.translation.distance(center);if health.state()==LifeState::ActorAlive && distance<=5.08 {damage.0.push(crate::damage::Impact{attacker:grenade.owner,target:enemy,source:crate::damage::ImpactSource::Weapon(grenade.definition.clone()),category:crate::origins::DamageCategory::Explosive,region:crate::weapons::HitRegion::None,amount:(8000.-distance/5.08*1000.)as u32});}}
            crate::audio::emit(&mut commands,"zmb_robot_mortar_explode",crate::audio::SoundSource::World(center));
        }
        if barrage.elapsed>=8.5 && !barrage.completed {barrage.completed=true;commands.entity(entity).despawn();}
    }
}
