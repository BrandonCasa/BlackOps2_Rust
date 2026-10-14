#[cfg(test)]
use crate::{catalog::Catalog, geometry};
use crate::{core::*, geometry::Surface, pack::Pack, texture};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc};

mod atlas;
mod emissive;
mod gpu;
#[cfg(test)]
mod installed_tests;
mod layered;
mod output;
mod recipe;
mod special;
#[cfg(test)]
mod tests;
mod tiled;
use atlas::*;
#[cfg(test)]
pub use installed_tests::{check_response_geometry, check_special_response};
use output::*;
use recipe::{Input, Recipe, auxiliary};
use special::prepare_special;

enum Family {
    Layered(layered::Definition),
    Tiled(tiled::Definition),
    Emissive(emissive::Definition),
}

fn family(name: &str) -> Option<Family> {
    layered::lookup(name)
        .map(Family::Layered)
        .or_else(|| tiled::lookup(name).map(Family::Tiled))
        .or_else(|| emissive::lookup(name).map(Family::Emissive))
}

impl Family {
    fn read(
        self,
        native: &Value,
        images: Option<&BTreeMap<String, texture::Image>>,
    ) -> Result<Recipe> {
        match self {
            Self::Layered(definition) => layered::read(definition, native, images),
            Self::Tiled(definition) => tiled::read(definition, native, images),
            Self::Emissive(definition) => emissive::read(definition, native, images),
        }
    }

    fn output(&self) -> Output {
        match self {
            Self::Layered(definition) => definition.output,
            Self::Tiled(definition) => Output {
                layer_specular: definition.specular,
                ..Default::default()
            },
            Self::Emissive(_) => Output {
                layer_specular: true,
                ..Default::default()
            },
        }
    }
}

fn special_family(name: &str) -> bool {
    matches!(family(name), Some(Family::Tiled(_) | Family::Emissive(_)))
}

#[cfg(test)]
fn emissive_family(name: &str) -> bool {
    matches!(family(name), Some(Family::Emissive(_)))
}

pub fn supported(name: &str) -> bool {
    family(name).is_some()
}

pub fn validate_native(native: &Value) -> Result<()> {
    recipe(native, None).map(|_| ())
}

#[cfg(test)]
pub fn world(
    catalog: &Catalog<'_>,
    native: &BTreeMap<String, Value>,
    images: &BTreeMap<String, texture::Image>,
    converted: &mut BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<BTreeMap<String, Vec<Surface>>> {
    let needs_geometry = native.values().any(|material| {
        material["technique_set"]
            .as_str()
            .is_some_and(|family| supported(family) && !special_family(family))
    });
    let mut supplied = Vec::new();
    if needs_geometry {
        for (_, zone, root) in catalog.definitions(17) {
            supplied.extend(geometry::world(catalog, zone, root)?);
        }
    }
    surfaces(native, supplied, images, converted, pack)
}

pub fn surfaces(
    native: &BTreeMap<String, Value>,
    supplied: Vec<Surface>,
    images: &BTreeMap<String, texture::Image>,
    converted: &mut BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<BTreeMap<String, Vec<Surface>>> {
    let mut required: BTreeMap<_, _> = native
        .iter()
        .filter(|(_, material)| material["technique_set"].as_str().is_some_and(supported))
        .collect();
    let special: Vec<String> = required
        .iter()
        .filter(|(_, material)| {
            material["technique_set"]
                .as_str()
                .is_some_and(special_family)
        })
        .map(|(name, _)| (*name).clone())
        .collect();
    let mut shared_sources = BTreeMap::new();
    for key in special {
        let material = required.remove(&key).unwrap();
        let mut bounds = [[f32::INFINITY; 2], [f32::NEG_INFINITY; 2]];
        let nonperiodic = material["technique_set"] == "mc_sw4_3d_water_on_surface_reveal_77320z68"
            || material["technique_set"]
                .as_str()
                .unwrap()
                .contains("phong_emissive_fritz");
        if nonperiodic && material["code_constants"].is_object() {
            for surface in supplied
                .iter()
                .filter(|surface| surface.material.eq_ignore_ascii_case(&key))
            {
                for uv in &surface.uv {
                    for axis in 0..2 {
                        bounds[0][axis] = bounds[0][axis].min(uv[axis]);
                        bounds[1][axis] = bounds[1][axis].max(uv[axis]);
                    }
                }
            }
        }
        let bounds = bounds
            .iter()
            .flatten()
            .all(|value| value.is_finite())
            .then_some(bounds);
        let uri = format!("prepared/materials/{}/surface", converted.len());
        converted.insert(
            key.clone(),
            prepare_special(material, images, &uri, bounds, pack, &mut shared_sources)?,
        );
    }
    if required.is_empty() {
        return Ok(BTreeMap::new());
    }
    for material in required.values() {
        validate_native(material)?;
    }
    let mut groups = BTreeMap::<String, Vec<Surface>>::new();
    for surface in supplied {
        let key = surface.material.to_ascii_lowercase();
        if required.contains_key(&key) {
            groups.entry(key).or_default().push(surface.unindex()?);
        }
    }
    let mut baked = BTreeMap::<String, Vec<Surface>>::new();
    for (key, material) in required {
        let surfaces = groups
            .remove(key)
            .ok_or_else(|| format!("Layered material {key} has no supplied source geometry"))?;
        let source_counts: BTreeMap<_, _> = surfaces
            .iter()
            .map(|surface| (surface.name.clone(), surface.indices.len()))
            .collect();
        let input_triangles: usize = surfaces
            .iter()
            .map(|surface| surface.indices.len() / 3)
            .sum();
        let material_recipe = recipe(material, Some(images))?;
        let baker = gpu::Baker::new(&material_recipe)?;
        let mut periodic_cache = BTreeMap::<Vec<u32>, String>::new();
        let mut remaining = Vec::new();
        let mut periodic_triangles = 0;
        for surface in surfaces {
            if (!material_recipe.reveal
                && !material_recipe.base_alpha_response
                && !material_recipe.base_specular)
                || material_recipe.decal
                || material_recipe
                    .inputs
                    .iter()
                    .any(|image| image.sampler & 96 != 0)
            {
                remaining.push(surface);
                continue;
            }
            let mut triangles = BTreeMap::<Vec<u32>, Vec<usize>>::new();
            let mut variable = Vec::new();
            for triangle in 0..surface.indices.len() / 3 {
                if let Some(recipe) =
                    periodic_key(&surface, triangle * 3, material_recipe.color_only_layer())?
                {
                    triangles.entry(recipe).or_default().push(triangle);
                } else {
                    variable.push(triangle);
                }
            }
            for (recipe, triangles) in triangles {
                let identity = if let Some(identity) = periodic_cache.get(&recipe) {
                    identity.clone()
                } else {
                    let identity = format!("{key}#periodic{}", periodic_cache.len());
                    let uri = format!("prepared/materials/{}/periodic", converted.len());
                    let mut value =
                        periodic(material, &material_recipe, &baker, &recipe, &uri, pack)?;
                    value["name"] = json!(identity);
                    value["extras"]["surface"]["identity"] = json!(identity);
                    converted.insert(identity.clone(), value);
                    periodic_cache.insert(recipe, identity.clone());
                    identity
                };
                let mut part = surface.select_triangles(&triangles)?;
                part.material = identity;
                part.colors.fill([255; 4]);
                periodic_triangles += triangles.len();
                baked.entry(surface.name.clone()).or_default().push(part);
            }
            if !variable.is_empty() {
                remaining.push(surface.select_triangles(&variable)?);
            }
        }
        let remaining_triangles: usize = remaining
            .iter()
            .map(|surface| surface.indices.len() / 3)
            .sum();
        if periodic_triangles + remaining_triangles != input_triangles {
            return Err("Layered partition changed the source triangle count".into());
        }
        println!(
            "Layered source partition {key}: {periodic_triangles} periodic triangles in {} shared recipes, {remaining_triangles} variable triangles",
            periodic_cache.len()
        );
        let uri = format!("prepared/materials/{}/layered", converted.len());
        if !remaining.is_empty() {
            for surface in layered(
                material,
                &remaining,
                &material_recipe,
                &baker,
                &uri,
                converted,
                pack,
            )? {
                baked.entry(surface.name.clone()).or_default().push(surface);
            }
        } else {
            let value = converted[periodic_cache
                .values()
                .next()
                .ok_or("Layered material has no geometry")?]
            .clone();
            converted.insert(key.clone(), value);
        }
        for (name, count) in source_counts {
            if baked
                .get(&name)
                .ok_or("Baked source surface is missing")?
                .iter()
                .map(|surface| surface.indices.len())
                .sum::<usize>()
                != count
            {
                return Err(
                    format!("Baked parts change native surface {name} triangle count").into(),
                );
            }
        }
    }
    Ok(baked)
}

fn recipe(native: &Value, images: Option<&BTreeMap<String, texture::Image>>) -> Result<Recipe> {
    let name = native["technique_set"]
        .as_str()
        .ok_or("Baked material family is missing")?;
    family(name)
        .ok_or_else(|| format!("Unreviewed baked material family: {name}"))?
        .read(native, images)
}
