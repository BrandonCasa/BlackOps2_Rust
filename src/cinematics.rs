use crate::GameState;
use bevy::prelude::*;
#[derive(Resource,Default)]
struct Playback { elapsed:f32, frames:Vec<String>, rate:f32, duration:f32 }
#[derive(Component)]
struct CinematicRoot;
#[derive(Component)]
struct CinematicFrame;
pub struct CinematicPlugin;
impl Plugin for CinematicPlugin {
    fn build(&self,app:&mut App){app.init_resource::<Playback>()
        .add_systems(OnEnter(GameState::Ending),start.after(crate::audio::stop))
        .add_systems(Update,advance.run_if(in_state(GameState::Ending)))
        .add_systems(OnExit(GameState::Ending),close);}
}
fn start(mut commands:Commands,mut playback:ResMut<Playback>,game:Res<crate::assets::GameDefinition>,server:Res<AssetServer>,cameras:Query<Entity,With<crate::presentation::ViewModelCamera>>){
    *playback=Playback{duration:12.,rate:30.,..default()};
    if let Some(sequence)=game.content.cinematics.iter().find(|c|c.id.0.contains("ending") || c.id.0.contains("outro")){
        playback.frames=sequence.frames.clone();playback.rate=sequence.frames_per_second;playback.duration=playback.frames.len()as f32/playback.rate;
        if let Some(audio)=&sequence.audio{crate::audio::emit(&mut commands,&audio.0,crate::audio::SoundSource::World(Vec3::ZERO));}
    }
    let Ok(camera)=cameras.single()else{return;};
    commands.spawn((CinematicRoot,UiTargetCamera(camera),GlobalZIndex(200),Node{position_type:PositionType::Absolute,width:percent(100.),height:percent(100.),align_items:AlignItems::Center,justify_content:JustifyContent::Center,flex_direction:FlexDirection::Column,..default()},BackgroundColor(Color::BLACK))).with_children(|parent|{
        if let Some(frame)=playback.frames.first(){parent.spawn((CinematicFrame,ImageNode::new(server.load(frame.clone())),Node{width:percent(100.),height:percent(100.),..default()}));}else{parent.spawn((Text::new("LITTLE GIRL LOST"),TextFont{font_size:FontSize::Px(48.),..default()},TextColor(Color::WHITE)));}
        parent.spawn((Text::new("Esc / Space: skip"),TextFont{font_size:FontSize::Px(18.),..default()},TextColor(Color::WHITE)));
    });
}
fn advance(time:Res<Time>,keys:Res<ButtonInput<KeyCode>>,mut playback:ResMut<Playback>,server:Res<AssetServer>,mut frames:Query<&mut ImageNode,With<CinematicFrame>>,mut next:ResMut<NextState<GameState>>){
    playback.elapsed+=time.delta_secs();
    if !playback.frames.is_empty(){let index=((playback.elapsed*playback.rate)as usize).min(playback.frames.len()-1);for mut frame in &mut frames{frame.image=server.load(playback.frames[index].clone());}}
    if playback.elapsed>=playback.duration || keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::Space){next.set(GameState::GameOver);}
}
fn close(mut commands:Commands,roots:Query<Entity,With<CinematicRoot>>){for entity in &roots{commands.entity(entity).despawn();}}
