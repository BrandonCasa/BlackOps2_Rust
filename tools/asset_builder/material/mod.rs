use crate::{catalog::Catalog, core::*, fastfile::Address, pack::Pack, texture};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

mod camo;
mod character;
pub(crate) mod contract;
mod coverage;
mod effect_model;
mod falloff;
mod ordinary;
mod read;
mod single_unlit;
mod smoke;
#[cfg(test)]
mod tests;
mod unlit;
mod world;

#[cfg(test)]
pub use camo::{camo_variants, read_camo_options, read_camos, read_material_selection};
pub use camo::{finish_camos, prepare_library};
pub use coverage::{accept_initial_visual_approximations, missing, unimplemented};
use falloff::{FalloffRecipe, convert_falloff, falloff_recipe, validate_falloff};
#[cfg(test)]
pub use ordinary::pbr_pixel;
use ordinary::{OrdinaryRecipe, ordinary_recipe, validate_ordinary};
pub use read::read;
pub use read::read_used;
use single_unlit::{
    SingleUnlitRecipe, convert_single_unlit, single_unlit_recipe, validate_single_unlit,
};
use unlit::{UnlitRecipe, convert_unlit, unlit_recipe, validate_unlit};

fn store_image(
    source: &texture::Image,
    uri: String,
    srgb: bool,
    role: &str,
    pack: &mut Pack,
) -> Result<String> {
    let bytes = texture::encode(source, srgb, role)?;
    texture::validate(&bytes)?;
    if pack.entries.insert(uri.clone(), Arc::new(bytes)).is_some() {
        return Err(format!("Duplicate converted material texture: {uri}").into());
    }
    Ok(uri)
}

enum Recipe {
    Ordinary(OrdinaryRecipe),
    Unlit(UnlitRecipe),
    SingleUnlit(SingleUnlitRecipe),
    Falloff(FalloffRecipe),
    Character(character::CharacterRecipe),
    EffectModel(effect_model::EffectModelRecipe),
    Smoke(smoke::SmokeRecipe),
    World(world::WorldRecipe),
    Baked,
}

impl Recipe {
    fn lookup(family: &str) -> Option<Self> {
        ordinary_recipe(family)
            .map(Self::Ordinary)
            .or_else(|| unlit_recipe(family).map(Self::Unlit))
            .or_else(|| single_unlit_recipe(family).map(Self::SingleUnlit))
            .or_else(|| falloff_recipe(family).map(Self::Falloff))
            .or_else(|| character::lookup(family).map(Self::Character))
            .or_else(|| effect_model::lookup(family).map(Self::EffectModel))
            .or_else(|| smoke::lookup(family).map(Self::Smoke))
            .or_else(|| world::lookup(family).map(Self::World))
            .or_else(|| crate::bake::supported(family).then_some(Self::Baked))
    }

    fn technique(&self) -> usize {
        match self {
            Self::Ordinary(_) | Self::Baked => 4,
            Self::Unlit(_) => 2,
            Self::SingleUnlit(recipe) => recipe.index,
            Self::Falloff(recipe) => recipe.index,
            Self::Character(recipe) => recipe.index(),
            Self::EffectModel(recipe) => recipe.index(),
            Self::Smoke(recipe) => recipe.index(),
            Self::World(recipe) => recipe.index(),
        }
    }

    fn validate(&self, key: &str, material: &Value) -> Result<()> {
        match self {
            Self::Ordinary(recipe) => validate_ordinary(key, material, recipe),
            Self::Unlit(recipe) => validate_unlit(key, material, *recipe).map(|_| ()),
            Self::SingleUnlit(recipe) => validate_single_unlit(key, material, *recipe).map(|_| ()),
            Self::Falloff(recipe) => validate_falloff(key, material, *recipe).map(|_| ()),
            Self::Character(recipe) => recipe.validate(key, material),
            Self::EffectModel(recipe) => recipe.validate(key, material),
            Self::Smoke(recipe) => recipe.validate(key, material),
            Self::World(recipe) => recipe.validate(key, material),
            Self::Baked => crate::bake::validate_native(material),
        }
    }

    fn convert(
        &self,
        key: &str,
        material: &Value,
        images: &BTreeMap<String, texture::Image>,
        shared: &mut BTreeMap<(String, String), String>,
        pack: &mut Pack,
    ) -> Result<Option<Value>> {
        Ok(Some(match self {
            Self::Ordinary(recipe) => {
                ordinary::convert(key, material, *recipe, images, shared, pack)?
            }
            Self::Unlit(recipe) => convert_unlit(key, material, *recipe, images, shared, pack)?,
            Self::SingleUnlit(recipe) => {
                convert_single_unlit(key, material, *recipe, images, shared, pack)?
            }
            Self::Falloff(recipe) => convert_falloff(key, material, *recipe, images, shared, pack)?,
            Self::Character(recipe) => recipe.convert(key, material, images, shared, pack)?,
            Self::EffectModel(recipe) => recipe.convert(key, material, images, shared, pack)?,
            Self::Smoke(recipe) => recipe.convert(key, material, images, shared, pack)?,
            Self::World(recipe) => recipe.convert(key, material, images, shared, pack)?,
            Self::Baked => return Ok(None),
        }))
    }
}

pub fn selected_technique(family: &str) -> Option<usize> {
    Recipe::lookup(family).map(|recipe| recipe.technique())
}

pub(crate) fn script_layer_mode(family: &str) -> Option<&'static str> {
    falloff_recipe(family)
        .map(|recipe| recipe.mode)
        .or_else(|| {
            single_unlit_recipe(family)
                .filter(|recipe| matches!(recipe.mode, "layer" | "layer_reflection"))
                .map(|_| "single")
        })
}

pub(crate) fn validate_script_layers(native: &Value) -> Result<usize> {
    let name = native["name"]
        .as_str()
        .ok_or("Script-layer material name is missing")?;
    let family = native["technique_set"]
        .as_str()
        .ok_or("Script-layer material family is missing")?;
    if let Some(recipe) = falloff_recipe(family) {
        validate_falloff(name, native, recipe).map(|(index, _)| index)
    } else {
        let recipe = single_unlit_recipe(family).ok_or("Unsupported script-layer material")?;
        validate_single_unlit(name, native, recipe).map(|(index, _, _)| index)
    }
}

pub(crate) fn script_layer_has_reflection(family: &str) -> bool {
    falloff_recipe(family).map_or_else(
        || single_unlit_recipe(family).is_some_and(|recipe| recipe.mode == "layer_reflection"),
        |recipe| !matches!(recipe.mode, "normal_script_unlit" | "zfeather"),
    )
}

pub(crate) fn validate_static_alpha_reveal(native: &Value) -> Result<bool> {
    let family = native["technique_set"]
        .as_str()
        .ok_or("Missing static alpha-reveal family")?;
    if !matches!(
        family,
        "mc_lit_sm_b0c0n0v0_3559e8jj"
            | "wpc_lit_sm_b0c0n0v0_ew347fe3"
            | "wpc_lit_sm_b0c0n0s0v0_efz5w56f"
    ) {
        return Err(format!("Unreviewed static alpha-reveal family {family}").into());
    }
    let recipe = ordinary_recipe(family).ok_or("Static alpha-reveal ordinary recipe is absent")?;
    validate_ordinary(
        native["name"]
            .as_str()
            .ok_or("Static alpha-reveal native name is absent")?,
        native,
        &recipe,
    )?;
    Ok(recipe.specular == 1)
}

pub fn inputs(
    native: &BTreeMap<String, Value>,
    catalog: &Catalog<'_>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut required = std::collections::BTreeSet::new();
    for (key, material) in native {
        let family = material["technique_set"]
            .as_str()
            .ok_or("Missing native material technique")?;
        let recipe = Recipe::lookup(family)
            .ok_or_else(|| format!("No reviewed recipe for {key}: {family}"))?;
        recipe.validate(key, material)?;
        let index = recipe.technique();
        let pass = contract::pass(material, index)?;
        for argument in pass["arguments"]
            .as_array()
            .ok_or("Missing reviewed pass arguments")?
            .iter()
            .filter(|argument| argument["type"] == 2)
        {
            let binding = material["textures"]
                .as_array()
                .unwrap()
                .iter()
                .find(|binding| binding["hash"] == argument["value"])
                .ok_or_else(|| {
                    format!(
                        "Reviewed material {key} lacks active texture {}",
                        argument["value"]
                    )
                })?;
            let name = binding["image"]
                .as_str()
                .ok_or("Missing active image name")?;
            let (zone, root) = catalog.resolve(8, name)?;
            let streams = &catalog.zones[zone].1.streams;
            if streams.u16(root.add(20)?)? == 0
                || streams.u16(root.add(22)?)? == 0
                || streams.u16(root.add(24)?)? != 1
                || streams.u8(root.add(4)?)? == 5
            {
                return Err(format!(
                    "Reviewed material {key} active image {name} is not a valid 2D image"
                )
                .into());
            }
            required.insert(name.to_ascii_lowercase());
        }
    }
    Ok(required)
}

pub fn convert_known(
    native: &BTreeMap<String, Value>,
    images: &BTreeMap<String, texture::Image>,
    pack: &mut Pack,
) -> Result<BTreeMap<String, Value>> {
    let mut converted = BTreeMap::new();
    let mut missing = BTreeSet::new();
    let mut shared = BTreeMap::new();
    for (key, material) in native {
        let family = material["technique_set"]
            .as_str()
            .ok_or("Missing native material technique")?;
        let Some(recipe) = Recipe::lookup(family) else {
            missing.insert(family);
            continue;
        };
        let Some(value) = recipe.convert(key, material, images, &mut shared, pack)? else {
            continue;
        };
        converted.insert(key.clone(), value);
        if converted.len() % 25 == 0 {
            println!(
                "Converted {} reviewed materials with {} shared texture channels",
                converted.len(),
                shared.len()
            );
        }
    }
    println!(
        "Converted {} source-reviewed materials; {} other technique families remain",
        converted.len(),
        missing.len()
    );
    Ok(converted)
}
