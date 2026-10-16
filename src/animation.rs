use crate::assets::GameDefinition;
use bevy::{
    animation::{
        AnimatedBy, AnimationTargetId, animated_field, animation_curves::AnimatableProperty,
        graph::AnimationNodeType,
    },
    ecs::system::SystemParam,
    mesh::skinning::SkinnedMesh,
    prelude::*,
    world_serialization::WorldInstanceReady,
};
use std::collections::{BTreeMap, HashSet};

mod transition;
pub(crate) use transition::{ClipBlend, GoalWeight};

struct SceneLayer {
    node: AnimationNodeIndex,
    seconds: f32,
    duration: f32,
    looping: bool,
    rate: f32,
    goal: GoalWeight,
}

/// Clip handles live in the animated object's graph and leave with that object.
#[derive(Component, Default)]
#[require(AnimationPlayer, AnimationGraphHandle)]
pub(crate) struct SceneClips {
    nodes: BTreeMap<String, AnimationNodeIndex>,
    main: Option<AnimationNodeIndex>,
    rest_node: Option<AnimationNodeIndex>,
    layers: Vec<SceneLayer>,
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
        let rest_node = self.rest_node;
        let node = *self.nodes.entry(path.to_owned()).or_insert_with(|| {
            let clip = assets.server.load(path.to_owned());
            // Reuse retired leaves so graph size follows concurrent layers, not session length.
            let empty = owned.nodes().find(|&node| {
                node != owned.root
                    && Some(node) != rest_node
                    && matches!(owned[node].node_type, AnimationNodeType::Blend)
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
        self.layers.retain(|layer| retained.contains(&layer.node));
        if self.main.is_some_and(|node| !retained.contains(&node)) {
            self.main = None;
        }
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
        self.sample_transition(
            Some(path),
            seconds,
            looping,
            ClipBlend::CUT,
            1.,
            0.,
            player,
            graph,
            assets,
        )
    }

    /// Source SetAnim/ClearAnim weights, with gameplay still owning source time.
    /// Bevy's AnimationTransitions ignores paused clips and rewrites the main weight,
    /// so it cannot own these explicitly sampled, independently timed layers.
    pub fn sample_transition(
        &mut self,
        path: Option<&str>,
        seconds: f32,
        looping: bool,
        blend: ClipBlend,
        rate: f32,
        dt: f32,
        player: &mut AnimationPlayer,
        graph: &mut AnimationGraphHandle,
        assets: &mut ClipAssets,
    ) -> bool {
        let (node, duration) = if let Some(path) = path {
            let Some(sample) = self.node(path, graph, assets) else {
                return false;
            };
            sample
        } else if let Some(node) = self.rest_node {
            (node, 0.)
        } else {
            if !assets.graphs.contains(&graph.0) {
                graph.0 = assets.graphs.add(AnimationGraph::new());
            }
            let mut owned = assets.graphs.get_mut(&graph.0).unwrap();
            let root = owned.root;
            // An empty blend leaf contributes the authored rest frame in
            // blend_scene_rest without manufacturing another clip asset.
            let node = owned.add_blend(1., root);
            self.rest_node = Some(node);
            (node, 0.)
        };
        if !self.ready {
            return false;
        }
        let changed = self.main != Some(node) || player.animation(node).is_none();
        if changed {
            // A fresh owner has no outgoing pose. Initialize it directly at its
            // authoritative time instead of showing an unrelated bind frame.
            let initial = self.layers.is_empty() || player.playing_animations().next().is_none();
            if initial {
                self.layers.clear();
                player.stop_all();
            }
            for layer in &mut self.layers {
                layer.goal.set(0., blend.outgoing);
            }
            if let Some(layer) = self.layers.iter_mut().find(|layer| layer.node == node) {
                layer.goal.set(1., blend.incoming);
            } else {
                let mut goal = GoalWeight::default();
                goal.set(1., if initial { 0. } else { blend.incoming });
                self.layers.push(SceneLayer {
                    node,
                    seconds,
                    duration,
                    looping,
                    rate,
                    goal,
                });
            }
            self.main = Some(node);
        }
        let mut retired = false;
        self.layers.retain_mut(|layer| {
            let weight = layer.goal.advance(dt);
            if layer.node != node && weight == 0. {
                player.stop(layer.node);
                retired = true;
                return false;
            }
            if layer.node == node {
                layer.seconds = seconds;
                layer.looping = looping;
                layer.rate = rate;
            } else {
                layer.seconds += dt * layer.rate;
            }
            let seek = if layer.looping && layer.duration > 0. {
                layer.seconds.rem_euclid(layer.duration)
            } else {
                layer.seconds.min(layer.duration)
            };
            player
                .play(layer.node)
                .set_weight(weight)
                .set_seek_time(seek)
                .pause();
            true
        });
        if changed || retired {
            let nodes: Vec<_> = self.layers.iter().map(|layer| layer.node).collect();
            self.retain(nodes, graph, assets);
        }
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
            )
            .add_systems(
                PostUpdate,
                blend_scene_rest
                    .after(bevy::app::AnimationSystems)
                    .before(AnimationPoseModifiers),
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

// Bevy normalizes only curves that exist for a property. A sparse incoming clip
// must also blend an outgoing keyed property back to its authored rest value.
// Account for that missing contribution only while scene layers overlap.
fn blend_scene_rest(
    owners: Query<(&SceneClips, &AnimationGraphHandle)>,
    graphs: Res<Assets<AnimationGraph>>,
    clips: Res<Assets<AnimationClip>>,
    mut targets: Query<(&RestPose, &AnimatedBy, &AnimationTargetId, &mut Transform)>,
) {
    if !owners.iter().any(|(owner, _)| owner.layers.len() > 1) {
        return;
    }
    let translation = animated_field!(Transform::translation);
    let rotation = animated_field!(Transform::rotation);
    let scale = animated_field!(Transform::scale);
    for (rest, animated, target, mut pose) in &mut targets {
        let Ok((owner, graph)) = owners.get(animated.0) else {
            continue;
        };
        if owner.layers.len() < 2 {
            continue;
        }
        let graph = graphs.get(&graph.0).expect("Sampled scene graph");
        let total: f32 = owner.layers.iter().map(|layer| layer.goal.weight).sum();
        let mut weights = [0.; 3];
        for layer in &owner.layers {
            let AnimationNodeType::Clip(clip) = &graph[layer.node].node_type else {
                continue;
            };
            let Some(curves) = clips
                .get(clip)
                .and_then(|clip| clip.curves_for_target(*target))
            else {
                continue;
            };
            for (weight, property) in weights.iter_mut().zip([
                translation.evaluator_id(),
                rotation.evaluator_id(),
                scale.evaluator_id(),
            ]) {
                if curves
                    .iter()
                    .any(|curve| curve.0.evaluator_id() == property)
                {
                    *weight += layer.goal.weight;
                }
            }
        }
        if total > 0. {
            pose.translation = rest
                .0
                .translation
                .lerp(pose.translation, (weights[0] / total).min(1.));
            pose.rotation = rest
                .0
                .rotation
                .slerp(pose.rotation, (weights[1] / total).min(1.));
            pose.scale = rest.0.scale.lerp(pose.scale, (weights[2] / total).min(1.));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::curve::{ConstantCurve, Interval};

    #[test]
    fn sparse_scene_transition_includes_the_missing_rest_contribution() {
        let target = AnimationTargetId::from_name(&Name::new("joint"));
        let mut old = AnimationClip::default();
        old.add_curve_to_target(
            target,
            AnimatableCurve::new(
                animated_field!(Transform::translation),
                ConstantCurve::new(Interval::EVERYWHERE, Vec3::X * 10.),
            ),
        );
        let mut new = AnimationClip::default();
        new.add_curve_to_target(
            target,
            AnimatableCurve::new(
                animated_field!(Transform::rotation),
                ConstantCurve::new(Interval::EVERYWHERE, Quat::from_rotation_y(1.)),
            ),
        );
        let mut clips = Assets::default();
        let mut graph = AnimationGraph::new();
        let root = graph.root;
        let old = graph.add_clip(clips.add(old), 1., root);
        let new = graph.add_clip(clips.add(new), 1., root);
        let mut graphs = Assets::default();
        let graph = AnimationGraphHandle(graphs.add(graph));
        let layer = |node, weight| {
            let mut goal = GoalWeight::default();
            goal.set(weight, 0.);
            SceneLayer {
                node,
                goal,
                seconds: 0.,
                duration: 1.,
                looping: false,
                rate: 1.,
            }
        };
        let mut app = App::new();
        app.insert_resource(clips)
            .insert_resource(graphs)
            .add_systems(Update, blend_scene_rest);
        let owner = app
            .world_mut()
            .spawn((
                SceneClips {
                    layers: vec![layer(old, 0.25), layer(new, 0.75)],
                    ..default()
                },
                graph,
            ))
            .id();
        // This is Bevy's normalized keyed result: each property has only one curve.
        let joint = app
            .world_mut()
            .spawn((
                RestPose(Transform::from_translation(Vec3::X * 2.)),
                AnimatedBy(owner),
                target,
                Transform::from_translation(Vec3::X * 10.).with_rotation(Quat::from_rotation_y(1.)),
            ))
            .id();
        app.update();
        let pose = app.world().get::<Transform>(joint).unwrap();
        assert!(pose.translation.abs_diff_eq(Vec3::X * 4., 1e-6));
        assert!(pose.rotation.abs_diff_eq(Quat::from_rotation_y(0.75), 1e-6));
        assert_eq!(pose.scale, Vec3::ONE);

        // A bind-only actor part has no clip channels at all, but still owns
        // the incoming weight while the old translation fades out.
        let handle = app
            .world()
            .get::<AnimationGraphHandle>(owner)
            .unwrap()
            .0
            .clone();
        let rest = {
            let mut graphs = app.world_mut().resource_mut::<Assets<AnimationGraph>>();
            let mut graph = graphs.get_mut(&handle).unwrap();
            let root = graph.root;
            graph.add_blend(1., root)
        };
        app.world_mut().get_mut::<SceneClips>(owner).unwrap().layers[1].node = rest;
        *app.world_mut().get_mut::<Transform>(joint).unwrap() =
            Transform::from_translation(Vec3::X * 10.);
        app.update();
        let pose = app.world().get::<Transform>(joint).unwrap();
        assert!(pose.translation.abs_diff_eq(Vec3::X * 4., 1e-6));
        assert_eq!(pose.rotation, Quat::IDENTITY);
    }
}
