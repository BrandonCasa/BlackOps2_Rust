//! Explicit unlit color preview; no original lighting or full material equivalence claim.
use bevy::{
    asset::{Asset, RenderAssetUsages},
    image::{
        CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageSampler,
        ImageSamplerDescriptor, ImageType,
    },
    mesh::{Indices, Mesh, PrimitiveTopology},
    reflect::TypePath,
};
use bo2_assets::appearance::{AppearanceError, MaterialSceneInput};

#[derive(Asset, TypePath)]
pub struct MaterialScene {
    /// Canonical Bevy +Y-up positions, with original corner UVs.
    pub mesh: Mesh,
    pub image: Image,
    pub source_surface: usize,
    /// Unchanged source-space metadata; bounds are not Bevy runtime bounds.
    pub surface_record: bo2_assets::geometry_snapshot::SnapshotSurface,
    pub source_indices: Vec<u32>,
}
pub fn source_sampler(bits: u8) -> Result<ImageSamplerDescriptor, AppearanceError> {
    let f = bits & 7;
    if !(1..=4).contains(&f) || (bits >> 3) & 3 != 2 {
        return Err(AppearanceError::Unsupported);
    }
    let mode = |bit| {
        if bits & bit == 0 {
            ImageAddressMode::Repeat
        } else {
            ImageAddressMode::ClampToEdge
        }
    };
    let filter = if f == 1 {
        ImageFilterMode::Nearest
    } else {
        ImageFilterMode::Linear
    };
    Ok(ImageSamplerDescriptor {
        address_mode_u: mode(32),
        address_mode_v: mode(64),
        address_mode_w: mode(128),
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: ImageFilterMode::Linear,
        anisotropy_clamp: match f {
            3 => 2,
            4 => 4,
            _ => 1,
        },
        ..Default::default()
    })
}
pub fn material_scene(
    input: MaterialSceneInput,
    formats: CompressedImageFormats,
) -> Result<MaterialScene, AppearanceError> {
    bo2_assets::appearance::validate_dds(&input.texture_dds)?;
    let sampler = source_sampler(input.sampler)?;
    let surface = input
        .geometry
        .surfaces()
        .get(input.surface)
        .ok_or(AppearanceError::Reference)?;
    let range = surface.index_range();
    if range.len() > 1_000_000 || input.corner_uv.iter().flatten().any(|v| !v.is_finite()) {
        return Err(AppearanceError::Attributes);
    }
    let mut source_indices = Vec::new();
    source_indices
        .try_reserve_exact(range.len())
        .map_err(|_| AppearanceError::Limit)?;
    source_indices.extend_from_slice(&input.geometry.indices()[range]);
    if input.corner_uv.len() != source_indices.len() {
        return Err(AppearanceError::Attributes);
    }
    let mut positions = Vec::new();
    positions
        .try_reserve_exact(source_indices.len())
        .map_err(|_| AppearanceError::Limit)?;
    positions.extend(
        source_indices
            .iter()
            .map(|&i| crate::coordinates::source_to_bevy(input.geometry.positions()[i as usize])),
    );
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, input.corner_uv);
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(source_indices.len())
        .map_err(|_| AppearanceError::Limit)?;
    indices.extend(0..source_indices.len() as u32);
    mesh.insert_indices(Indices::U32(indices));
    let image = Image::from_buffer(
        &input.texture_dds,
        ImageType::Extension("dds"),
        formats,
        false,
        ImageSampler::Descriptor(sampler),
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .map_err(|_| AppearanceError::Unsupported)?;
    let (_, _, surfaces) = input.geometry.into_parts();
    let surface_record = surfaces
        .into_iter()
        .nth(input.surface)
        .ok_or(AppearanceError::Reference)?;
    Ok(MaterialScene {
        mesh,
        image,
        source_surface: input.surface,
        surface_record,
        source_indices,
    })
}
