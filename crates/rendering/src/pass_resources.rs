//! Exact pass argument assembly. Engine-owned resources are supplied explicitly.
use crate::original_material::{BindingKind, OriginalMaterialDescriptor, OriginalResource};
use bevy::{image::ImageSamplerDescriptor, prelude::*, render::render_resource::ShaderStages};
use bo2_assets::shader_bank::{
    ArgumentDestination, ArgumentSource, MaterialContract, MaterialImage, ShaderPass, ShaderStage,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PassResourceError {
    UnsupportedPassFlags {
        custom_sampler_flags: u8,
        material_type: u8,
    },
    MissingCodeConstant(u16),
    MissingCodeTexture(u32),
    MissingMaterialConstant(u32),
    MissingMaterialTexture(u32),
    MissingSceneImage(usize),
    MissingBuiltin(String),
    MissingBinding(u32),
    UninitializedUniform {
        binding: u32,
        offset: usize,
    },
    ConflictingBinding(u32),
    ConflictingUniform {
        binding: u32,
        offset: usize,
    },
    InvalidArgument,
    InvalidSampler(u32),
}

#[derive(Clone)]
pub struct RuntimeTexture {
    pub image: Handle<Image>,
    pub sampler: ImageSamplerDescriptor,
}
/// This record is admitted only after the caller qualifies the pass's implicit
/// lighting/probe assignments. Its resources are complete explicit bindings.
pub struct QualifiedImplicitBindings {
    pub custom_sampler_flags: u8,
    pub material_type: u8,
    pub resources: Vec<OriginalResource>,
}
impl QualifiedImplicitBindings {
    /// Qualified native world slots: bit one is reflection t15/s15; bit two
    /// secondary lightmap t13/s13. Material type is caller-qualified metadata.
    pub fn world_lighting(
        pass: &ShaderPass,
        secondary_lightmap: Option<RuntimeTexture>,
        reflection_probe: Option<RuntimeTexture>,
    ) -> Result<Self, PassResourceError> {
        if pass.custom_sampler_flags & !3 != 0 {
            return Err(PassResourceError::UnsupportedPassFlags {
                custom_sampler_flags: pass.custom_sampler_flags,
                material_type: pass.material_type,
            });
        }
        let mut resources = Vec::new();
        for (flag, t, s, texture) in [
            (1, 47, 79, reflection_probe),
            (2, 45, 77, secondary_lightmap),
        ] {
            if pass.custom_sampler_flags & flag != 0 {
                let texture = texture.ok_or(PassResourceError::MissingBinding(t))?;
                resources.push(OriginalResource::Image {
                    binding: t,
                    image: texture.image,
                });
                resources.push(OriginalResource::Sampler {
                    binding: s,
                    descriptor: texture.sampler,
                });
            }
        }
        Ok(Self {
            custom_sampler_flags: pass.custom_sampler_flags,
            material_type: pass.material_type,
            resources,
        })
    }
}
#[derive(Default)]
pub struct PassResourceContext {
    /// Source code constant arrays in native float4 row order (not transposed).
    pub code_constants: BTreeMap<u16, Vec<[f32; 4]>>,
    pub code_textures: BTreeMap<u32, RuntimeTexture>,
    pub scene_images: BTreeMap<usize, Handle<Image>>,
    pub builtin_images: BTreeMap<String, Handle<Image>>,
    /// Explicit native buffers initialize bytes not covered by pass arguments.
    /// Their values never become engine defaults.
    pub initial_uniforms: BTreeMap<u32, Vec<u8>>,
    pub qualified_implicit: Option<QualifiedImplicitBindings>,
}
struct UniformAssembly {
    data: Vec<u8>,
    initialized: Vec<bool>,
    written: Vec<bool>,
}

fn uniform_binding(stage: ShaderStage, buffer: u16) -> Result<u32, PassResourceError> {
    if buffer >= 16 {
        return Err(PassResourceError::InvalidArgument);
    }
    Ok(u32::from(buffer)
        + if stage == ShaderStage::Fragment {
            16
        } else {
            0
        })
}
fn bytes(rows: &[[f32; 4]]) -> Result<Vec<u8>, PassResourceError> {
    if rows.iter().flatten().any(|c| !c.is_finite()) {
        return Err(PassResourceError::InvalidArgument);
    }
    Ok(rows
        .iter()
        .flatten()
        .flat_map(|c| c.to_le_bytes())
        .collect())
}
fn sampler_equal(a: &ImageSamplerDescriptor, b: &ImageSamplerDescriptor) -> bool {
    a.address_mode_u == b.address_mode_u
        && a.address_mode_v == b.address_mode_v
        && a.address_mode_w == b.address_mode_w
        && a.mag_filter == b.mag_filter
        && a.min_filter == b.min_filter
        && a.mipmap_filter == b.mipmap_filter
        && a.lod_min_clamp == b.lod_min_clamp
        && a.lod_max_clamp == b.lod_max_clamp
        && a.compare == b.compare
        && a.anisotropy_clamp == b.anisotropy_clamp
        && a.border_color == b.border_color
}
fn same(a: &OriginalResource, b: &OriginalResource) -> bool {
    match (a, b) {
        (OriginalResource::Image { image: a, .. }, OriginalResource::Image { image: b, .. }) => {
            a == b
        }
        (
            OriginalResource::Sampler { descriptor: a, .. },
            OriginalResource::Sampler { descriptor: b, .. },
        ) => sampler_equal(a, b),
        (
            OriginalResource::Uniform { bytes: a, .. },
            OriginalResource::Uniform { bytes: b, .. },
        ) => a == b,
        _ => false,
    }
}
fn insert(
    resources: &mut BTreeMap<u32, OriginalResource>,
    resource: OriginalResource,
) -> Result<(), PassResourceError> {
    let binding = resource.binding();
    if let Some(existing) = resources.get(&binding) {
        if !same(existing, &resource) {
            return Err(PassResourceError::ConflictingBinding(binding));
        }
    } else {
        resources.insert(binding, resource);
    }
    Ok(())
}

/// Assemble only reflected resources. Non-reflected pass writes are dead inputs
/// for this exact shader pair and do not require unrelated engine values.
pub fn assemble_pass_resources(
    descriptor: &OriginalMaterialDescriptor,
    pass: &ShaderPass,
    material: &MaterialContract,
    context: &PassResourceContext,
) -> Result<Vec<OriginalResource>, PassResourceError> {
    use PassResourceError as E;
    descriptor.validate().map_err(|_| E::InvalidArgument)?;
    let mut resources = BTreeMap::new();
    if pass.custom_sampler_flags != 0 || pass.material_type != 0 {
        let qualified = context
            .qualified_implicit
            .as_ref()
            .filter(|q| {
                q.custom_sampler_flags == pass.custom_sampler_flags
                    && q.material_type == pass.material_type
            })
            .ok_or(E::UnsupportedPassFlags {
                custom_sampler_flags: pass.custom_sampler_flags,
                material_type: pass.material_type,
            })?;
        for resource in &qualified.resources {
            insert(&mut resources, resource.clone())?;
        }
    }
    // Initialization validity and pass-write collisions are tracked separately:
    // explicit seed bytes may legitimately be overwritten by current arguments.
    let mut uniforms: BTreeMap<u32, UniformAssembly> = BTreeMap::new();
    for spec in &descriptor.bindings {
        if let BindingKind::Uniform { minimum_size } = spec.kind {
            let size = usize::try_from(minimum_size)
                .map_err(|_| E::InvalidArgument)?
                .next_multiple_of(16);
            let seed = context.initial_uniforms.get(&spec.binding);
            if seed
                .is_some_and(|b| b.len() < size || b.len() > 65536 || !b.len().is_multiple_of(16))
            {
                return Err(E::InvalidArgument);
            }
            let data = seed.cloned().unwrap_or_else(|| vec![0; size]);
            let initialized = vec![seed.is_some(); data.len()];
            let written = vec![false; data.len()];
            uniforms.insert(
                spec.binding,
                UniformAssembly {
                    data,
                    initialized,
                    written,
                },
            );
        }
    }
    for argument in &pass.args {
        match argument.destination {
            ArgumentDestination::Uniform {
                buffer,
                offset,
                size,
            } => {
                let binding = uniform_binding(argument.stage, buffer)?;
                let Some(UniformAssembly {
                    data,
                    initialized,
                    written,
                }) = uniforms.get_mut(&binding)
                else {
                    continue;
                };
                let stage = if argument.stage == ShaderStage::Vertex {
                    ShaderStages::VERTEX
                } else {
                    ShaderStages::FRAGMENT
                };
                if !descriptor
                    .bindings
                    .iter()
                    .any(|s| s.binding == binding && s.visibility.contains(stage))
                {
                    return Err(E::InvalidArgument);
                }
                if size == 0 || size % 4 != 0 || offset % 4 != 0 {
                    return Err(E::InvalidArgument);
                }
                let source = match &argument.source {
                    ArgumentSource::MaterialConstant { name_hash } => bytes(&[material
                        .constants
                        .iter()
                        .find(|c| c.name_hash == *name_hash)
                        .ok_or(E::MissingMaterialConstant(*name_hash))?
                        .value])?,
                    ArgumentSource::Literal { value } => bytes(&[*value])?,
                    ArgumentSource::CodeConstant {
                        index,
                        first_row,
                        row_count,
                    } => {
                        if *row_count == 0 {
                            return Err(E::InvalidArgument);
                        }
                        let rows = context
                            .code_constants
                            .get(index)
                            .ok_or(E::MissingCodeConstant(*index))?;
                        let start = usize::from(*first_row);
                        let end = start
                            .checked_add(usize::from(*row_count))
                            .ok_or(E::InvalidArgument)?;
                        bytes(rows.get(start..end).ok_or(E::InvalidArgument)?)?
                    }
                    _ => return Err(E::InvalidArgument),
                };
                let start = usize::from(offset);
                let end = start
                    .checked_add(usize::from(size))
                    .ok_or(E::InvalidArgument)?;
                if end > data.len() || source.len() < usize::from(size) {
                    return Err(E::InvalidArgument);
                }
                for (index, value) in (start..end).zip(source) {
                    if written[index] && data[index] != value {
                        return Err(E::ConflictingUniform {
                            binding,
                            offset: index,
                        });
                    }
                    data[index] = value;
                    initialized[index] = true;
                    written[index] = true;
                }
            }
            ArgumentDestination::TextureSampler { texture, sampler } => {
                if argument.stage != ShaderStage::Fragment || texture >= 32 || sampler >= 32 {
                    return Err(E::InvalidArgument);
                }
                let tb = 32 + u32::from(texture);
                let sb = 64 + u32::from(sampler);
                let texture_used = descriptor.bindings.iter().any(|s| s.binding == tb);
                let sampler_used = descriptor.bindings.iter().any(|s| s.binding == sb);
                if !texture_used && !sampler_used {
                    continue;
                }
                let runtime = match &argument.source {
                    ArgumentSource::CodeTexture { index } => context
                        .code_textures
                        .get(index)
                        .cloned()
                        .ok_or(E::MissingCodeTexture(*index))?,
                    ArgumentSource::MaterialTexture { name_hash } => {
                        let texture = material
                            .textures
                            .iter()
                            .find(|t| t.name_hash == *name_hash)
                            .ok_or(E::MissingMaterialTexture(*name_hash))?;
                        let image = match &texture.image {
                            MaterialImage::SceneImage { index } => context
                                .scene_images
                                .get(index)
                                .cloned()
                                .ok_or(E::MissingSceneImage(*index))?,
                            MaterialImage::UnresolvedBuiltin { source_sha256 } => context
                                .builtin_images
                                .get(source_sha256)
                                .cloned()
                                .ok_or_else(|| E::MissingBuiltin(source_sha256.clone()))?,
                        };
                        let sampler = crate::appearance::source_sampler(texture.sampler)
                            .map_err(|_| E::InvalidSampler(*name_hash))?;
                        RuntimeTexture { image, sampler }
                    }
                    _ => return Err(E::InvalidArgument),
                };
                if texture_used {
                    insert(
                        &mut resources,
                        OriginalResource::Image {
                            binding: tb,
                            image: runtime.image,
                        },
                    )?;
                }
                if sampler_used {
                    insert(
                        &mut resources,
                        OriginalResource::Sampler {
                            binding: sb,
                            descriptor: runtime.sampler,
                        },
                    )?;
                }
            }
        }
    }
    for (
        binding,
        UniformAssembly {
            data, initialized, ..
        },
    ) in uniforms
    {
        if let Some(offset) = initialized.iter().position(|v| !v) {
            return Err(E::UninitializedUniform { binding, offset });
        }
        insert(
            &mut resources,
            OriginalResource::Uniform {
                binding,
                bytes: data,
            },
        )?;
    }
    for spec in &descriptor.bindings {
        let resource = resources
            .get(&spec.binding)
            .ok_or(E::MissingBinding(spec.binding))?;
        if !matches!(
            (&spec.kind, resource),
            (
                BindingKind::Uniform { .. },
                OriginalResource::Uniform { .. }
            ) | (BindingKind::Texture { .. }, OriginalResource::Image { .. })
                | (
                    BindingKind::Sampler { .. },
                    OriginalResource::Sampler { .. }
                )
        ) {
            return Err(E::ConflictingBinding(spec.binding));
        }
    }
    if resources
        .keys()
        .any(|binding| !descriptor.bindings.iter().any(|s| s.binding == *binding))
    {
        return Err(E::InvalidArgument);
    }
    Ok(resources.into_values().collect())
}
