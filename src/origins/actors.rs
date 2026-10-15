//! Bind existing authored enemy animation records to exact external scene labels.
use super::{AnimationNotification, GameDefinition, HitVolumeDefinition, EnemyPhysics, EnemyRagdoll};
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
struct NativeActorData {
    hit_regions: Vec<HitVolumeDefinition>,
    physics: Option<EnemyPhysics>,
    ragdoll: Option<EnemyRagdoll>,
}

fn bind_native_data(game: &mut GameDefinition) -> Result<(), String> {
    let source: BTreeMap<String, NativeActorData> = serde_json::from_str(include_str!("../../gameplay/origins/actors.json"))
        .map_err(|error| format!("Invalid source-authored actor collision data: {error}"))?;
    for enemy in &mut game.content.enemies {
        let presentation = enemy.presentation.as_ref()
            .ok_or_else(|| format!("Enemy {} has no source presentation", enemy.id.0))?;
        let model = presentation.0.strip_prefix("actor.")
            .ok_or_else(|| format!("Enemy {} lacks exact source model identity", enemy.id.0))?;
        let body = source.get(model).ok_or_else(|| format!("Enemy {} source model {model} has no recovered native collision data", enemy.id.0))?;
        let mut hits = body.hit_regions.clone();
        for part in &enemy.parts {
            let model = part.presentation.0.strip_prefix("actor.")
                .ok_or_else(|| format!("Enemy {} part has no source model identity", enemy.id.0))?;
            // Source hats are cosmetic attachments; native damage volumes belong to body/head.
            if matches!(model, "c_zom_tomb_german_hat_1" | "c_zom_tomb_german_hat_2" | "c_zom_tomb_german_hat_3" | "c_zom_tomb_crusader_hat_3") { continue; }
            let part = source.get(model).ok_or_else(|| format!("Enemy {} part model {model} has no recovered native collision data", enemy.id.0))?;
            hits.extend(part.hit_regions.iter().cloned());
        }
        enemy.hit_regions = hits;
        if let Some(physics) = body.physics { enemy.physics = Some(physics); }
        if let Some(ragdoll) = &body.ragdoll { enemy.ragdoll = Some(ragdoll.clone()); }
    }
    Ok(())
}

#[derive(Deserialize)]
struct ActorCatalog {
    enemies: Vec<super::EnemyDefinition>,
    presentations: Vec<super::PresentationDefinition>,
    animations: Vec<super::AnimationDefinition>,
}

pub(crate) fn populate(game: &mut GameDefinition) -> Result<(), String> {
    let source: ActorCatalog = serde_json::from_str(include_str!("../../gameplay/origins/enemies.json"))
        .map_err(|error| format!("Invalid source-authored enemy catalog: {error}"))?;
    if game.content.enemies.is_empty() { game.content.enemies = source.enemies; }
    for presentation in source.presentations {
        if let Some(existing) = game.content.presentations.iter_mut().find(|existing| existing.id == presentation.id) {
            *existing = presentation;
        } else { game.content.presentations.push(presentation); }
    }
    for animation in source.animations {
        if let Some(existing) = game.content.animations.iter_mut().find(|existing| existing.id == animation.id) {
            *existing = animation;
        } else { game.content.animations.push(animation); }
    }
    Ok(())
}

/// Gameplay definitions remain authoritative for actor identity, hit volumes,
/// physics, ragdolls, state selection and traversal root motion.
pub(crate) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    populate(game)?;
    bind_native_data(game)?;
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
            return Err(format!("Actor presentation {} has no exact exported animated source binding", presentation.id.0));
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

