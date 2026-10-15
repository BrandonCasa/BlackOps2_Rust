use crate::assets::GameDefinition;
use bevy::{
    animation::{AnimatedBy, AnimationTargetId, graph::AnimationNodeType},
    ecs::system::SystemParam,
    mesh::skinning::SkinnedMesh,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::{BTreeMap, HashSet};

/// Clip handles live in the animated object's graph and leave with that object.
#[derive(Component, Default)]
#[require(AnimationPlayer, AnimationGraphHandle)]
pub(crate) struct SceneClips {
    nodes: BTreeMap<String, AnimationNodeIndex>,
    pub ready: bool,
}

#[derive(SystemParam)]
pub(crate) struct ClipAssets<'w> {
    pub server: Res<'w, AssetServer>,
    pub clips: Res<'w, Assets<AnimationClip>>,
    pub graphs: ResMut<'w, Assets<AnimationGraph>>,
}
impl ClipAssets<'_> {
    pub fn duration(&self, node: AnimationNodeIndex, graph: &AnimationGraphHandle) -> Option<f32> {
        let AnimationNodeType::Clip(clip) = &self.graphs.get(&graph.0)?[node].node_type else {
            return None;
        };
        Some(self.clips.get(clip)?.duration())
    }
}

pub(crate) fn clip_path<'a>(
    game: &'a GameDefinition,
    source: &Handle<Gltf>,
    name: &str,
) -> Option<&'a str> {
    clip_binding(game, source, name).flatten()
}

/// The outer option reports missing data; the inner option is an authored bind-only pose.
pub(crate) fn clip_binding<'a>(
    game: &'a GameDefinition,
    source: &Handle<Gltf>,
    name: &str,
) -> Option<Option<&'a str>> {
    let scene = source.path()?.path().to_str()?;
    Some(game.scene_animations.get(scene)?.get(name)?.as_deref())
}

impl SceneClips {
    pub fn bound() -> Self {
        Self {
            ready: true,
            ..Default::default()
        }
    }
    pub fn node(
        &mut self,
        path: &str,
        graph: &mut AnimationGraphHandle,
        assets: &mut ClipAssets,
    ) -> Option<(AnimationNodeIndex, f32)> {
        if let Some(&node) = self.nodes.get(path) {
            return Some((node, assets.duration(node, graph)?));
        }
        if !assets.graphs.contains(&graph.0) {
            graph.0 = assets.graphs.add(AnimationGraph::new());
        }
        let mut owned = assets.graphs.get_mut(&graph.0).unwrap();
        let node = *self.nodes.entry(path.to_owned()).or_insert_with(|| {
            let clip = assets.server.load(path.to_owned());
            // Reuse retired leaves so graph size follows concurrent layers, not session length.
            let empty = owned.nodes().find(|&node| {
                node != owned.root && matches!(owned[node].node_type, AnimationNodeType::Blend)
            });
            if let Some(node) = empty {
                owned[node].node_type = AnimationNodeType::Clip(clip);
                node
            } else {
                let root = owned.root;
                owned.add_clip(clip, 1., root)
            }
        });
        let AnimationNodeType::Clip(clip) = &owned[node].node_type else {
            unreachable!()
        };
        Some((node, assets.clips.get(clip)?.duration()))
    }

    /// Release clips after their last contributing layer or owned next action retires.
    pub fn retain(
        &mut self,
        nodes: impl IntoIterator<Item = AnimationNodeIndex>,
        graph: &AnimationGraphHandle,
        assets: &mut ClipAssets,
    ) {
        let retained: HashSet<_> = nodes.into_iter().collect();
        let Some(mut owned) = assets.graphs.get_mut(&graph.0) else {
            return;
        };
        self.nodes.retain(|_, node| {
            if retained.contains(node) {
                return true;
            }
            owned[*node].node_type = AnimationNodeType::Blend;
            false
        });
    }

    /// Sample the owner-selected source time directly; gameplay owns action selection and notifies.
    pub fn sample(
        &mut self,
        path: &str,
        seconds: f32,
        looping: bool,
        player: &mut AnimationPlayer,
        graph: &mut AnimationGraphHandle,
        assets: &mut ClipAssets,
    ) -> bool {
        let Some((node, duration)) = self.node(path, graph, assets) else {
            return false;
        };
        if !self.ready {
            return false;
        }
        if player.animation(node).is_none() {
            player.stop_all();
            player.start(node);
            self.retain([node], graph, assets);
        }
        player
            .play(node)
            .set_weight(1.)
            .set_seek_time(if looping && duration > 0. {
                seconds.rem_euclid(duration)
            } else {
                seconds.min(duration)
            })
            .pause();
        true
    }
}

#[derive(Component)]
struct RestPose(Transform);

#[derive(SystemParam)]
pub(crate) struct SceneJoints<'w, 's> {
    children: Query<'w, 's, &'static Children>,
    parents: Query<'w, 's, &'static ChildOf>,
    names: Query<'w, 's, &'static Name>,
    poses: Query<'w, 's, &'static Transform>,
    skins: Query<'w, 's, &'static SkinnedMesh>,
    players: Query<'w, 's, (), With<AnimationPlayer>>,
}
impl SceneJoints<'_, '_> {
    pub fn bind(&self, owner: Entity, commands: &mut Commands) {
        let joints: HashSet<_> = self
            .children
            .iter_descendants(owner)
            .filter_map(|entity| self.skins.get(entity).ok())
            .flat_map(|skin| skin.joints.iter().copied())
            .collect();
        for &joint in &joints {
            let mut path = vec![self.names.get(joint).expect("Exported joint name").clone()];
            for parent in self
                .parents
                .iter_ancestors(joint)
                .take_while(|parent| joints.contains(parent))
            {
                path.push(
                    self.names
                        .get(parent)
                        .expect("Exported joint parent name")
                        .clone(),
                );
            }
            path.reverse();
            commands.entity(joint).insert((
                AnimationTargetId::from_names(path.iter()),
                AnimatedBy(owner),
                RestPose(
                    *self
                        .poses
                        .get(joint)
                        .expect("Exported joint bind transform"),
                ),
            ));
        }
        for entity in self
            .children
            .iter_descendants(owner)
            .filter(|entity| self.players.contains(*entity))
        {
            commands.entity(entity).remove::<AnimationPlayer>();
        }
    }
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct AnimationPoseModifiers;

pub(crate) struct GameAnimationPlugin;
impl Plugin for GameAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(bind_scene)
            .configure_sets(
                PostUpdate,
                AnimationPoseModifiers
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                PostUpdate,
                restore_rest_pose.before(bevy::app::AnimationSystems),
            );
    }
}
fn bind_scene(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    mut owners: Query<&mut SceneClips, Without<crate::presentation::Rig>>,
    joints: SceneJoints,
) {
    if let Ok(mut owner) = owners.get_mut(event.entity) {
        joints.bind(event.entity, &mut commands);
        owner.ready = true;
    }
}
fn restore_rest_pose(mut targets: Query<(&RestPose, &mut Transform)>) {
    for (rest, mut pose) in &mut targets {
        pose.set_if_neq(rest.0);
    }
}
