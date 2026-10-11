use super::*;
use crate::{gameplay::{GameStep,Session},player::Player};
#[derive(Resource,Default)]
struct Narrative {
    round:u32,stage:Option<crate::quests::QuestStage>,entries:u32,evacuations:u32,raid:Option<String>,
    crafted:BTreeSet<String>,discoveries:BTreeSet<String>,queue:std::collections::VecDeque<String>,wait:f32,
    alarms:[bool;3],robot:Option<usize>,ambience:Option<Entity>,
}
pub(super) fn install(app:&mut App){app.init_resource::<Narrative>()
    .add_systems(OnEnter(GameState::PreparingShaders),reset)
    .add_systems(FixedUpdate,events.after(GameStep::Rounds).run_if(in_state(GameState::Playing)));}
fn reset(mut state:ResMut<Narrative>){*state=default();}
fn category(state:&mut Narrative,game:&GameDefinition,session:&Session,event:&str){let prefix=format!("vox_plr_{}_{}",session.character,event);if let Some(alias)=game.audio.aliases.keys().find(|a|a.starts_with(&prefix)){state.queue.push_back(alias.clone());}}
fn events(mut commands:Commands,time:Res<Time<Fixed>>,mut state:ResMut<Narrative>,game:Res<GameDefinition>,session:Res<Session>,quest:Res<crate::quests::MainQuest>,robots:Res<crate::robots::Robots>,raids:Res<crate::generators::GeneratorRaids>,crafting:Res<crate::crafting::Crafting>,focus:Res<crate::interactions::InteractionFocus>,players:Query<(Entity,&Transform),With<Player>>,usables:Query<&crate::interactions::Usable>,stations:Query<&crate::stations::Station>){
    let Ok((player,pose))=players.single()else{return;};
    if state.round!=session.round{
        state.round=session.round;
        if session.round==1{state.queue.push_back(format!("vox_plr_{}_game_start_0",session.character));if session.character==2{state.queue.push_back("vox_plr_2_game_start_1".into());}}
        match session.round{5=>{state.queue.push_back("vox_sam_sam_help_5_0".into());category(&mut state,&game,&session,"hear_samantha_1");},6=>{if session.character==2{category(&mut state,&game,&session,"heroes_confer");}},7=>{state.queue.push_back("vox_sam_hear_samantha_3_0".into());category(&mut state,&game,&session,"hear_samantha_3");},2..=4=>category(&mut state,&game,&session,&format!("during_round_{}",session.round)),_=>{}}
    }
    if state.stage!=Some(quest.stage){if state.stage.is_some() && quest.enabled{state.queue.push_back("vox_sam_generic_encourage_6".into());category(&mut state,&game,&session,"generic_encourage");emit(&mut commands,"mus_event_quest_complete",SoundSource::World(pose.translation));}state.stage=Some(quest.stage);}
    let raid=raids.target.as_ref().map(|id|id.0.clone());if state.raid!=raid{if raid.is_some(){category(&mut state,&game,&session,"recapture_generator_attacked");}state.raid=raid;}
    if robots.entries>state.entries{state.entries=robots.entries;emit(&mut commands,"mus_event_location_robot",SoundSource::World(pose.translation));state.queue.push_back("vox_maxi_purge_robot_0".into());category(&mut state,&game,&session,"enter_robot");}
    if robots.evacuations>state.evacuations{state.evacuations=robots.evacuations;category(&mut state,&game,&session,"exit_robot");}
    let interior=robots.occupancy.map(|o|o.robot);
    if interior!=state.robot{if let Some(entity)=state.ambience.take(){commands.entity(entity).try_despawn();}if interior.is_some(){state.ambience=Some(emit_loop(&mut commands,"amb_robot_head_2d",SoundSource::attached(player)));}state.robot=interior;}

    if let Some(occupancy)=robots.occupancy && occupancy.remaining<5. && occupancy.remaining>5.-time.delta_secs(){emit(&mut commands,"zmb_bot_timeout_alarm",SoundSource::attached(player));state.queue.push_back("vox_maxi_purge_countdown_0".into());}
    for (i,pass) in robots.passes.iter().enumerate(){let warn=pass.is_some_and(|p|p.delay<=0. && p.clock>=6.);if warn && !state.alarms[i]{emit(&mut commands,"zmb_robot_pre_stomp_a",SoundSource::World(pose.translation));}state.alarms[i]=warn;}
    for recipe in &crafting.crafted{if state.crafted.insert(recipe.clone()){if recipe=="dieseldrone"{state.queue.push_back("vox_maxi_maxis_drone_1_0".into());state.queue.push_back("vox_maxi_maxis_drone_4_0".into());}else{category(&mut state,&game,&session,"buildable_built");}}}
    if let Some(target)=focus.target{
        if let Ok(usable)=usables.get(target){let event=match usable.source.action{crate::origins::content::InteractionAction::Generator{..}=>Some("generator_find"),crate::origins::content::InteractionAction::Perk{..}=>Some("perk_machine_find"),_=>None};if let Some(event)=event && state.discoveries.insert(event.into()){category(&mut state,&game,&session,event);}}
        if matches!(stations.get(target),Ok(crate::stations::Station::Box(_))) && state.discoveries.insert("box".into()){category(&mut state,&game,&session,"respond_maxis");}
    }
    state.wait=(state.wait-time.delta_secs()).max(0.);if state.wait==0. && let Some(alias)=state.queue.pop_front(){state.wait=game.audio.aliases.get(&alias).map_or(1.,|a|a.seconds+1.);emit(&mut commands,alias,SoundSource::attached(player));}
}
