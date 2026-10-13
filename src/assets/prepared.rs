use bevy::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Resource)]
pub(crate) struct PreparedArt {
    data: Value,
    indices: HashMap<String, HashMap<String, Vec<usize>>>,
}

impl PreparedArt {
    pub(crate) fn load(pack: &crate::pack::Pack) -> Result<Self, String> {
        let bytes = pack
            .get("prepared/runtime.json")
            .map_err(|error| error.to_string())?;
        let data: Value = serde_json::from_slice(bytes)
            .map_err(|error| format!("Invalid runtime art data: {error}"))?;
        if data["schema_version"] != 1 {
            return Err("Unsupported prepared art schema".into());
        }
        let mut indices = HashMap::new();
        for category in ["models", "images", "sounds", "presentations", "ui_assets"] {
            let mut names: HashMap<String, Vec<usize>> = HashMap::new();
            for (index, row) in data[category].as_array().into_iter().flatten().enumerate() {
                let key = if matches!(category, "presentations" | "ui_assets") {
                    "id"
                } else {
                    "name"
                };
                if let Some(name) = row[key].as_str() {
                    names.entry(name.to_owned()).or_default().push(index);
                }
            }
            indices.insert(category.to_owned(), names);
        }
        Ok(Self { data, indices })
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
        self.entries(category, name)
            .min_by_key(|value| Self::priority(value))
    }

    fn entries<'a>(&'a self, category: &str, name: &str) -> impl Iterator<Item = &'a Value> {
        let rows = &self.data[category];
        self.indices
            .get(category)
            .and_then(|names| names.get(name))
            .into_iter()
            .flatten()
            .map(move |index| &rows[*index])
    }

    fn presentation(&self, id: &str) -> Option<&Value> {
        self.entry("presentations", id)
    }

    fn model(&self, scene: &str) -> Option<&Value> {
        let path = scene.split('#').next()?;
        self.data["models"]
            .as_array()?
            .iter()
            .filter(|model| {
                model["uri"].as_str() == Some(path)
                    || model["lods"].as_array().is_some_and(|lods| {
                        lods.iter().any(|lod| lod["uri"].as_str() == Some(path))
                    })
            })
            .min_by_key(|value| Self::priority(value))
    }

    fn ui_image(&self, id: &str, path: &str) -> Result<&str, String> {
        if let Some(binding) = self.entry("ui_assets", id) {
            let name = binding["name"]
                .as_str()
                .ok_or_else(|| format!("UI image has no native identity: {id}"))?;
            let uri = binding["uri"]
                .as_str()
                .ok_or_else(|| format!("UI image has no prepared URI: {id}"))?;
            return self
                .entries("images", name)
                .filter(|image| {
                    binding["zone"]
                        .as_str()
                        .is_none_or(|zone| image["zone"].as_str() == Some(zone))
                })
                .find_map(|image| image["uris"]["srgb"].as_str().filter(|path| *path == uri))
                .ok_or_else(|| {
                    format!("Native UI image binding is not prepared: {id} ({name}, {uri})")
                });
        }
        self.data["images"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|image| image["uris"]["srgb"].as_str().filter(|uri| *uri == path))
            .ok_or_else(|| format!("UI image has no authoritative prepared binding: {id} ({path})"))
    }

    pub(crate) fn world(&self) -> Option<&Value> {
        self.data["worlds"]
            .as_array()?
            .iter()
            .filter(|world| world["map"].as_str() == Some("zm_tomb"))
            .min_by_key(|world| Self::priority(world))
    }

    pub(crate) fn image(&self, name: &str, transfer: &str) -> Option<&str> {
        self.entry("images", name)?["uris"][transfer].as_str()
    }

    pub(crate) fn effects_uri(&self) -> Option<&str> {
        self.data["effects_uri"].as_str()
    }

    pub(crate) fn camo_uri(&self) -> Option<&str> {
        self.data["camo_uri"].as_str()
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
        let mut skeletons = HashMap::new();
        for presentation in &mut game.content.presentations {
            let Some(scene) = &mut presentation.scene else {
                continue;
            };
            let binding = self.presentation(&presentation.id.0);
            let model = binding.or_else(|| self.model(scene)).ok_or_else(|| {
                format!(
                    "Native presentation is not prepared: {} ({scene})",
                    presentation.id.0
                )
            })?;
            let path = if binding.is_some() {
                uri(model, "uri")?
            } else {
                scene.split('#').next().unwrap_or_default().to_owned()
            };
            skeletons.insert(presentation.id.0.clone(), (model, path.clone()));
            *scene = format!("{path}#Scene0");
        }
        for animation in &mut game.content.animations {
            let (model, path) = skeletons.get(&animation.skeleton.0).ok_or_else(|| {
                format!(
                    "Native animation skeleton is missing: {}",
                    animation.skeleton.0
                )
            })?;
            let clips = model["animations"].as_object().ok_or_else(|| {
                format!(
                    "Native skeleton has no animation bindings: {}",
                    animation.skeleton.0
                )
            })?;
            let mut matches = clips.iter().filter(|(name, _)| {
                animation
                    .id
                    .0
                    .to_ascii_lowercase()
                    .ends_with(&format!(".{}", name.to_ascii_lowercase()))
            });
            let (name, index) = matches
                .next()
                .ok_or_else(|| format!("Native animation is not bound: {}", animation.id.0))?;
            if matches.next().is_some() {
                return Err(format!(
                    "Native animation identity is ambiguous: {}",
                    animation.id.0
                ));
            }
            let index = index
                .as_u64()
                .ok_or_else(|| format!("Invalid native animation index: {name}"))?;
            animation.path = format!("{path}#Animation{index}");
        }
        let mut present: HashSet<_> = game
            .content
            .presentations
            .iter()
            .map(|presentation| presentation.id.0.clone())
            .collect();
        let mut models: Vec<_> = self.data["models"]
            .as_array()
            .into_iter()
            .flatten()
            .collect();
        models.sort_by_key(|model| Self::priority(model));
        for model in models {
            let Some(name) = model["name"].as_str() else {
                continue;
            };
            let id = format!("actor.{name}");
            if !id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
                || !present.insert(id.clone())
            {
                continue;
            }
            game.content
                .presentations
                .push(crate::origins::PresentationDefinition {
                    id: crate::origins::ContentId(id),
                    scene: Some(format!("{}#Scene0", uri(model, "uri")?)),
                    animations: Vec::new(),
                    sounds: Vec::new(),
                });
        }
        for image in &mut game.content.ui_assets {
            image.path = self.ui_image(&image.id.0, &image.path)?.to_owned();
        }
        let semantic_paths: HashSet<_> = game
            .content
            .sounds
            .iter()
            .filter(|sound| !sound.id.0.starts_with("sound.native."))
            .map(|sound| sound.path.clone())
            .collect();
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
            let binding = self
                .entries("sounds", name)
                .filter(|value| value["index"] == index)
                .min_by_key(|value| Self::priority(value))
                .ok_or_else(|| format!("Native audio payload is not prepared: {name}/{index}"))?;
            let prepared = uri(binding, "uri")?;
            if semantic_paths.contains(&sound.path)
                && let Some(previous) = sound_paths.insert(sound.path.clone(), prepared.clone())
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
                if self.data["sounds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|binding| binding["uri"].as_str() == Some(sound.path.as_str()))
                {
                    continue;
                }
                sound.path = sound_paths
                    .get(&sound.path)
                    .ok_or_else(|| format!("Sound has no native alias payload: {}", sound.id.0))?
                    .clone();
            }
        }
        Ok(())
    }
}
