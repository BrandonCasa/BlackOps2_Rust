//! Bind existing authored enemy animation records to exact external scene labels.
use super::{AnimationNotification, GameDefinition};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(Deserialize)]
struct Scene {
    model: String,
    machine: String,
    scene: String,
    tracks: Vec<SceneClip>,
}
#[derive(Deserialize)]
struct SceneClip {
    name: String,
    animation_path: Option<String>,
}
#[derive(Deserialize)]
struct ClipTrack {
    fps: f32,
    frames: u32,
    notetracks: Vec<(String, f32)>,
}

#[derive(Deserialize)]
struct ActorCatalog {
    enemies: Vec<super::EnemyDefinition>,
    presentations: Vec<super::PresentationDefinition>,
    animations: Vec<super::AnimationDefinition>,
}

/// Gameplay definitions remain authoritative for actor identity, hit volumes,
/// physics, ragdolls, state selection and traversal root motion.
pub(crate) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    let catalog: ActorCatalog = super::read(&root.join("enemies.json"));
    game.content.enemies = catalog.enemies;
    game.content.presentations.extend(catalog.presentations);
    game.content.animations.extend(catalog.animations);
    let path = root.join("source-actor-clips.json");
    let scenes: Vec<Scene> =
        serde_json::from_slice(&std::fs::read(&path).map_err(|error| {
            format!("Required actor scene bindings {}: {error}", path.display())
        })?)
        .map_err(|error| format!("Invalid actor scene bindings {}: {error}", path.display()))?;
    let owned: BTreeSet<_> = game
        .content
        .enemies
        .iter()
        .flat_map(|enemy| {
            enemy
                .presentation
                .iter()
                .chain(enemy.parts.iter().map(|part| &part.presentation))
        })
        .cloned()
        .collect();
    for presentation in game
        .content
        .presentations
        .iter_mut()
        .filter(|presentation| owned.contains(&presentation.id))
    {
        let model = presentation.id.0.strip_prefix("actor.").ok_or_else(|| {
            format!(
                "Actor presentation {} has no explicit source model identity",
                presentation.id.0
            )
        })?;
        let source = if let Some(source) = scenes.iter().find(|scene| scene.model == model) {
            source
        } else if presentation.animations.is_empty() {
            continue;
        } else {
            return Err(format!(
                "Actor presentation {} has no exact exported animated source binding",
                presentation.id.0
            ));
        };

        let tracks_path = root.join(format!("actors/{}/source-tracks.json", source.machine));
        let tracks: BTreeMap<String, ClipTrack> =
            serde_json::from_slice(&std::fs::read(&tracks_path).map_err(|error| {
                format!(
                    "Required actor source tracks {}: {error}",
                    tracks_path.display()
                )
            })?)
            .map_err(|error| {
                format!(
                    "Invalid actor source tracks {}: {error}",
                    tracks_path.display()
                )
            })?;
        presentation.scene = Some(format!("{}#Scene0", source.scene));
        for animation in game
            .content
            .animations
            .iter_mut()
            .filter(|animation| animation.skeleton == presentation.id)
        {
            let name = animation.id.0.rsplit('.').next().unwrap_or("");
            let source_clip = source
                .tracks
                .iter()
                .find(|clip| clip.name == name)
                .ok_or_else(|| {
                    format!("Actor {} source clip {name} was not exported", source.model)
                })?;
            let label = source_clip.animation_path.as_ref().ok_or_else(|| {
                format!(
                    "Actor {} source clip {name} has no animated scene channels",
                    source.model
                )
            })?;

            let track = tracks.get(name).ok_or_else(|| {
                format!(
                    "Actor {} source clip {name} lacks timing/notifies",
                    source.model
                )
            })?;

            let seconds = track.frames as f32 / track.fps;

            let notifications = track
                .notetracks
                .iter()
                .map(|(event, frame)| {
                    let seconds = *frame / track.fps;

                    AnimationNotification {
                        seconds,
                        event: event.clone(),
                    }
                })
                .collect::<Vec<_>>();
            animation.path = label.clone();
            animation.seconds = seconds;
            animation.notifications = notifications;
        }
    }
    Ok(())
}
