//! Bind existing authored enemy animation records to exact external scene labels.
use super::{
    AnimationDefinition, AnimationNotification, AnimationRootMotion, ContentId, GameDefinition,
};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(Deserialize)]
pub(super) struct Scene {
    pub(super) model: String,
    pub(super) machine: String,
    pub(super) scene: String,
    pub(super) tracks: Vec<SceneClip>,
}
#[derive(Deserialize)]
pub(super) struct SceneClip {
    pub(super) name: String,
    pub(super) animation_path: Option<String>,
}
#[derive(Deserialize)]
pub(super) struct ClipTrack {
    pub(super) fps: f32,
    pub(super) frames: u32,
    pub(super) notetracks: Vec<(String, f32)>,
    #[serde(default)]
    looped: bool,
    delta_translation: Option<DeltaTrack>,
    delta_rotation: Option<DeltaTrack>,
}

#[derive(Deserialize)]
struct DeltaTrack {
    indices: Vec<f32>,
    values: Vec<Vec<f32>>,
}

impl ClipTrack {
    pub(super) fn motion(&self) -> Result<Option<AnimationRootMotion>, String> {
        let keys = |indices: &[f32], values: &[Vec<f32>], width: usize| {
            indices.len() == values.len()
                && indices.iter().all(|v| v.is_finite())
                && indices.windows(2).all(|p| p[0] < p[1])
                && values
                    .iter()
                    .all(|v| v.len() == width && v.iter().all(|v| v.is_finite()))
        };
        let mut translation = Vec::new();
        if let Some(track) = &self.delta_translation {
            if !keys(&track.indices, &track.values, 3) {
                return Err("invalid source delta translation keys".into());
            }
            translation.extend(track.indices.iter().zip(&track.values).map(|(frame, v)| {
                [
                    frame / self.fps,
                    v[0] * 0.0254,
                    v[2] * 0.0254,
                    -v[1] * 0.0254,
                ]
            }));
        }
        let mut rotation = Vec::new();
        if let Some(track) = &self.delta_rotation {
            if !keys(&track.indices, &track.values, 4) {
                return Err("invalid source delta rotation keys".into());
            }
            rotation.extend(
                track
                    .indices
                    .iter()
                    .zip(&track.values)
                    .map(|(frame, v)| [frame / self.fps, v[0], v[2], -v[1], v[3]]),
            );
        }
        Ok(
            (!translation.is_empty() || !rotation.is_empty()).then_some(AnimationRootMotion {
                seconds: self.frames as f32 / self.fps,
                translation,
                rotation,
            }),
        )
    }
}

#[derive(Deserialize)]
struct ActorCatalog {
    enemies: Vec<super::EnemyDefinition>,
    presentations: Vec<super::PresentationDefinition>,
    animations: Vec<super::AnimationDefinition>,
}
#[derive(Deserialize)]
struct SourceState {
    clips: Vec<(Option<String>, String)>,
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
        let states: BTreeMap<String, SourceState> =
            super::read(&root.join(format!("actors/{}/source-states.json", source.machine)));
        for (state, name) in [
            (super::EnemyCombat::LightningDeath, "zm_death_tesla"),
            (
                super::EnemyCombat::LightningDeathCrawl,
                "zm_death_tesla_crawl",
            ),
            (super::EnemyCombat::ElectricStun, "zm_electric_stun"),
        ] {
            let Some(authored) = states.get(name) else {
                continue;
            };
            let mut clips = Vec::new();
            for (_, clip_name) in &authored.clips {
                let track = tracks
                    .get(clip_name)
                    .ok_or_else(|| format!("{model} lacks source {name} clip {clip_name}"))?;
                let path = source
                    .tracks
                    .iter()
                    .find(|clip| clip.name == *clip_name)
                    .and_then(|clip| clip.animation_path.as_ref())
                    .ok_or_else(|| format!("{model} lacks exported {name} clip {clip_name}"))?;
                let duration = track.frames as f32 / track.fps;
                let id = ContentId(format!("animation.{model}.{clip_name}"));
                let seconds = track
                    .notetracks
                    .iter()
                    .find(|(event, _)| event == "end")
                    .map_or(duration, |(_, frame)| frame / track.fps);
                if !game
                    .content
                    .animations
                    .iter()
                    .any(|animation| animation.id == id)
                {
                    game.content.animations.push(AnimationDefinition {
                        id: id.clone(),
                        path: path.clone(),
                        skeleton: presentation.id.clone(),
                        seconds: duration,
                        looping: track.looped,
                        notifications: track
                            .notetracks
                            .iter()
                            .map(|(event, frame)| AnimationNotification {
                                event: event.clone(),
                                seconds: frame / track.fps,
                            })
                            .collect(),
                        root_motion: track.motion()?,
                    });
                }
                if !presentation.animations.contains(&id) {
                    presentation.animations.push(id.clone());
                }
                clips.push(super::EnemyCombatClip {
                    animation: id,
                    seconds,
                    duration: Some(duration),
                    hits: Vec::new(),
                    stops: Vec::new(),
                });
            }
            for enemy in game
                .content
                .enemies
                .iter_mut()
                .filter(|enemy| enemy.presentation.as_ref() == Some(&presentation.id))
            {
                enemy.combat_states.insert(state, clips.clone());
            }
        }
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
    bind_scripted(game, root)?;
    super::barriers::bind(game, root)
}

/// Scripted map actors are not ordinary enemies. Their exact source clips still
/// belong in the same animation catalog, including hatch and Panzer actions.
fn bind_scripted(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    #[derive(Deserialize)]
    struct Model {
        model: String,
        tracks: Vec<Track>,
    }
    #[derive(Deserialize)]
    struct Track {
        name: String,
        animation_path: Option<String>,
        source: Option<ClipTrack>,
    }
    let models: Vec<Model> = super::read(&root.join("source-model-clips.json"));
    for model in [
        "veh_t6_dlc_zm_robot_0",
        "veh_t6_dlc_zm_robot_1",
        "veh_t6_dlc_zm_robot_2",
        "veh_t6_dlc_zm_robot_foot_hatch",
        "c_zom_mech_body",
        "c_zom_mech_claw",
        "p6_anim_zm_barricade_board_01",
        "p6_anim_zm_barricade_board_02",
        "p6_anim_zm_barricade_board_03",
        "p6_anim_zm_barricade_board_04",
        "p6_anim_zm_barricade_board_05",
        "p6_anim_zm_barricade_board_06",
    ] {
        let source = models
            .iter()
            .find(|entry| entry.model == model)
            .ok_or_else(|| {
                format!("Required scripted actor {model} has no exported clip bindings")
            })?;
        let skeleton = ContentId(format!("actor.{model}"));
        let presentation = game
            .content
            .presentations
            .iter_mut()
            .find(|p| p.id == skeleton)
            .ok_or_else(|| format!("Required scripted actor {model} has no presentation"))?;
        for clip in &source.tracks {
            let Some(path) = &clip.animation_path else {
                continue;
            };
            let track = clip.source.as_ref().ok_or_else(|| format!(
                "Scripted actor {model} clip {} lacks source timing; rerun export_animations.py", clip.name
            ))?;
            if !track.fps.is_finite() || track.fps <= 0. || track.frames == 0 {
                return Err(format!(
                    "Scripted actor {model} clip {} has invalid duration",
                    clip.name
                ));
            }
            if !root.join(path.split('#').next().unwrap()).is_file() {
                return Err(format!(
                    "Scripted actor {model} clip {} is missing: {path}",
                    clip.name
                ));
            }
            let id = ContentId(format!("animation.{model}.{}", clip.name));
            let animation = AnimationDefinition {
                id: id.clone(),
                path: path.clone(),
                skeleton: skeleton.clone(),
                seconds: track.frames as f32 / track.fps,
                notifications: track
                    .notetracks
                    .iter()
                    .map(|(event, frame)| AnimationNotification {
                        event: event.clone(),
                        seconds: frame / track.fps,
                    })
                    .collect(),
                looping: track.looped,
                root_motion: track
                    .motion()
                    .map_err(|e| format!("{model} {}: {e}", clip.name))?,
            };
            if let Some(existing) = game.content.animations.iter_mut().find(|a| a.id == id) {
                *existing = animation;
            } else {
                game.content.animations.push(animation);
            }
            if !presentation.animations.contains(&id) {
                presentation.animations.push(id);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_staff_reactions_bind_the_original_asd_clips_and_notetracks() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let game = GameDefinition::load_from_root(&root);
        let mut count = 0;
        for enemy in game
            .content
            .enemies
            .iter()
            .filter(|enemy| enemy.family == "german")
        {
            for state in [
                super::super::EnemyCombat::LightningDeath,
                super::super::EnemyCombat::LightningDeathCrawl,
                super::super::EnemyCombat::ElectricStun,
            ] {
                let clips = &enemy.combat_states[&state];
                assert!(!clips.is_empty());
                for clip in clips {
                    let animation = game
                        .content
                        .animations
                        .iter()
                        .find(|a| a.id == clip.animation)
                        .unwrap();
                    assert!(
                        root.join(animation.path.split('#').next().unwrap())
                            .is_file()
                    );
                    assert!(animation.seconds > 0.);
                    assert_eq!(clip.duration, Some(animation.seconds));
                    if state != super::super::EnemyCombat::ElectricStun {
                        assert!(
                            animation
                                .notifications
                                .iter()
                                .any(|note| note.event == "die")
                        );
                    }
                }
            }
            count += 1;
        }
        assert!(count > 10);
    }

    #[test]
    fn scripted_delta_keeps_absolute_start_and_converts_source_axes() {
        let track: ClipTrack = serde_json::from_value(serde_json::json!({
            "fps": 30., "frames": 30, "notetracks": [],
            "delta_translation": { "indices": [0, 30], "values": [[100., 200., 300.], [200., 400., 600.]] },
            "delta_rotation": { "indices": [0], "values": [[0., 0., 0.70710677, 0.70710677]] }
        })).unwrap();
        let motion = track.motion().unwrap().unwrap();
        let first = motion.translation_at(0.);
        assert!((first[0] - 2.54).abs() < 0.00001);
        assert!((first[1] - 7.62).abs() < 0.00001);
        assert!((first[2] + 5.08).abs() < 0.00001);
        assert!(motion.rotation_at(0.)[1] > 0.7);
        assert!((motion.translation_at(0.5)[0] - 3.81).abs() < 0.00001);
    }

    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_robot_and_panzer_actions_have_source_timing_and_motion() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let game = GameDefinition::load_from_root(&root);
        for model in [
            "veh_t6_dlc_zm_robot_0",
            "veh_t6_dlc_zm_robot_1",
            "veh_t6_dlc_zm_robot_2",
        ] {
            let intro = game
                .content
                .animations
                .iter()
                .find(|a| {
                    a.skeleton.0 == format!("actor.{model}")
                        && a.id.0.ends_with(".ai_zombie_giant_robot_bunker_intro")
                })
                .unwrap();
            assert_eq!(intro.seconds, 38.5);
            assert!(intro.root_motion.as_ref().unwrap().translation_at(0.) != [0.; 3]);
            assert!(
                intro
                    .notifications
                    .iter()
                    .any(|n| n.event == "footstep_left_large" && n.seconds < 0.04)
            );
            assert!(!intro.notifications.iter().any(|n| n.event == "end"));
        }
        for name in [
            "ai_zombie_giant_robot_hatch_open",
            "ai_zombie_giant_robot_hatch_close",
        ] {
            assert_eq!(
                game.content
                    .animations
                    .iter()
                    .find(|a| a.id.0.ends_with(&format!(".{name}")))
                    .unwrap()
                    .seconds,
                1.
            );
        }
        for name in [
            "ai_zombie_mech_intro_jump_in",
            "ai_zombie_mech_run",
            "ai_zombie_mech_ft_intro_sprint_to_aim_5",
            "ai_zombie_mech_ft_burn_player",
            "ai_zombie_mech_grapple_intro_sprint_to_aim_5",
            "ai_zombie_mech_melee_a",
            "ai_zombie_mech_melee_b",
            "ai_zombie_mech_injury_hit_by_footstep",
            "ai_zombie_mech_injury_recover_from_tank",
        ] {
            let clip = game
                .content
                .animations
                .iter()
                .find(|a| {
                    a.skeleton.0 == "actor.c_zom_mech_body"
                        && a.id.0.rsplit('.').next() == Some(name)
                })
                .unwrap();
            assert!(clip.seconds > 0.);
            assert!(root.join(clip.path.split('#').next().unwrap()).is_file());
        }
    }
}
