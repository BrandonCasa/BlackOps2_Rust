//! Source-authored Origins weapon gameplay data and ordinary exported clip bindings.
use super::{
    AnimationDefinition, ContentId, GameDefinition, LocalizedString, PresentationDefinition,
    UiAsset, WeaponDefinition,
};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};
#[derive(Deserialize)]
struct Catalog {
    weapons: Vec<WeaponDefinition>,
    presentations: Vec<PresentationDefinition>,
    animations: Vec<AnimationDefinition>,
    ui_assets: Vec<UiAsset>,
    definitions: Vec<ContentId>,
    localized_strings: Vec<LocalizedString>,
}
#[derive(Deserialize)]
struct SourceWeapon {
    scene: String,
    clip_bindings: BTreeMap<String, String>,
}
pub(crate) fn bind(game: &mut GameDefinition, root: &Path) -> Result<(), String> {
    let mut catalog: Catalog =
        serde_json::from_str(include_str!("../../gameplay/origins/weapons.json"))
            .map_err(|error| format!("Invalid source-authored weapon catalog: {error}"))?;
    let file = root.join("source-weapons.json");
    let facts: BTreeMap<String, SourceWeapon> =
        serde_json::from_slice(&std::fs::read(&file).map_err(|error| {
            format!(
                "Required weapon source bindings {}: {error}",
                file.display()
            )
        })?)
        .map_err(|error| format!("Invalid weapon source bindings: {error}"))?;
    let file = root.join("source-hands.json");
    let hands: BTreeMap<String, BTreeMap<String, String>> =
        serde_json::from_slice(&std::fs::read(&file).map_err(|error| {
            format!("Required hands source bindings {}: {error}", file.display())
        })?)
        .map_err(|error| format!("Invalid hands source bindings: {error}"))?;
    let hands = hands
        .get("dempsey")
        .ok_or("Required named Dempsey source hand clip bindings absent")?;
    for weapon in &catalog.weapons {
        let name = weapon
            .id
            .0
            .strip_prefix("weapon.")
            .ok_or_else(|| format!("Invalid source weapon ID {}", weapon.id.0))?;
        let Some(source) = facts.get(name) else {
            continue;
        };
        let Some(id) = &weapon.presentation else {
            continue;
        };
        let presentation = catalog
            .presentations
            .iter_mut()
            .find(|p| &p.id == id)
            .ok_or_else(|| format!("Missing weapon presentation {}", id.0))?;
        presentation.scene = Some(source.scene.clone());
        for animation in catalog
            .animations
            .iter_mut()
            .filter(|animation| &animation.skeleton == id)
        {
            let name = animation
                .id
                .0
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_lowercase();
            animation.path = source
                .clip_bindings
                .get(&name)
                .or_else(|| hands.get(&name))
                .cloned()
                .ok_or_else(|| {
                    format!("Required exported weapon animation {name} has no exact asset binding")
                })?;
        }
    }
    game.content.weapons = catalog.weapons;
    macro_rules! merge {
        ($field:ident) => {{
            for entry in catalog.$field {
                game.content.$field.retain(|old| old.id != entry.id);
                game.content.$field.push(entry);
            }
        }};
    }
    merge!(presentations);
    merge!(animations);
    merge!(ui_assets);
    merge!(localized_strings);
    for id in catalog.definitions {
        if !game.content.definitions.contains(&id) {
            game.content.definitions.push(id);
        }
    }
    Ok(())
}
