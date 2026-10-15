mod preview;
mod runtime;

use crate::core::*;
use crate::{catalog, fastfile, images, native, pack};
use std::path::{Path, PathBuf};

pub fn installation() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("ProgramFiles(x86)").ok_or("ProgramFiles(x86) is unavailable")?,
    )
    .join("Steam/steamapps/common/Call of Duty Black Ops II"))
}

pub(crate) fn read_zones(install: &Path) -> Result<Vec<(String, native::Zone)>> {
    let authorized = installation()?.canonicalize()?;
    if install.canonicalize()? != authorized {
        return Err("Assets must be read directly from the authorized Steam installation".into());
    }
    let mut zones = Vec::new();
    for (language, name) in [
        ("all", "code_pre_gfx_zm"),
        ("all", "code_post_gfx_zm"),
        ("english", "en_code_post_gfx_zm"),
        ("all", "common_zm"),
        ("english", "en_common_zm"),
        ("all", "zm_tomb"),
        ("english", "en_zm_tomb"),
        ("all", "patch_zm"),
        ("english", "en_patch_zm"),
        ("all", "zm_tomb_patch"),
    ] {
        let path = authorized
            .join("zone")
            .join(language)
            .join(format!("{name}.ff"));
        let bytes = std::fs::read(&path)
            .map_err(|error| format!("Required original zone {}: {error}", path.display()))?;
        let file = fastfile::decode(&bytes, name)?;
        let size = file.bytes.len();
        let blocks = file.compressed_blocks;
        println!("Decoded {name}: {blocks} compressed blocks, {size} bytes");
        let zone = native::Zone::load(file).map_err(|error| format!("Zone {name}: {error}"))?;
        println!(
            "Traversed {name}: {} registered native assets",
            zone.assets.len()
        );
        zones.push((name.to_owned(), zone));
    }
    Ok(zones)
}

pub fn build(install: &Path, output: &Path, preview: Option<&str>) -> Result<()> {
    let authorized = installation()?.canonicalize()?;
    let zones = read_zones(install)?;
    let catalog = catalog::Catalog::new(&zones);
    if let Some(name) = preview {
        return preview::build(&authorized, &catalog, name, output);
    }
    let references: std::collections::BTreeSet<_> = zones
        .iter()
        .flat_map(|(_, zone)| zone.assets.iter())
        .filter(|asset| asset.reference)
        .map(|asset| (asset.kind, asset.name.as_str()))
        .collect();
    let entities = crate::entities::read(&catalog)?;
    crate::scenes::validate_placements(&catalog, &entities)?;
    let mut game = if preview.is_none() {
        Some(crate::game::read(&catalog, &entities)?)
    } else {
        None
    };
    let initial = game
        .as_ref()
        .is_some_and(|game| game.0["delivery_scope"] == "first_playable_solo");
    if !initial {
        for (kind, name) in &references {
            catalog.resolve(*kind, name)?;
        }
        println!("Resolved {} distinct native references", references.len());
    }
    let effects = game
        .as_ref()
        .map(|game| {
            crate::effects::Effects::read(
                &catalog,
                crate::effects::initial_roots(&catalog, &game.0)?,
            )
        })
        .transpose()?;
    let required_models = if initial {
        let mut models = crate::scenes::required_models(&catalog, &game.as_ref().unwrap().0)?;
        models.extend(effects.as_ref().unwrap().models.iter().cloned());
        Some(models)
    } else {
        None
    };
    let used_materials = crate::geometry::material_names(&catalog, required_models.as_ref())?;
    let mut materials = crate::material::read_used(&catalog, &used_materials)?;
    let research =
        PathBuf::from(std::env::var_os("USERPROFILE").ok_or("USERPROFILE is unavailable")?)
            .join("Documents/T6ZMResearch/t6zm-master");
    if !research.is_dir() {
        return Err("The authorized t6zm-master research workspace is missing".into());
    }
    let (mut camo_library, camo_variants) = if initial {
        (None, std::collections::BTreeMap::new())
    } else {
        let (library, variants) = crate::material::prepare_library(&catalog, &mut materials)?;
        (Some(library), variants)
    };
    let game = game
        .as_mut()
        .ok_or("Required native game data was not selected")?;
    let required_animations = initial.then(|| {
        game.0["content"]["animations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|animation| {
                animation["source_clip"].as_str().or_else(|| {
                    animation["id"]
                        .as_str()
                        .and_then(|id| id.rsplit('.').next())
                })
            })
            .map(str::to_ascii_lowercase)
            .collect::<std::collections::BTreeSet<_>>()
    });
    let animations = crate::animation::read(&catalog, required_animations.as_ref())?;
    for definition in game.0["content"]["animations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let name = definition["source_clip"]
            .as_str()
            .ok_or("Selected animation source clip is missing")?;
        let decoded = animations
            .iter()
            .find(|clip| clip.clip.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Selected source animation {name} was not decoded"))?;
        if decoded.has_root_delta && definition["root_motion"] != crate::game::root_motion(decoded)
        {
            return Err(format!("Required animation {name} has native root-delta tracks without matching runtime motion data").into());
        }
    }
    let wide = animations
        .iter()
        .filter(|entry| entry.clip.frames >= 256)
        .count();
    let modes: std::collections::BTreeSet<_> = animations
        .iter()
        .map(|entry| entry.clip.source_flags)
        .collect();
    let jumpcuts = animations
        .iter()
        .filter(|entry| {
            entry.clip.notifies.iter().any(|(name, _)| {
                name.as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case("jumpcut"))
            })
        })
        .count();
    println!(
        "Validated {} native ordinary animation streams ({wide} wide); {} separate root-delta tracks, {} native loop/delta/IK flag patterns and {jumpcuts} jumpcut clips require playback/pose selection before export",
        animations.len(),
        animations
            .iter()
            .filter(|entry| entry.has_root_delta)
            .count(),
        modes.len()
    );
    let unsupported = crate::material::unimplemented(&materials);
    if !unsupported.is_empty() {
        return Err(format!(
            "Required material conversion has {} unimplemented families:\n{}",
            unsupported.len(),
            unsupported.join("\n")
        )
        .into());
    }
    let validated_materials = crate::geometry::validate(&catalog, required_models.as_ref())?;
    if validated_materials != used_materials {
        return Err("Selected model material metadata and decoded source geometry disagree".into());
    }
    let mut pack = pack::Pack::streaming(output)?;
    let collision = crate::collision::convert(&catalog, &mut pack)?;
    crate::game::prepare(game, &entities, &collision)?;
    let required_sounds = initial.then(|| {
        let mut aliases: std::collections::BTreeSet<_> = game.0["audio"]["aliases"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect();
        aliases.extend(
            game.0["audio"]["bindings"]
                .as_object()
                .into_iter()
                .flatten()
                .filter_map(|(_, alias)| alias.as_str())
                .map(str::to_ascii_lowercase),
        );
        aliases.extend(
            game.0["content"]["sounds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|sound| sound["source_alias"].as_str())
                .map(str::to_ascii_lowercase),
        );
        let absent = game.0["audio"]["absent"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_ascii_lowercase)
            .collect::<std::collections::BTreeSet<_>>();
        aliases.retain(|alias| !absent.contains(alias));
        aliases
    });
    let sounds = crate::sound::convert(&authorized, &catalog, required_sounds.as_ref(), &mut pack)?;
    crate::game::bind_audio(&mut game.0, &sounds)?;
    let required_images = if initial {
        let mut images = crate::material::inputs(&materials, &catalog)?;
        if let Some(effects) = &effects {
            images.extend(effects.inputs(&catalog)?);
        }
        images.extend(crate::atmosphere::inputs(&catalog)?);
        for asset in game.0["content"]["ui_assets"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if let Some(name) = asset["source_image"].as_str() {
                catalog.resolve(8, name)?;
                images.insert(name.to_ascii_lowercase());
            }
        }
        Some(images)
    } else {
        None
    };
    let (images, image_sources) =
        images::convert(&authorized, &catalog, &mut pack, required_images.as_ref())?;
    let mut conversion_inputs = materials.clone();
    for (name, material) in camo_variants {
        if conversion_inputs.insert(name.clone(), material).is_some() {
            return Err(format!("Duplicate camo conversion identity {name}").into());
        }
    }
    let mut converted_materials =
        crate::material::convert_known(&conversion_inputs, &image_sources, &mut pack)?;
    if initial {
        for approximation in crate::material::accept_initial_visual_approximations(
            &materials,
            &mut converted_materials,
        )? {
            println!("Initial visual approximation: {approximation}");
        }
    }
    let known_materials = materials
        .iter()
        .filter(|(_, material)| {
            !crate::bake::supported(material["technique_set"].as_str().unwrap_or_default())
        })
        .map(|(name, material)| (name.clone(), material.clone()))
        .collect();
    let known_gaps = crate::material::missing(&known_materials, &converted_materials);
    if !known_gaps.is_empty() {
        return Err(format!(
            "Required non-baked material conversion has {} unresolved materials:\n{}",
            known_gaps.len(),
            known_gaps.join("\n")
        )
        .into());
    }
    let mut geometry = Vec::new();
    for (_, zone, root) in catalog.definitions(17) {
        geometry.extend(crate::geometry::world(&catalog, zone, root)?);
    }
    for (name, zone, root) in catalog.definitions(5) {
        if required_models
            .as_ref()
            .is_some_and(|models| !models.contains(&name.to_ascii_lowercase()))
        {
            continue;
        }
        let streams = &catalog.zones[zone].1.streams;
        let bindings = fastfile::Address::decode(streams.u32(root.add(36)?)?)?;
        let mut needs_bake = false;
        for index in 0..streams.u8(root.add(6)?)? as usize {
            let material = catalog.pointer(zone, bindings.add(index * 4)?, 6)?.0;
            needs_bake |= materials
                .get(&material.to_ascii_lowercase())
                .is_some_and(|material| {
                    crate::bake::supported(material["technique_set"].as_str().unwrap_or_default())
                });
        }
        if needs_bake {
            geometry.extend(
                crate::geometry::model(&catalog, name, zone, root)?
                    .into_iter()
                    .flatten(),
            );
        }
    }
    let baked = crate::bake::surfaces(
        &materials,
        geometry,
        &image_sources,
        &mut converted_materials,
        &mut pack,
    )?;
    if initial {
        for approximation in crate::material::accept_initial_visual_approximations(
            &materials,
            &mut converted_materials,
        )? {
            println!("Initial visual approximation: {approximation}");
        }
    }
    let mut material_gaps = crate::material::missing(&materials, &converted_materials);
    if let Some(camo_library) = &mut camo_library {
        material_gaps.extend(crate::material::finish_camos(
            camo_library,
            &converted_materials,
            &mut pack,
        )?);
    }
    let atmosphere = crate::atmosphere::read(&catalog, &image_sources, &mut pack)?;
    if !material_gaps.is_empty() {
        return Err(format!(
            "Required material conversion has {} unresolved materials:\n{}",
            material_gaps.len(),
            material_gaps.join("\n")
        )
        .into());
    }
    let exported_models = initial.then(|| {
        let mut models = game.0["content"]["presentations"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|presentation| presentation["source_model"].as_str())
            .map(str::to_ascii_lowercase)
            .collect::<std::collections::BTreeSet<_>>();
        models.extend(effects.as_ref().unwrap().models.iter().cloned());
        models
    });
    let mut animation_bindings =
        std::collections::BTreeMap::<String, Vec<&crate::animation::DecodedClip>>::new();
    for definition in game.0["content"]["animations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let name = definition["source_clip"]
            .as_str()
            .or_else(|| {
                definition["id"]
                    .as_str()
                    .and_then(|id| id.rsplit('.').next())
            })
            .ok_or("Selected animation native name is missing")?;
        let clip = animations
            .iter()
            .find(|clip| clip.clip.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| format!("Required native animation {name} was not decoded"))?;
        let presentation = game.0["content"]["presentations"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|presentation| presentation["id"] == definition["skeleton"])
            .ok_or("Selected animation presentation is missing")?;
        let model = presentation["source_model"]
            .as_str()
            .ok_or("Selected animation model is missing")?
            .to_ascii_lowercase();
        let bindings = animation_bindings.entry(model).or_default();
        if !bindings
            .iter()
            .any(|existing| existing.clip.name == clip.clip.name)
        {
            bindings.push(clip);
        }
    }
    let (models, mut worlds) = crate::scenes::convert(
        &catalog,
        &entities,
        exported_models.as_ref(),
        &animation_bindings,
        &converted_materials,
        &baked,
        &mut pack,
    )?;
    crate::scenes::bind_presentations(&mut game.0, &models)?;
    if let Some(effects) = &effects {
        effects.write(&catalog, &models, &images, initial, &mut pack)?;
    }
    for asset in game.0["content"]["ui_assets"]
        .as_array_mut()
        .into_iter()
        .flatten()
    {
        if let Some(name) = asset["source_image"].as_str() {
            let image = images
                .iter()
                .find(|image| {
                    image["name"]
                        .as_str()
                        .is_some_and(|image| image.eq_ignore_ascii_case(name))
                })
                .ok_or_else(|| {
                    format!(
                        "Initial UI asset {} image {name} was not converted",
                        asset["id"]
                    )
                })?;
            asset["path"] = image["uris"]["srgb"].clone();
            asset
                .as_object_mut()
                .ok_or("Initial UI asset is not an object")?
                .remove("source_image");
        }
    }
    let atmosphere_uri = "prepared/worlds/zm_tomb.atmosphere.json";
    pack.entries.insert(
        atmosphere_uri.into(),
        std::sync::Arc::new(serde_json::to_vec(&atmosphere)?),
    );
    for world in &mut worlds {
        if world["map"] == "zm_tomb" {
            world["collision_uri"] = collision["uri"].clone();
            world["collision_meshes"] = collision["meshes"].clone();
            world["atmosphere_uri"] = serde_json::json!(atmosphere_uri);
        }
    }
    crate::game::write(game.0.clone(), &worlds, &mut pack)?;
    let runtime = runtime::assemble(&pack, models, worlds, images, sounds)?;
    pack.entries.insert(
        "prepared/runtime.json".into(),
        std::sync::Arc::new(serde_json::to_vec(&runtime)?),
    );
    crate::validate_pack(&pack)?;
    pack.write_to(output)?;
    println!(
        "Packed {} runtime entries into {}",
        pack.entries.len(),
        output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests;
