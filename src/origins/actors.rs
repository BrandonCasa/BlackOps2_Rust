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
        if hits.is_empty() { return Err(format!("Enemy {} has no source-authored native hit volumes", enemy.id.0)); }
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
    if game.content.enemies.is_empty() {
        return Err("Required source-authored enemy catalog is missing".into());
    }
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
            let path = presentation.scene.as_ref().ok_or_else(|| format!("Source-static actor part {} has no ordinary scene binding", presentation.id.0))?;
            super::validate_asset_path(path).map_err(|error| error.to_string())?;
            let file = path.split('#').next().unwrap_or("");
            if !root.join(file).is_file() {
                return Err(format!("Required source-static actor part scene {file} is missing"));
            }
            continue;
        } else {
            return Err(format!("Actor presentation {} has no exact exported animated source binding", presentation.id.0));
        };
        super::validate_asset_path(&source.scene).map_err(|error| error.to_string())?;
        if !root.join(&source.scene).is_file() {
            return Err(format!("Required actor scene {} is missing", source.scene));
        }
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
            super::validate_asset_path(label).map_err(|error| error.to_string())?;
            let track = tracks.get(name).ok_or_else(|| {
                format!(
                    "Actor {} source clip {name} lacks timing/notifies",
                    source.model
                )
            })?;
            if !track.fps.is_finite() || track.fps <= 0. || track.frames == 0 {
                return Err(format!(
                    "Actor {} source clip {name} has invalid timing",
                    source.model
                ));
            }
            let seconds = track.frames as f32 / track.fps;
            if (animation.seconds - seconds).abs() > 0.001 {
                return Err(format!(
                    "Authored actor clip {} duration disagrees with source",
                    animation.id.0
                ));
            }
            let notifications = track
                .notetracks
                .iter()
                .map(|(event, frame)| {
                    let seconds = *frame / track.fps;
                    if !seconds.is_finite()
                        || seconds < 0.
                        || seconds > track.frames as f32 / track.fps + 0.001
                    {
                        return Err(format!(
                            "Actor {} source notify {event} has invalid frame",
                            source.model
                        ));
                    }
                    Ok(AnimationNotification {
                        seconds,
                        event: event.clone(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            animation.path = label.clone();
            animation.seconds = seconds;
            animation.notifications = notifications;
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authored_enemy_bundle_preserves_all_existing_source_identities() {
        let data: ActorCatalog = serde_json::from_str(include_str!("../../gameplay/origins/enemies.json")).unwrap();
        assert_eq!(data.enemies.len(),51);
        assert!(data.enemies.iter().any(|enemy|enemy.id.0=="enemy.zombie"));
        assert!(data.enemies.iter().any(|enemy|enemy.id.0=="enemy.panzer"));
        assert!(data.enemies.iter().all(|enemy|!enemy.hit_regions.is_empty()));
        for animation in &data.animations {
            assert!(animation.seconds>0. && animation.path.contains("#Animation"));
            assert!(animation.root_motion.as_ref().is_none_or(|motion|motion.valid()));
        }
    }

    #[test]
    fn recovered_native_actor_data_covers_every_source_model_and_human_ragdoll() {
        let source: BTreeMap<String, NativeActorData> = serde_json::from_str(include_str!("../../gameplay/origins/actors.json")).unwrap();
        assert_eq!(source.len(), 31);
        assert_eq!(source.values().map(|model|model.hit_regions.len()).sum::<usize>(),463);
        assert_eq!(source.values().filter(|model|model.ragdoll.is_some()).count(),13);
        let body=&source["c_zom_tomb_german_body_1a"];
        assert!(body.hit_regions.iter().any(|hit|hit.joint=="j_spinelower"));
        let ragdoll=body.ragdoll.as_ref().unwrap();
        assert_eq!(ragdoll.bones.len(),12);
        assert_eq!(ragdoll.joints.len(),11);
        assert_eq!(ragdoll.baselerp.len(),5);
        assert!(ragdoll.bones.iter().all(|bone|bone.base.iter().all(|value|value.is_finite()) && bone.mass>0. && bone.radius>0.));
        assert!(source["c_zom_mech_body"].ragdoll.is_none());
        assert!(source["veh_t6_dlc_zm_robot"].physics.is_none());
        for model in source.values() { for hit in &model.hit_regions {
            assert!((0..3).all(|axis|hit.bounds[0][axis]<hit.bounds[1][axis]));
            assert!(hit.sphere[3]>0.);
        }}
    }
}
