//! Explicit first-color diagnostic preview, separate from original shaded admission.
use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, Image, ImageSampler, ImageType},
    mesh::{Indices, Mesh, PrimitiveTopology},
};
use bo2_assets::{appearance::AppearanceError, full_scene::FullSceneInput};
use std::collections::BTreeMap;

pub struct DiagnosticTextureMaterial {
    /// Index into prepared images; None explicitly marks missing color texture.
    pub image: Option<usize>,
    pub source_material: usize,
    pub selected_binding: Option<usize>,
    pub color_layer_count: usize,
}
pub struct DiagnosticTextureMesh {
    pub mesh: Mesh,
    pub material: usize,
    /// Ordered original surface identifiers represented by this grouped mesh.
    pub source_surfaces: Vec<usize>,
}
pub struct DiagnosticTextureScene {
    pub images: Vec<Image>,
    pub materials: Vec<DiagnosticTextureMaterial>,
    pub meshes: Vec<DiagnosticTextureMesh>,
    pub source_surface_count: usize,
    pub missing_color_surfaces: usize,
    pub multi_color_surfaces: usize,
}
struct Corners {
    positions: Vec<[f32; 3]>,
    uv: Vec<[f32; 2]>,
    surfaces: Vec<usize>,
}

/// Prepare all admitted geometry and first semantic-2 color bindings only.
/// This API explicitly ignores other color layers, normal/specular maps and all
/// original lighting/state/transfer. Missing color remains a reported marker.
/// Callers must label this mode diagnostic; it is not original scene admission.
pub fn diagnostic_texture_scene(
    input: FullSceneInput,
    formats: CompressedImageFormats,
) -> Result<DiagnosticTextureScene, AppearanceError> {
    use AppearanceError as E;
    if input.surfaces.len() != input.geometry.surfaces().len()
        || input.corner_uv.len() != input.geometry.indices().len()
        || input.corner_uv.iter().flatten().any(|f| !f.is_finite())
    {
        return Err(E::Attributes);
    }
    let mut images = Vec::new();
    let mut materials = Vec::new();
    let mut shared = BTreeMap::new();
    for (material_index, material) in input.materials.iter().enumerate() {
        if material.contract != "diagnostic-unlit-color" {
            return Err(E::Unsupported);
        }
        let colors = material
            .bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| b.semantic == 2)
            .collect::<Vec<_>>();
        let selected = colors.first().copied();
        let image = if let Some((_, binding)) = selected {
            let key = (binding.image, binding.sampler);
            if let Some(&index) = shared.get(&key) {
                Some(index)
            } else {
                let source = input.images.get(binding.image).ok_or(E::Reference)?;
                let metadata = bo2_assets::full_scene::inspect_dds(&source.dds)?;
                if metadata != source.metadata || metadata.cube {
                    return Err(E::Unsupported);
                }
                let sampler = crate::appearance::source_sampler(binding.sampler)?;
                let image = Image::from_buffer(
                    &source.dds,
                    ImageType::Extension("dds"),
                    formats,
                    false,
                    ImageSampler::Descriptor(sampler),
                    RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                )
                .map_err(|_| E::Unsupported)?;
                if image.texture_descriptor.mip_level_count != metadata.mips
                    || image.width() != metadata.width
                    || image.height() != metadata.height
                    || image.texture_descriptor.format.is_srgb()
                {
                    return Err(E::Unsupported);
                }
                let index = images.len();
                images.push(image);
                shared.insert(key, index);
                Some(index)
            }
        } else {
            None
        };
        materials.push(DiagnosticTextureMaterial {
            image,
            source_material: material_index,
            selected_binding: selected.map(|(i, _)| i),
            color_layer_count: colors.len(),
        });
    }
    let mut groups = (0..materials.len())
        .map(|_| Corners {
            positions: Vec::new(),
            uv: Vec::new(),
            surfaces: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut missing_color_surfaces = 0;
    let mut multi_color_surfaces = 0;
    for (surface_index, (surface, record)) in input
        .surfaces
        .iter()
        .zip(input.geometry.surfaces())
        .enumerate()
    {
        let descriptor = materials.get(surface.material).ok_or(E::Reference)?;
        if descriptor.image.is_none() {
            missing_color_surfaces += 1;
        }
        if descriptor.color_layer_count > 1 {
            multi_color_surfaces += 1;
        }
        let group = groups.get_mut(surface.material).ok_or(E::Reference)?;
        let range = record.index_range();
        group
            .positions
            .try_reserve(range.len())
            .map_err(|_| E::Limit)?;
        group.uv.try_reserve(range.len()).map_err(|_| E::Limit)?;
        group.surfaces.try_reserve(1).map_err(|_| E::Limit)?;
        group.surfaces.push(surface_index);
        for index in range {
            let reference = *input.geometry.indices().get(index).ok_or(E::Reference)?;
            let position = *input
                .geometry
                .positions()
                .get(reference as usize)
                .ok_or(E::Reference)?;
            group
                .positions
                .push(crate::coordinates::source_to_bevy(position));
            group.uv.push(input.corner_uv[index]);
        }
    }
    let mut meshes = Vec::new();
    for (material, group) in groups.into_iter().enumerate() {
        if group.positions.is_empty() {
            continue;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        let mut indices = Vec::new();
        indices
            .try_reserve_exact(group.positions.len())
            .map_err(|_| E::Limit)?;
        indices.extend(0..u32::try_from(group.positions.len()).map_err(|_| E::Limit)?);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, group.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, group.uv);
        mesh.insert_indices(Indices::U32(indices));
        meshes.push(DiagnosticTextureMesh {
            mesh,
            material,
            source_surfaces: group.surfaces,
        });
    }
    Ok(DiagnosticTextureScene {
        images,
        materials,
        meshes,
        source_surface_count: input.surfaces.len(),
        missing_color_surfaces,
        multi_color_surfaces,
    })
}
