use bevy::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Resource)]
pub(crate) struct PreparedArt {
    data: Value,
}

impl PreparedArt {
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("prepared/runtime.json");
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("Cannot read prepared art {}: {error}", path.display()))?;
        let data: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Invalid prepared art {}: {error}", path.display()))?;
        if data["schema_version"] != 1 {
            return Err("Unsupported prepared art schema".into());
        }
        Ok(Self { data })
    }

    fn priority(value: &Value) -> u8 {
        let zone = value["zone"].as_str().unwrap_or_default();
        if zone.ends_with("-zm_tomb_patch") {
            0
        } else if zone.ends_with("-zm_tomb") {
            1
        } else if zone.ends_with("-patch_zm") {
            2
        } else if zone.ends_with("-common_zm") {
            3
        } else if zone.ends_with("-code_post_gfx_zm") {
            4
        } else {
            5
        }
    }

    fn entry(&self, category: &str, name: &str) -> Option<&Value> {
        self.data[category]
            .as_array()?
            .iter()
            .filter(|value| value["name"].as_str() == Some(name))
            .min_by_key(|value| Self::priority(value))
    }

    pub(crate) fn world(&self) -> Option<&Value> {
        self.data["worlds"]
            .as_array()?
            .iter()
            .find(|world| world["map"].as_str() == Some("zm_tomb"))
    }

    pub(crate) fn image(&self, name: &str, transfer: &str) -> Option<&str> {
        self.entry("images", name)?["uris"][transfer].as_str()
    }

    pub(crate) fn bind(&self, game: &mut crate::origins::GameDefinition) -> Result<(), String> {
        let world = self.world().ok_or("Prepared Origins world is missing")?;
        let uri = |value: &Value, key: &str| {
            value[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("Prepared art is missing {key}"))
        };
        game.scene = format!("{}#Scene0", uri(world, "uri")?);
        game.collision_scene = format!("{}#Scene0", uri(world, "collision_uri")?);
        for (key, target) in [
            ("source_surfaces", &mut game.source_surfaces),
            ("render_meshes", &mut game.render_meshes),
            ("collision_meshes", &mut game.collision_meshes),
            ("static_model_instances", &mut game.static_model_instances),
        ] {
            *target = world[key]
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| format!("Prepared world is missing {key}"))?;
        }
        for presentation in &mut game.content.presentations {
            let Some(scene) = &mut presentation.scene else {
                continue;
            };
            let model_name = Path::new(scene.split('#').next().unwrap_or_default())
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix("_lod0"))
                .ok_or_else(|| {
                    format!("Presentation has no native model: {}", presentation.id.0)
                })?;
            let model = self
                .entry("models", model_name)
                .ok_or_else(|| format!("Native model is not prepared: {model_name}"))?;
            *scene = format!("{}#Scene0", uri(model, "uri")?);
        }
        for animation in &mut game.content.animations {
            let model_name =
                animation.skeleton.0.strip_prefix("actor.").ok_or_else(|| {
                    format!("Animation has no native skeleton: {}", animation.id.0)
                })?;
            let model = self
                .entry("models", model_name)
                .ok_or_else(|| format!("Native animation model is missing: {model_name}"))?;
            let prefix = format!("animation.{model_name}.");
            let name =
                animation.id.0.strip_prefix(&prefix).ok_or_else(|| {
                    format!("Animation has no native clip name: {}", animation.id.0)
                })?;
            let index = model["animations"][name]
                .as_u64()
                .ok_or_else(|| format!("Native animation is not bound: {model_name}/{name}"))?;
            animation.path = format!("{}#Animation{index}", uri(model, "uri")?);
        }
        for image in &mut game.content.ui_assets {
            let name = Path::new(&image.path)
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or_else(|| format!("UI image has no native identity: {}", image.id.0))?;
            image.path = self
                .image(name, "srgb")
                .ok_or_else(|| format!("Native UI image is not prepared: {name}"))?
                .to_owned();
        }
        let mut sound_paths = HashMap::new();
        for sound in &mut game.content.sounds {
            let Some((name, index)) = sound
                .id
                .0
                .strip_prefix("sound.native.")
                .and_then(|name| name.rsplit_once('.'))
                .and_then(|(name, index)| index.parse::<u64>().ok().map(|index| (name, index)))
            else {
                continue;
            };
            let binding = self.data["sounds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|value| value["name"] == name && value["index"] == index)
                .min_by_key(|value| Self::priority(value))
                .ok_or_else(|| format!("Native audio payload is not prepared: {name}/{index}"))?;
            let prepared = uri(binding, "uri")?;
            if let Some(previous) = sound_paths.insert(sound.path.clone(), prepared.clone())
                && previous != prepared
            {
                return Err(format!(
                    "Sound aliases disagree on prepared payload: {}",
                    sound.id.0
                ));
            }
            sound.path = prepared;
        }
        for sound in &mut game.content.sounds {
            if !sound.id.0.starts_with("sound.native.") {
                sound.path = sound_paths
                    .get(&sound.path)
                    .ok_or_else(|| format!("Sound has no native alias payload: {}", sound.id.0))?
                    .clone();
            }
        }
        Ok(())
    }
}
