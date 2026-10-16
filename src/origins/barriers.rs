//! Window actions use the same source ASD and raw animation tracks as their actors.
use super::{
    actors::{ClipTrack, Scene},
    *,
};
use crate::barriers::{BarrierActorAction, BarrierActorClip, BarrierEntryDefinition};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Deserialize)]
struct State {
    clips: Vec<(Option<String>, String)>,
}

pub(super) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    let states: BTreeMap<String, State> =
        read(&root.join("actors/zm_tomb_basic/source-states.json"));
    let tracks: BTreeMap<String, ClipTrack> =
        read(&root.join("actors/zm_tomb_basic/source-tracks.json"));
    let action = |name: &str| -> Result<BarrierActorClip, String> {
        let track = tracks
            .get(name)
            .ok_or_else(|| format!("Required window source track {name}"))?;
        let duration = track.frames as f32 / track.fps;
        Ok(BarrierActorClip {
            name: name.into(),
            seconds: duration,
            duration,
            hits: track
                .notetracks
                .iter()
                .filter(|(event, _)| event == "fire")
                .map(|(_, frame)| frame / track.fps)
                .collect(),
            motion: track.motion()?,
        })
    };
    let state = |name: &str| {
        states
            .get(name)
            .ok_or_else(|| format!("Required window source state {name}"))
    };
    let clips = |name: &str| -> Result<Vec<BarrierActorClip>, String> {
        state(name)?
            .clips
            .iter()
            .map(|(_, name)| action(name))
            .collect()
    };
    let entries = |name: &str| -> Result<BTreeMap<String, Vec<BarrierActorClip>>, String> {
        let mut result: BTreeMap<String, Vec<BarrierActorClip>> = BTreeMap::new();
        for (alias, name) in &state(name)?.clips {
            let alias = alias
                .as_ref()
                .ok_or_else(|| format!("Window entry {name} lacks source alias"))?;
            result.entry(alias.clone()).or_default().push(action(name)?);
        }
        Ok(result)
    };
    let enter = BarrierEntryDefinition {
        standing: entries("zm_barricade_enter")?,
        crawler: entries("zm_barricade_enter_crawl")?,
    };
    let reach = BarrierActorAction {
        state: "zm_window_melee".into(),
        frequency: 50,
        range: 109.8 * 0.0254,
        clips: clips("zm_window_melee")?,
    };
    let taunt = BarrierActorAction {
        state: "zm_taunt".into(),
        frequency: 5,
        range: 0.,
        clips: clips("zm_taunt")?,
    };
    // These source states are used explicitly by the window owner rather than
    // ordinary walk/attack role selection. Register their exact exported labels.
    let required: BTreeSet<_> = states
        .iter()
        .filter(|(name, _)| {
            name.starts_with("zm_zbarrier_board_tear_")
                || name.starts_with("zm_barricade_enter")
                || matches!(name.as_str(), "zm_window_melee" | "zm_taunt")
        })
        .flat_map(|(_, state)| state.clips.iter().map(|(_, name)| name.as_str()))
        .collect();
    let scenes: Vec<Scene> = read(&root.join("source-actor-clips.json"));
    for scene in scenes.iter().filter(|s| s.machine == "zm_tomb_basic") {
        let skeleton = ContentId(format!("actor.{}", scene.model));
        for name in &required {
            let path = scene
                .tracks
                .iter()
                .find(|t| t.name == *name)
                .and_then(|t| t.animation_path.as_ref())
                .ok_or_else(|| {
                    format!("Required window clip {name} missing for {}", scene.model)
                })?;
            let id = ContentId(format!("animation.{}.{name}", scene.model));
            if game.content.animations.iter().any(|a| a.id == id) {
                continue;
            }
            let track = &tracks[*name];
            game.content.animations.push(AnimationDefinition {
                id: id.clone(),
                path: path.clone(),
                skeleton: skeleton.clone(),
                seconds: track.frames as f32 / track.fps,
                looping: false,
                // The window action owns scripted motion. Actor bindings only
                // need the pose clip and its timing; do not copy root keys per model.
                root_motion: None,
                notifications: track
                    .notetracks
                    .iter()
                    .map(|(event, frame)| AnimationNotification {
                        event: event.clone(),
                        seconds: frame / track.fps,
                    })
                    .collect(),
            });
            if let Some(p) = game
                .content
                .presentations
                .iter_mut()
                .find(|p| p.id == skeleton)
            {
                p.animations.push(id);
            }
        }
    }
    for window in &mut game.content.barriers {
        window.enter = Some(enter.clone());
        window.reach_through = Some(reach.clone());
        window.taunt = Some(taunt.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires exported installed Origins assets"]
    fn exported_windows_have_distinct_board_roots_and_complete_actor_actions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let game = GameDefinition::load_from_root(&root);
        for window in &game.content.barriers {
            let mut positions = Vec::new();
            for board in &window.boards {
                let repair = game
                    .content
                    .animations
                    .iter()
                    .find(|a| a.id == board.repair)
                    .unwrap();
                let pull = game
                    .content
                    .animations
                    .iter()
                    .find(|a| a.id == board.pull)
                    .unwrap();
                let closed = bevy::prelude::Vec3::from_array(
                    repair
                        .root_motion
                        .as_ref()
                        .unwrap()
                        .translation_at(repair.seconds),
                );
                let start = bevy::prelude::Vec3::from_array(
                    pull.root_motion.as_ref().unwrap().translation_at(0.),
                );
                assert!(
                    closed.distance(start) < 0.001,
                    "board jumps before tear: {}",
                    repair.id.0
                );
                assert!(
                    positions
                        .iter()
                        .all(|p: &bevy::prelude::Vec3| p.distance(closed) > 0.05)
                );
                positions.push(closed);
            }
            for gait in [
                crate::rounds::Gait::Walk,
                crate::rounds::Gait::Run,
                crate::rounds::Gait::Sprint,
            ] {
                for slot in 0..3 {
                    for crawler in [false, true] {
                        let clip = window.entry_clip(gait, slot, crawler, 0).unwrap();
                        assert!(clip.motion.is_some());
                        assert!(
                            game.content
                                .animations
                                .iter()
                                .any(|a| a.id.0.rsplit('.').next() == Some(clip.name.as_str()))
                        );
                    }
                }
            }
            let reach = window.reach_through.as_ref().unwrap();
            assert_eq!(reach.frequency, 50);
            assert!(reach.clips.iter().all(|c| !c.hits.is_empty()));
        }
    }
}
