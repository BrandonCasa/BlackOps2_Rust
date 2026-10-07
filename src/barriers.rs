//! Authored wood windows, individual boards and fixed-step repair/tear ownership.
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, Session, SessionEntity},
    origins::{ContentId, PlacementId},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarrierDefinition {
    pub id: ContentId,
    pub placement: PlacementId,
    pub entry: ContentId,
    pub exit: ContentId,
    pub boards: Vec<BoardDefinition>,
    pub repair_sound: ContentId,
    pub break_sound: ContentId,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoardDefinition {
    pub presentation: ContentId,
    pub repair: ContentId,
    pub pull: ContentId,
    pub repair_seconds: f32,
    pub pull_seconds: f32,
    pub grab_seconds: f32,
    pub tear_seconds: f32,
    pub destroy_seconds: f32,
    pub grab_clip: String,
    pub tear_clip: String,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoardState {
    Closed,
    Reserved(Entity),
    Opening(f32),
    Missing,
    Closing(f32),
}
#[derive(Resource, Default)]
pub struct Barriers {
    pub windows: BTreeMap<ContentId, Vec<BoardState>>,
    pub repaired: u32,
    pub destroyed: u32,
    pub entries: u32,
    pub sounds: u32,
    pub playing_sounds: u32,
    pub animated_boards: u32,
    pub awarded: u32,
    pub reward_counter: u32,
    owners: BTreeMap<ContentId, Entity>,
    round: u32,
}
impl Barriers {
    pub fn repairable(&self, id: &ContentId) -> bool {
        self.windows
            .get(id)
            .is_some_and(|b| b.contains(&BoardState::Missing))
    }
    pub fn repair(&mut self, id: &ContentId) -> bool {
        let Some(board) = self
            .windows
            .get_mut(id)
            .and_then(|b| b.iter_mut().find(|b| **b == BoardState::Missing))
        else {
            return false;
        };
        *board = BoardState::Closing(0.);
        true
    }
    pub fn obstructed(&self, id: &ContentId) -> bool {
        self.windows.get(id).is_some_and(|b| {
            b.iter()
                .any(|b| matches!(b, BoardState::Closed | BoardState::Reserved(_)))
        })
    }
    pub fn claim(&mut self, id: &ContentId, owner: Entity) -> Option<usize> {
        if self.owners.contains_key(id) {
            return None;
        }
        let boards = self.windows.get_mut(id)?;
        let i = boards.iter().position(|b| *b == BoardState::Closed)?;
        boards[i] = BoardState::Reserved(owner);
        self.owners.insert(id.clone(), owner);
        Some(i)
    }
    pub fn destroy(&mut self, id: &ContentId, index: usize, owner: Entity) -> bool {
        let Some(b) = self.windows.get_mut(id).and_then(|b| b.get_mut(index)) else {
            return false;
        };
        if *b != BoardState::Reserved(owner) {
            return false;
        }
        *b = BoardState::Opening(0.);
        self.destroyed += 1;
        true
    }
    pub fn release(&mut self, owner: Entity) {
        self.owners.retain(|_, e| *e != owner);
        for boards in self.windows.values_mut() {
            for b in boards {
                if *b == BoardState::Reserved(owner) {
                    *b = BoardState::Closed;
                }
            }
        }
    }
}
#[derive(Component)]
pub struct BarrierBlocker(pub ContentId);
#[derive(Component)]
pub struct PlayerIngressBlocker;
#[derive(Component)]
pub struct BoardVisual {
    id: ContentId,
    index: usize,
    source: Handle<Gltf>,
    graph: Option<(
        Handle<AnimationGraph>,
        AnimationNodeIndex,
        AnimationNodeIndex,
    )>,
}
#[derive(Component)]
struct WoodAudio;
#[derive(Component)]
struct BoardPlayback;
pub struct BarrierPlugin;
impl Plugin for BarrierPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Barriers>()
            .add_systems(OnEnter(GameState::Playing), assemble)
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(Update, (animate, audio_evidence));
    }
}
fn assemble(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
    server: Res<AssetServer>,
    mut barriers: ResMut<Barriers>,
) {
    if !players.is_empty() {
        return;
    }
    *barriers = Barriers::default();
    for d in &definition.content.barriers {
        let Some(p) = definition
            .content
            .placements
            .iter()
            .find(|p| p.id == d.placement)
        else {
            continue;
        };
        barriers
            .windows
            .insert(d.id.clone(), vec![BoardState::Closed; d.boards.len()]);
        let origin = Vec3::from_array(p.position);
        let rotation = Quat::from_rotation_y(p.yaw);
        commands.spawn((
            SessionEntity,
            BarrierBlocker(d.id.clone()),
            RigidBody::Static,
            Collider::cuboid(0.16, 1.5, 1.8),
            Transform::from_translation(origin + Vec3::Y * 0.75).with_rotation(rotation),
        ));
        // Reconstructed player clip spans the window and its sandbag shoulders.
        // Its top exceeds a full jump and its bottom closes the crouch/step path;
        // the stacked bunker entrances retain their separate floor clearance.
        // Board destruction never disables this actor-specific boundary.
        let interior = definition
            .content
            .navigation_nodes
            .iter()
            .find(|n| n.id == d.exit)
            .map(|n| Vec3::from_array(n.position))
            .expect("validated barrier exit");
        // The authored board pivot sits inside the exterior pocket. Seal its
        // playable-side mouth at the native landing point.
        let clip_origin = interior;
        commands.spawn((
            SessionEntity,
            PlayerIngressBlocker,
            RigidBody::Static,
            CollisionLayers::new(crate::environment::PLAYER_INGRESS_LAYER, LayerMask::NONE),
            Collider::cuboid(0.2, 4.2, 3.2),
            Transform::from_translation(clip_origin + Vec3::Y * 1.1).with_rotation(rotation),
        ));
        for (index, b) in d.boards.iter().enumerate() {
            let Some(scene) = definition
                .content
                .presentations
                .iter()
                .find(|p| p.id == b.presentation)
                .and_then(|p| p.scene.as_ref())
            else {
                continue;
            };
            let base = scene.split('#').next().unwrap_or(scene);
            commands.spawn((
                SessionEntity,
                BoardVisual {
                    id: d.id.clone(),
                    index,
                    source: server.load(base.to_owned()),
                    graph: None,
                },
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(origin).with_rotation(rotation),
                Visibility::Inherited,
            ));
        }
    }
}
pub fn sound(
    commands: &mut Commands,
    server: &AssetServer,
    definition: &GameDefinition,
    id: &ContentId,
    position: Vec3,
) {
    if let Some(s) = definition.content.sounds.iter().find(|s| s.id == *id) {
        commands.spawn((
            SessionEntity,
            WoodAudio,
            AudioPlayer::new(server.load(s.path.clone())),
            PlaybackSettings::DESPAWN
                .with_spatial(true)
                .with_volume(bevy::audio::Volume::Linear(s.volume)),
            Transform::from_translation(position),
        ));
    }
}
fn audio_evidence(
    sinks: Query<(), (With<WoodAudio>, Added<bevy::audio::SpatialAudioSink>)>,
    mut barriers: ResMut<Barriers>,
) {
    barriers.playing_sounds += sinks.iter().count() as u32;
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut barriers: ResMut<Barriers>,
    mut session: ResMut<Session>,
    blockers: Query<(Entity, &BarrierBlocker, Option<&ColliderDisabled>)>,
    enemies: Query<&crate::life::Vitality, With<crate::enemies::Zombie>>,
) {
    if barriers.round != session.round {
        barriers.round = session.round;
        barriers.reward_counter = 0;
    }
    let retired: Vec<_> = barriers
        .owners
        .values()
        .copied()
        .filter(|e| {
            !enemies
                .get(*e)
                .is_ok_and(|v| v.state() == crate::life::LifeState::ActorAlive)
        })
        .collect();
    for e in retired {
        barriers.release(e);
    }
    for d in &definition.content.barriers {
        let Some(boards) = barriers.windows.get_mut(&d.id) else {
            continue;
        };
        let mut repaired = 0;
        for (i, b) in boards.iter_mut().enumerate() {
            match *b {
                BoardState::Reserved(owner)
                    if !enemies
                        .get(owner)
                        .is_ok_and(|v| v.state() == crate::life::LifeState::ActorAlive) =>
                {
                    *b = BoardState::Closed
                }
                BoardState::Opening(age) => {
                    let age = age + time.delta_secs();
                    *b = if age >= d.boards[i].pull_seconds {
                        BoardState::Missing
                    } else {
                        BoardState::Opening(age)
                    };
                }
                BoardState::Closing(age) => {
                    let age = age + time.delta_secs();
                    *b = if age >= d.boards[i].repair_seconds {
                        repaired += 1;
                        BoardState::Closed
                    } else {
                        BoardState::Closing(age)
                    };
                }
                _ => {}
            }
        }
        let obstructed = boards
            .iter()
            .any(|b| matches!(b, BoardState::Closed | BoardState::Reserved(_)));
        for (e, b, disabled) in &blockers {
            if b.0 != d.id {
                continue;
            }
            if obstructed && disabled.is_some() {
                commands.entity(e).remove::<ColliderDisabled>();
            }
            if !obstructed && disabled.is_none() {
                commands.entity(e).insert(ColliderDisabled);
            }
        }
        for _ in 0..repaired {
            barriers.repaired += 1;
            barriers.reward_counter = barriers.reward_counter.saturating_add(10);
            if barriers.reward_counter < session.round.saturating_mul(50).min(500) {
                session.points = session.points.saturating_add(10);
                barriers.awarded += 10;
            }
            if let Some(p) = definition
                .content
                .placements
                .iter()
                .find(|p| p.id == d.placement)
            {
                sound(
                    &mut commands,
                    &server,
                    &definition,
                    &d.repair_sound,
                    Vec3::from_array(p.position),
                );
                barriers.sounds += 1;
            }
        }
    }
}
fn animate(
    mut commands: Commands,
    definition: Option<Res<GameDefinition>>,
    mut barriers: ResMut<Barriers>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut boards: Query<(Entity, &mut BoardVisual, &mut Visibility)>,
    children: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, Option<&BoardPlayback>)>,
) {
    let Some(definition) = definition else {
        return;
    };
    for (entity, mut visual, mut visibility) in &mut boards {
        let Some(d) = definition
            .content
            .barriers
            .iter()
            .find(|d| d.id == visual.id)
        else {
            continue;
        };
        let Some(state) = barriers
            .windows
            .get(&visual.id)
            .and_then(|b| b.get(visual.index))
            .copied()
        else {
            continue;
        };
        *visibility = if state == BoardState::Missing {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
        if visual.graph.is_none() {
            let Some(g) = gltfs.get(&visual.source) else {
                continue;
            };
            if g.animations.len() != 2 {
                continue;
            }
            let (graph, nodes) = AnimationGraph::from_clips(g.animations.clone());
            visual.graph = Some((graphs.add(graph), nodes[0], nodes[1]));
            barriers.animated_boards += 1;
        }
        let Some((graph, repair, pull)) = &visual.graph else {
            continue;
        };
        let (node, elapsed) = match state {
            BoardState::Opening(age) => (*pull, age),
            BoardState::Closing(age) => (*repair, age),
            _ => (*repair, d.boards[visual.index].repair_seconds),
        };
        for child in children.iter_descendants(entity) {
            let Ok((mut player, attached)) = players.get_mut(child) else {
                continue;
            };
            if attached.is_none() {
                commands
                    .entity(child)
                    .insert((AnimationGraphHandle(graph.clone()), BoardPlayback));
            }
            if player.animation(node).is_none() {
                player.stop_all();
                player.play(node);
            }
            if let Some(active) = player.animation_mut(node) {
                active.seek_to(elapsed);
            }
            player.pause_all();
        }
    }
}
