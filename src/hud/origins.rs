use crate::{GameState,player::Player};
use bevy::prelude::*;
#[derive(Component)]
struct InventoryReadout;
#[derive(Component)]
struct PerkRow;
pub struct OriginsHudPlugin;
impl Plugin for OriginsHudPlugin {
    fn build(&self,app:&mut App){app.add_systems(Startup,setup.after(crate::presentation::first_person_light))
        .add_systems(Update,(update,icons).run_if(in_state(GameState::Playing)));}
}
fn setup(mut commands:Commands,cameras:Query<Entity,With<crate::presentation::ViewModelCamera>>){
    let Ok(camera)=cameras.single()else{return;};
    commands.spawn((InventoryReadout,super::PlayerOverlay,UiTargetCamera(camera),Text::new(""),TextFont{font_size:FontSize::Px(18.),..default()},TextColor(Color::WHITE),Node{position_type:PositionType::Absolute,left:percent(3.),top:percent(16.),width:percent(80.),..default()}));
    commands.spawn((PerkRow,super::PlayerOverlay,UiTargetCamera(camera),Node{position_type:PositionType::Absolute,left:percent(3.),bottom:percent(34.),column_gap:px(6.),..default()}));
}
fn update(
    input:Res<crate::input::GameInput>,raids:Res<crate::generators::GeneratorRaids>,
    (crafting,parts):(Res<crate::crafting::Crafting>,Res<crate::environment::parts::Parts>),
    (upgrades,quest,rituals):(Res<crate::staff_upgrades::StaffUpgrades>,Res<crate::quests::MainQuest>,Res<crate::rituals::Rituals>),
    players:Query<(&crate::weapons::WeaponInventory,&crate::crafting::BuildableInventory,&crate::environment::digging::DigTools),With<Player>>,
    mut readouts:Query<(&mut Text,&mut Visibility),With<InventoryReadout>>,
){
    let Ok((weapons,buildable,dig))=players.single()else{return;};
    for (mut text,mut visible) in &mut readouts{
        *visible=Visibility::Inherited;
        let mut value=format!("{}{}{}  {}{}",if dig.shovel{"Shovel "}else{""},if dig.golden{"Golden "}else{""},if dig.helmet{"Golden Helmet"}else{""},buildable.equipped.as_deref().unwrap_or(""),if weapons.charge_level>0{format!("  Staff charge {}",weapons.charge_level)}else{String::new()});
        if let Some(target)=&raids.target{value.push_str(&format!("\nGENERATOR ATTACK: {}",target.0));}
        if input.scoreboard{value.push_str(&format!("\nParts {}\nRecords {}\nStaffs {}\nChallenges: {}/115 headshots  {}/6 generators  {}/30000 points  {}/4 chests\nQuest: {:?}",parts.pieces.iter().filter(|(_,p)|matches!(p,crate::environment::parts::PartState::Owned{..})).map(|(p,_)|p.strip_prefix("part.").unwrap_or(p)).collect::<Vec<_>>().join(", "),crafting.records.iter().cloned().collect::<Vec<_>>().join(", "),["Fire","Ice","Wind","Lightning"].iter().enumerate().map(|(i,n)|format!("{n}:{}",if upgrades.charged[i]{"upgraded"}else if upgrades.inserted[i]{"charging"}else{"base"})).collect::<Vec<_>>().join("  "),rituals.headshots,rituals.generators,rituals.spent,rituals.chests,quest.stage));}
        if text.0!=value{text.0=value;}
    }
}
fn icons(mut commands:Commands,game:Res<crate::assets::GameDefinition>,server:Res<AssetServer>,players:Query<&crate::solo_revive::PerkInventory,With<Player>>,roots:Query<(Entity,Option<&Children>),With<PerkRow>>,mut previous:Local<Vec<String>>){
    let Ok(perks)=players.single()else{return;};let ids:Vec<_>=perks.owned.iter().map(|p|p.0.clone()).collect();if *previous==ids{return;}*previous=ids;
    for (root,children) in &roots{if let Some(children)=children{for child in children.iter(){commands.entity(child).despawn();}}
        commands.entity(root).with_children(|parent|{for id in &perks.owned{if let Some(perk)=game.content.perks.iter().find(|p|p.id==*id){if let Some(icon)=perk.icon.as_ref().and_then(|id|game.content.ui_assets.iter().find(|a|a.id==*id)){parent.spawn((ImageNode::new(server.load(icon.path.clone())),Node{width:px(42.),height:px(42.),..default()}));}else{parent.spawn((Text::new(&perk.name),TextFont{font_size:FontSize::Px(16.),..default()}));}}}});
    }
}
