use crate::GameState;
use bevy::{
    animation::{AnimatedBy, AnimationTargetId, RepeatAnimation},
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone)]
pub(crate) struct GraphBindings {
    pub graph: Handle<AnimationGraph>,
    pub named: BTreeMap<String, AnimationNodeIndex>,
    pub ordered: Vec<AnimationNodeIndex>,
}

#[derive(Resource, Default)]
pub(crate) struct AnimationGraphs {
    scenes: HashMap<AssetId<Gltf>, GraphBindings>,
    clips: HashMap<AssetId<AnimationClip>, GraphBindings>,
}

impl AnimationGraphs {
    pub fn contains_scene(&self, source: &Handle<Gltf>) -> bool {
        self.scenes.contains_key(&source.id())
    }

    pub fn scene(
        &mut self,
        source: &Handle<Gltf>,
        gltfs: &Assets<Gltf>,
        graphs: &mut Assets<AnimationGraph>,
    ) -> Option<&GraphBindings> {
        if !self.scenes.contains_key(&source.id()) {
            let gltf = gltfs.get(source)?;
            let (graph, ordered) = AnimationGraph::from_clips(gltf.animations.clone());
            let indices: HashMap<_, _> = gltf
                .animations
                .iter()
                .zip(&ordered)
                .map(|(clip, node)| (clip.id(), *node))
                .collect();
            let named = gltf
                .named_animations
                .iter()
                .filter_map(|(name, clip)| Some((name.to_string(), *indices.get(&clip.id())?)))
                .collect();
            self.scenes.insert(
                source.id(),
                GraphBindings {
                    graph: graphs.add(graph),
                    named,
                    ordered,
                },
            );
        }
        self.scenes.get(&source.id())
    }

    pub fn clip(
        &mut self,
        clip: &Handle<AnimationClip>,
        clips: &Assets<AnimationClip>,
        graphs: &mut Assets<AnimationGraph>,
    ) -> Option<&GraphBindings> {
        if !self.clips.contains_key(&clip.id()) {
            clips.get(clip)?;
            let (graph, node) = AnimationGraph::from_clip(clip.clone());
            self.clips.insert(
                clip.id(),
                GraphBindings {
                    graph: graphs.add(graph),
                    named: BTreeMap::new(),
                    ordered: vec![node],
                },
            );
        }
        self.clips.get(&clip.id())
    }
}

#[derive(Clone, PartialEq)]
pub(crate) struct AnimationLayer {
    pub node: AnimationNodeIndex,
    pub seconds: Option<f32>,
    pub looping: bool,
    pub weight: f32,
    pub blend_seconds: Option<f32>,
    pub preserve_on_restart: bool,
}

impl AnimationLayer {
    pub fn new(node: AnimationNodeIndex, seconds: Option<f32>, looping: bool) -> Self {
        Self {
            node,
            seconds,
            looping,
            weight: 1.,
            blend_seconds: None,
            preserve_on_restart: false,
        }
    }

    pub fn weighted(mut self, weight: f32) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Component, Clone, PartialEq)]
#[require(AnimationPlayer, AppliedPlayback)]
pub(crate) struct AnimationPlayback {
    pub graph: Handle<AnimationGraph>,
    pub layers: Vec<AnimationLayer>,
    pub restart: u64,
    pub enabled: bool,
    pub blend_seconds: f32,
}

impl AnimationPlayback {
    pub fn new(graph: Handle<AnimationGraph>, layers: Vec<AnimationLayer>) -> Self {
        Self {
            graph,
            layers,
            restart: 0,
            enabled: true,
            blend_seconds: 0.,
        }
    }
}

#[derive(Component, PartialEq)]
pub(crate) struct NativeAnimation {
    pub source: Handle<Gltf>,
    pub clip: String,
    pub seconds: f32,
    pub looping: bool,
    pub restart: u64,
}
fn native_playback(
    mut commands: Commands,
    roots: Query<(Entity, &NativeAnimation)>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    players: Query<Option<&AnimationPlayback>, With<AnimationPlayer>>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut cache: ResMut<AnimationGraphs>,
    mut failure: ResMut<crate::assets::LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    for (root, animation) in &roots {
        let Some(bindings) = cache.scene(&animation.source, &gltfs, &mut graphs) else {
            continue;
        };
        let Some(node) = bindings.named.get(&animation.clip) else {
            failure.0 = format!(
                "Required named animation {} missing from {:?}",
                animation.clip,
                animation.source.path()
            );
            next.set(GameState::LoadError);
            return;
        };
        for player in std::iter::once(root)
            .chain(children.iter_descendants(root))
            .filter(|entity| players.contains(*entity))
            .filter(|entity| {
                *entity == root
                    || parents
                        .iter_ancestors(*entity)
                        .find(|ancestor| roots.contains(*ancestor))
                        == Some(root)
            })
        {
            let mut playback = AnimationPlayback::new(
                bindings.graph.clone(),
                vec![AnimationLayer::new(
                    *node,
                    Some(animation.seconds),
                    animation.looping,
                )],
            );
            playback.restart = animation.restart;
            if players.get(player).ok().flatten() != Some(&playback) {
                commands.entity(player).insert(playback);
            }
        }
    }
}

struct WeightGoal {
    node: AnimationNodeIndex,
    from: f32,
    target: f32,
    elapsed: f32,
    blend_seconds: f32,
    preserve_on_restart: bool,
}

#[derive(Component, Default)]
struct AppliedPlayback {
    graph: Option<AssetId<AnimationGraph>>,
    restart: u64,
    weights: Vec<WeightGoal>,
}

#[derive(Component)]
struct RestPose(Transform);

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AnimationPoseModifiers;

#[derive(Resource, Default)]
struct AnimationPoseChanges(HashMap<Entity, (Transform, bevy::ecs::change_detection::Tick)>);

fn remember_poses(
    mut previous: ResMut<AnimationPoseChanges>,
    targets: Query<(Entity, Ref<Transform>), With<RestPose>>,
) {
    previous.0.clear();
    previous.0.extend(
        targets
            .iter()
            .map(|(entity, pose)| (entity, (*pose, pose.last_changed()))),
    );
}

fn retain_unchanged_poses(
    previous: Res<AnimationPoseChanges>,
    mut targets: Query<(Entity, &mut Transform), With<RestPose>>,
) {
    for (entity, mut pose) in &mut targets {
        if let Some((before, tick)) = previous.0.get(&entity)
            && *pose == *before
        {
            pose.set_last_changed(*tick);
        }
    }
}

pub(crate) struct GameAnimationPlugin;

impl Plugin for GameAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimationGraphs>()
            .init_resource::<AnimationPoseChanges>()
            .add_observer(capture_rest_pose)
            .configure_sets(
                PostUpdate,
                AnimationPoseModifiers
                    .after(bevy::app::AnimationSystems)
                    .before(retain_unchanged_poses),
            )
            .add_systems(
                PostUpdate,
                (
                    native_playback,
                    apply_playback,
                    remember_poses,
                    restore_rest_pose,
                )
                    .chain()
                    .before(bevy::app::AnimationSystems),
            )
            .add_systems(
                PostUpdate,
                retain_unchanged_poses
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            );
    }
}

fn capture_rest_pose(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    targets: Query<&Transform, (With<AnimationTargetId>, Without<RestPose>)>,
) {
    for entity in std::iter::once(event.entity).chain(children.iter_descendants(event.entity)) {
        if let Ok(pose) = targets.get(entity) {
            commands.entity(entity).insert(RestPose(*pose));
        }
    }
}

fn restore_rest_pose(
    players: Query<(), With<AnimationPlayback>>,
    mut targets: Query<(&AnimatedBy, &RestPose, &mut Transform)>,
) {
    for (owner, rest, mut pose) in &mut targets {
        if players.contains(owner.0) {
            pose.set_if_neq(rest.0);
        }
    }
}

fn apply_playback(
    mut commands: Commands,
    state: Res<State<GameState>>,
    time: Res<Time>,
    mut players: Query<(
        Entity,
        &AnimationPlayback,
        &mut AppliedPlayback,
        &mut AnimationPlayer,
    )>,
) {
    for (entity, request, mut applied, mut player) in &mut players {
        if !request.enabled {
            player.pause_all();
            continue;
        }
        let initialized = applied.graph == Some(request.graph.id());
        if !initialized || applied.restart != request.restart {
            let retained = |node| {
                initialized
                    && (request
                        .layers
                        .iter()
                        .any(|layer| layer.node == node && layer.preserve_on_restart)
                        || applied
                            .weights
                            .iter()
                            .any(|goal| goal.node == node && goal.preserve_on_restart))
            };
            let stopped = player
                .playing_animations()
                .map(|(node, _)| *node)
                .filter(|node| !retained(*node))
                .collect::<Vec<_>>();
            for node in stopped {
                player.stop(node);
            }
            applied
                .weights
                .retain(|goal| initialized && goal.preserve_on_restart);
            applied.graph = Some(request.graph.id());
            applied.restart = request.restart;
            commands
                .entity(entity)
                .insert(AnimationGraphHandle(request.graph.clone()));
        }
        let playing = *state == GameState::Playing;
        let delta = if playing { time.delta_secs() } else { 0. };
        for layer in &request.layers {
            if player.animation(layer.node).is_none() {
                player.start(layer.node).set_weight(
                    if initialized && layer.blend_seconds.unwrap_or(request.blend_seconds) > 0. {
                        0.
                    } else {
                        layer.weight
                    },
                );
            }
            let active = player.animation_mut(layer.node).unwrap();
            active
                .set_repeat(if layer.looping {
                    RepeatAnimation::Forever
                } else {
                    RepeatAnimation::Never
                })
                .set_speed(1.);
            if let Some(seconds) = layer.seconds {
                if active.seek_time() != seconds {
                    active.set_seek_time(seconds);
                }
                active.pause();
            } else if playing {
                active.resume();
            } else {
                active.pause();
            }
        }
        let nodes: Vec<_> = player.playing_animations().map(|(node, _)| *node).collect();
        for node in nodes {
            let desired = request.layers.iter().find(|layer| layer.node == node);
            let target = desired.map_or(0., |layer| layer.weight);
            let active = player.animation_mut(node).unwrap();
            let index = applied.weights.iter().position(|goal| goal.node == node);
            let goal = match index {
                Some(index) => &mut applied.weights[index],
                None => {
                    applied.weights.push(WeightGoal {
                        node,
                        from: active.weight(),
                        target,
                        elapsed: 0.,
                        blend_seconds: desired
                            .and_then(|layer| layer.blend_seconds)
                            .unwrap_or(request.blend_seconds),
                        preserve_on_restart: desired.is_some_and(|layer| layer.preserve_on_restart),
                    });
                    applied.weights.last_mut().unwrap()
                }
            };
            if goal.target != target {
                goal.from = active.weight();
                goal.target = target;
                goal.elapsed = 0.;
            }
            goal.elapsed += delta;
            let fraction = if goal.blend_seconds > 0. {
                (goal.elapsed / goal.blend_seconds).min(1.)
            } else {
                1.
            };
            active.set_weight(goal.from.lerp(goal.target, fraction));
            if desired.is_none() {
                if playing {
                    active.resume();
                } else {
                    active.pause();
                }
                if fraction >= 1. {
                    player.stop(node);
                    applied.weights.retain(|goal| goal.node != node);
                }
            }
        }
    }
}

#[cfg(test)]
mod playback_tests {
    use super::*;

    #[test]
    fn independent_aim_layers_keep_their_seek_and_blend_when_a_shot_restarts() {
        let (_, nodes) = AnimationGraph::from_clips([
            Handle::default(),
            Handle::default(),
            Handle::default(),
            Handle::default(),
        ]);
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(20));
        app.insert_resource(time)
            .insert_resource(State::new(GameState::Playing))
            .add_systems(Update, apply_playback);
        let mut aim = AnimationLayer::new(nodes[1], Some(0.1), false);
        aim.blend_seconds = Some(0.1);
        aim.preserve_on_restart = true;
        let mut lower = AnimationLayer::new(nodes[2], Some(0.1), false).weighted(0.);
        lower.blend_seconds = Some(0.1);
        lower.preserve_on_restart = true;
        let entity = app
            .world_mut()
            .spawn(AnimationPlayback::new(
                Handle::default(),
                vec![AnimationLayer::new(nodes[0], None, false), aim, lower],
            ))
            .id();
        app.update();
        {
            let mut playback = app
                .world_mut()
                .get_mut::<AnimationPlayback>(entity)
                .unwrap();
            playback.layers[0].node = nodes[3];
            playback.layers[1].seconds = Some(0.15);
            playback.layers[1].weight = 0.;
            playback.layers[2].weight = 1.;
            playback.restart += 1;
        }
        app.update();
        let player = app.world().get::<AnimationPlayer>(entity).unwrap();
        assert!(player.animation(nodes[0]).is_none());
        assert_eq!(player.animation(nodes[3]).unwrap().weight(), 1.);
        assert_eq!(player.animation(nodes[1]).unwrap().seek_time(), 0.15);
        assert!((player.animation(nodes[1]).unwrap().weight() - 0.8).abs() < 1e-6);
        assert!((player.animation(nodes[2]).unwrap().weight() - 0.2).abs() < 1e-6);
        app.world_mut()
            .get_mut::<AnimationPlayback>(entity)
            .unwrap()
            .restart += 1;
        app.update();
        let player = app.world().get::<AnimationPlayer>(entity).unwrap();
        assert!((player.animation(nodes[1]).unwrap().weight() - 0.6).abs() < 1e-6);
        assert!((player.animation(nodes[2]).unwrap().weight() - 0.4).abs() < 1e-6);
    }
}

#[cfg(all(debug_assertions, feature = "dev-tools"))]
pub(crate) fn observation(world: &mut World) -> Vec<serde_json::Value> {
    let mut targets: HashMap<Entity, Vec<serde_json::Value>> = HashMap::new();
    for (owner, name, pose, rest) in world
        .query::<(&AnimatedBy, &Name, &Transform, &RestPose)>()
        .iter(world)
    {
        targets.entry(owner.0).or_default().push(serde_json::json!({
            "name":name.as_str(), "translation":pose.translation.to_array(), "rotation":pose.rotation.to_array(),
            "rest_translation":rest.0.translation.to_array(), "rest_rotation":rest.0.rotation.to_array(),
        }));
    }
    let players: Vec<_> = world
        .query::<(Entity, &AnimationPlayback, &AnimationPlayer)>()
        .iter(world)
        .map(|(entity, request, player)| (entity, request.clone(), player.clone()))
        .collect();
    let cache = world.resource::<AnimationGraphs>();
    players.into_iter().map(|(entity, request, player)| {
        let bindings = cache.scenes.values().chain(cache.clips.values()).find(|b| b.graph.id() == request.graph.id());
        let layers: Vec<_> = player.playing_animations().map(|(node, active)| {
            let desired = request.layers.iter().find(|l| l.node == *node);
            serde_json::json!({"clip":bindings.and_then(|b| b.named.iter().find(|(_, n)| **n == *node)).map(|(name, _)|name),
                "seconds":active.seek_time(), "weight":active.weight(), "paused":active.is_paused(), "speed":active.speed(),
                "requested_seconds":desired.and_then(|l| l.seconds), "requested_weight":desired.map(|l| l.weight)})
        }).collect();
        serde_json::json!({"entity":entity.to_bits(), "enabled":request.enabled, "restart":request.restart,
            "layers":layers, "bones":targets.remove(&entity).unwrap_or_default()})
    }).collect()
}
