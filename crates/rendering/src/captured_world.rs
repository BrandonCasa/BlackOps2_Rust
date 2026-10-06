//! Ordered, frozen captured-world dispatch through Bevy's stock material draws.
use crate::{original_material::*, vertex_routes::*};
use bevy::{
    core_pipeline::core_3d::Transparent3d,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    image::{ImageAddressMode, ImageCompareFunction, ImageFilterMode, ImageSamplerDescriptor},
    mesh::{Indices, VertexAttributeValues},
    pbr::DrawMaterial,
    prelude::*,
    render::{
        Extract, Render, RenderApp, RenderSystems,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult,
            TrackedRenderPass, ViewSortedRenderPhases, sort_phase_system,
        },
        render_resource::*,
    },
    shader::Shader,
};
use bo2_assets::{
    captured_frame::*,
    full_scene::FullSceneInput,
    secondary_stream::SourceSecondaryCorner,
    shader_bank::{ShaderBankInput, ShaderPass, ShaderStage},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapturedWorldError {
    UnsupportedState,
    MissingShader,
    MissingImage,
    MissingDraw,
    ResourceConflict,
    InvalidSlice,
    Material(OriginalMaterialError),
    Route(VertexRouteError),
}
impl From<OriginalMaterialError> for CapturedWorldError {
    fn from(e: OriginalMaterialError) -> Self {
        Self::Material(e)
    }
}
impl From<VertexRouteError> for CapturedWorldError {
    fn from(e: VertexRouteError) -> Self {
        Self::Route(e)
    }
}

/// Every replay entity must also disable automatic batching and frustum culling.
#[derive(Component, Clone, Debug)]
#[require(
    bevy::render::batching::NoAutomaticBatching,
    bevy::camera::visibility::NoFrustumCulling
)]
pub struct CapturedDraw {
    pub ordinal: u32,
    pub event: u32,
    pub viewport: [f32; 6],
    pub scissor: [u32; 4],
    pub blend_constant: [f32; 4],
}
pub struct PreparedCapturedDraw {
    pub mesh: Mesh,
    pub material: OriginalMaterial,
    pub dispatch: CapturedDraw,
}
pub struct CapturedHandles<'a> {
    /// Shader-bank index order; external shader sources are loaded once by caller.
    pub shaders: &'a [Handle<Shader>],
    /// Content SHA-256 -> exact typed/view image prepared by caller, including depth uploads.
    pub images: &'a BTreeMap<(String, String), Handle<Image>>,
}

pub fn native_compare(value: u32) -> Result<CompareFunction, CapturedWorldError> {
    Ok(match value {
        0 => CompareFunction::Never,
        1 => CompareFunction::Always,
        2 => CompareFunction::Less,
        3 => CompareFunction::LessEqual,
        4 => CompareFunction::Greater,
        5 => CompareFunction::GreaterEqual,
        6 => CompareFunction::Equal,
        7 => CompareFunction::NotEqual,
        _ => return Err(CapturedWorldError::UnsupportedState),
    })
}
fn factor(value: u32) -> Result<BlendFactor, CapturedWorldError> {
    Ok(match value {
        0 => BlendFactor::Zero,
        1 => BlendFactor::One,
        2 => BlendFactor::Src,
        6 => BlendFactor::SrcAlpha,
        7 => BlendFactor::OneMinusSrcAlpha,
        9 => BlendFactor::OneMinusDstAlpha,
        _ => return Err(CapturedWorldError::UnsupportedState),
    })
}
fn blend(values: [u32; 3]) -> Result<BlendComponent, CapturedWorldError> {
    if values[2] != 0 {
        return Err(CapturedWorldError::UnsupportedState);
    }
    Ok(BlendComponent {
        src_factor: factor(values[0])?,
        dst_factor: factor(values[1])?,
        operation: BlendOperation::Add,
    })
}
/// RenderDoc replay enums, not Direct3D enum numbers. Unsupported states fail.
pub fn native_raster(
    p: &PipelineState,
) -> Result<(OriginalRasterState, OriginalReplayState), CapturedWorldError> {
    if !p.depth_clip
        || p.stencil_enabled
        || p.alpha_to_coverage
        || p.conservative_raster != 0
        || p.forced_samples != 0
        || p.antialiased_lines
        || p.sample_mask != u32::MAX
        || p.color.view_format != "R8G8B8A8_UNORM"
        || p.color.samples != 4
        || p.depth.as_ref().is_none_or(|d| {
            d.samples != 4
                || d.width != p.color.width
                || d.height != p.color.height
                || !matches!(
                    d.view_format.as_str(),
                    "D32S8" | "D32_FLOAT_S8X24_UINT" | "D32_FLOAT"
                )
        })
        || p.write_mask & !15 != 0
        || !p.slope_bias.is_finite()
        || !p.bias_clamp.is_finite()
    {
        return Err(CapturedWorldError::UnsupportedState);
    }
    Ok((
        OriginalRasterState {
            front_face: if p.front_ccw {
                FrontFace::Ccw
            } else {
                FrontFace::Cw
            },
            cull_mode: match p.cull {
                0 => None,
                1 => Some(Face::Front),
                2 => Some(Face::Back),
                _ => return Err(CapturedWorldError::UnsupportedState),
            },
            depth_write: p.depth_enabled && p.depth_write,
            depth_compare: if p.depth_enabled {
                native_compare(p.depth_compare)?
            } else {
                CompareFunction::Always
            },
            blend: if p.blend_enabled {
                Some(BlendState {
                    color: blend(p.blend_color)?,
                    alpha: blend(p.blend_alpha)?,
                })
            } else {
                None
            },
            color_writes: ColorWrites::from_bits(u32::from(p.write_mask))
                .ok_or(CapturedWorldError::UnsupportedState)?,
        },
        OriginalReplayState {
            depth_bias: p.depth_bias,
            slope_bias_bits: p.slope_bias.to_bits(),
            clamp_bits: p.bias_clamp.to_bits(),
        },
    ))
}
pub fn native_dispatch(
    ordinal: u32,
    event: u32,
    p: &PipelineState,
) -> Result<CapturedDraw, CapturedWorldError> {
    let [x, y, w, h, min, max] = p.viewport;
    if !p.viewport.iter().all(|v| v.is_finite())
        || x < 0.0
        || y < 0.0
        || w <= 0.0
        || h <= 0.0
        || x + w > p.color.width as f32
        || y + h > p.color.height as f32
        || min < 0.0
        || max > 1.0
        || min > max
        || p.blend_constant.iter().any(|v| !v.is_finite())
    {
        return Err(CapturedWorldError::UnsupportedState);
    }
    let [x, y, width, height] = if p.scissor_enabled {
        p.scissor.map(i64::from)
    } else {
        [0, 0, i64::from(p.color.width), i64::from(p.color.height)]
    };
    if width <= 0 || height <= 0 {
        return Err(CapturedWorldError::UnsupportedState);
    }
    let left = x.clamp(0, i64::from(p.color.width)) as u32;
    let top = y.clamp(0, i64::from(p.color.height)) as u32;
    let right = (x + width).clamp(0, i64::from(p.color.width)) as u32;
    let bottom = (y + height).clamp(0, i64::from(p.color.height)) as u32;
    if right <= left || bottom <= top {
        return Err(CapturedWorldError::UnsupportedState);
    }
    Ok(CapturedDraw {
        ordinal,
        event,
        viewport: p.viewport,
        scissor: [left, top, right - left, bottom - top],
        blend_constant: p.blend_constant,
    })
}
pub fn native_sampler(s: &SamplerBinding) -> Result<ImageSamplerDescriptor, CapturedWorldError> {
    fn address(a: u32) -> Result<ImageAddressMode, CapturedWorldError> {
        Ok(match a {
            0 => ImageAddressMode::Repeat,
            1 => ImageAddressMode::MirrorRepeat,
            3 => ImageAddressMode::ClampToEdge,
            _ => return Err(CapturedWorldError::UnsupportedState),
        })
    }
    fn filter(f: u32) -> Result<ImageFilterMode, CapturedWorldError> {
        Ok(match f {
            1 => ImageFilterMode::Nearest,
            2 | 4 => ImageFilterMode::Linear,
            _ => return Err(CapturedWorldError::UnsupportedState),
        })
    }
    if s.mip_bias != 0.0
        || s.border != [0.0; 4]
        || !s.min_lod.is_finite()
        || !s.max_lod.is_finite()
        || s.filter[3] > 1
        || !s.max_anisotropy.is_finite()
    {
        return Err(CapturedWorldError::UnsupportedState);
    }
    let compare = if s.filter[3] == 1 {
        Some(match native_compare(s.comparison)? {
            CompareFunction::Greater => ImageCompareFunction::Greater,
            _ => return Err(CapturedWorldError::UnsupportedState),
        })
    } else {
        None
    };
    let anisotropy = if s.filter[..3].contains(&4) {
        if s.filter[..3] != [4, 4, 4]
            || s.max_anisotropy < 1.0
            || s.max_anisotropy > 16.0
            || s.max_anisotropy.fract() != 0.0
        {
            return Err(CapturedWorldError::UnsupportedState);
        }
        s.max_anisotropy as u16
    } else {
        1
    };
    Ok(ImageSamplerDescriptor {
        address_mode_u: address(s.address[0])?,
        address_mode_v: address(s.address[1])?,
        address_mode_w: address(s.address[2])?,
        min_filter: filter(s.filter[0])?,
        mag_filter: filter(s.filter[1])?,
        mipmap_filter: filter(s.filter[2])?,
        lod_min_clamp: s.min_lod.max(0.0),
        lod_max_clamp: s.max_lod.max(0.0),
        compare,
        anisotropy_clamp: anisotropy,
        ..Default::default()
    })
}
fn stage_binding(stage: ShaderStage, register: u8) -> u32 {
    u32::from(register)
        + if stage == ShaderStage::Fragment {
            16
        } else {
            0
        }
}

/// Batched slices stay one native draw; source shaders retain source object matrices.
pub fn captured_mesh(
    scene: &FullSceneInput,
    secondary: &[SourceSecondaryCorner],
    bank: &ShaderBankInput,
    pass: &ShaderPass,
    draw: &WorldDraw,
) -> Result<Mesh, CapturedWorldError> {
    let mut combined: Option<Mesh> = None;
    for slice in &draw.slices {
        let range = scene
            .geometry
            .surfaces()
            .get(slice.surface)
            .ok_or(CapturedWorldError::InvalidSlice)?
            .index_range();
        let end = slice
            .first_corner
            .checked_add(slice.corners)
            .ok_or(CapturedWorldError::InvalidSlice)?;
        if slice.first_corner < range.start
            || end > range.end
            || slice.corners == 0
            || !slice.corners.is_multiple_of(3)
        {
            return Err(CapturedWorldError::InvalidSlice);
        }
        let format = scene
            .surfaces
            .get(slice.surface)
            .ok_or(CapturedWorldError::InvalidSlice)?
            .source_vertex_format;
        let route = world_vertex_route(bank, pass, format)?;
        let full = routed_world_mesh(scene, slice.surface, Some(secondary), &route)?;
        let start = slice.first_corner - range.start;
        let stop = start + slice.corners;
        let mut mesh = Mesh::new(
            bevy::mesh::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::default(),
        );
        for (a, values) in full.attributes() {
            let values = match values {
                VertexAttributeValues::Float32x2(v) => {
                    VertexAttributeValues::Float32x2(v[start..stop].to_vec())
                }
                VertexAttributeValues::Float32x3(v) => {
                    VertexAttributeValues::Float32x3(v[start..stop].to_vec())
                }
                VertexAttributeValues::Float32x4(v) => {
                    VertexAttributeValues::Float32x4(v[start..stop].to_vec())
                }
                _ => return Err(CapturedWorldError::InvalidSlice),
            };
            mesh.insert_attribute(*a, values);
        }
        mesh.insert_indices(Indices::U32((0..slice.corners as u32).collect()));
        if let Some(combined) = &mut combined {
            combined
                .merge(&mesh)
                .map_err(|_| CapturedWorldError::InvalidSlice)?;
        } else {
            combined = Some(mesh);
        }
    }
    combined.ok_or(CapturedWorldError::InvalidSlice)
}

pub fn prepare_captured_world(
    frame: &CapturedFrameInput,
    scene: &FullSceneInput,
    secondary: &[SourceSecondaryCorner],
    bank: &ShaderBankInput,
    handles: CapturedHandles<'_>,
    limits: &WgpuLimits,
) -> Result<Vec<PreparedCapturedDraw>, CapturedWorldError> {
    let mut result = Vec::new();
    for receipt in &frame.manifest.draws {
        let Dispatch::World { draw } = &receipt.dispatch else {
            continue;
        };
        let loaded = frame
            .draws
            .iter()
            .find(|d| d.event == receipt.event)
            .ok_or(CapturedWorldError::MissingDraw)?;
        let vi = bank
            .shaders
            .iter()
            .position(|s| {
                s.declaration.source_sha256 == draw.vertex_shader_sha256
                    && s.declaration.stage == ShaderStage::Vertex
            })
            .ok_or(CapturedWorldError::MissingShader)?;
        let fi = bank
            .shaders
            .iter()
            .position(|s| {
                s.declaration.source_sha256 == draw.fragment_shader_sha256
                    && s.declaration.stage == ShaderStage::Fragment
            })
            .ok_or(CapturedWorldError::MissingShader)?;
        let surface = draw
            .slices
            .first()
            .ok_or(CapturedWorldError::InvalidSlice)?
            .surface;
        let format = scene
            .surfaces
            .get(surface)
            .ok_or(CapturedWorldError::InvalidSlice)?
            .source_vertex_format;
        let material = bank
            .materials
            .iter()
            .find(|m| m.scene_material == scene.surfaces[surface].material)
            .ok_or(CapturedWorldError::MissingShader)?;
        let family = bank
            .families
            .get(material.family)
            .ok_or(CapturedWorldError::MissingShader)?;
        let mut candidates = family
            .techniques
            .iter()
            .flat_map(|t| &t.passes)
            .filter(|p| p.vertex_shader == vi && p.pixel_shader == fi);
        let pass = candidates.next().ok_or(CapturedWorldError::MissingShader)?;
        let attributes = world_vertex_route(bank, pass, format)?;
        for other in candidates {
            if world_vertex_route(bank, other, format)? != attributes {
                return Err(CapturedWorldError::ResourceConflict);
            }
        }
        let (raster, replay) = native_raster(&draw.pipeline)?;
        let mut descriptor = translated_descriptor(
            &bank.shaders[vi].declaration,
            &bank.shaders[fi].declaration,
            handles
                .shaders
                .get(vi)
                .ok_or(CapturedWorldError::MissingShader)?
                .clone(),
            handles
                .shaders
                .get(fi)
                .ok_or(CapturedWorldError::MissingShader)?
                .clone(),
            attributes,
            raster,
        )?;
        descriptor.replay = Some(replay);
        let needed = |binding, stage| {
            let visibility = match stage {
                ShaderStage::Vertex => ShaderStages::VERTEX,
                ShaderStage::Fragment => ShaderStages::FRAGMENT,
            };
            descriptor
                .bindings
                .iter()
                .any(|s| s.binding == binding && s.visibility.contains(visibility))
        };
        if loaded.uniforms.len() != draw.uniforms.len()
            || loaded.textures.len() != draw.textures.len()
        {
            return Err(CapturedWorldError::MissingDraw);
        }
        let mut resources = BTreeMap::new();
        for (binding, bytes) in draw.uniforms.iter().zip(&loaded.uniforms) {
            let b = stage_binding(binding.stage, binding.register);
            if !needed(b, binding.stage) {
                continue;
            }
            if resources
                .insert(
                    b,
                    OriginalResource::Uniform {
                        binding: b,
                        bytes: bytes.to_vec(),
                    },
                )
                .is_some()
            {
                return Err(CapturedWorldError::ResourceConflict);
            }
        }
        for binding in &draw.textures {
            let b = 32 + u32::from(binding.register);
            if !needed(b, binding.stage) {
                continue;
            }
            let image = handles
                .images
                .get(&(binding.bytes.sha256.clone(), binding.view_format.clone()))
                .ok_or(CapturedWorldError::MissingImage)?
                .clone();
            if let Some(OriginalResource::Image { image: prior, .. }) = resources.get(&b)
                && *prior != image
            {
                return Err(CapturedWorldError::ResourceConflict);
            }
            resources.insert(b, OriginalResource::Image { binding: b, image });
        }
        for binding in &draw.samplers {
            let b = 64 + u32::from(binding.register);
            if !needed(b, binding.stage) {
                continue;
            }
            let sampler = native_sampler(binding)?;
            if let Some(OriginalResource::Sampler {
                descriptor: prior, ..
            }) = resources.get(&b)
                && *prior != sampler
            {
                return Err(CapturedWorldError::ResourceConflict);
            }
            resources.insert(
                b,
                OriginalResource::Sampler {
                    binding: b,
                    descriptor: sampler,
                },
            );
        }
        let resources = resources
            .into_iter()
            .filter(|(b, _)| descriptor.bindings.iter().any(|s| s.binding == *b))
            .map(|(_, r)| r)
            .collect();
        result.push(PreparedCapturedDraw {
            mesh: captured_mesh(scene, secondary, bank, pass, draw)?,
            material: OriginalMaterial::new(descriptor, resources, limits)?,
            dispatch: native_dispatch(result.len() as u32, receipt.event, &draw.pipeline)?,
        });
    }
    Ok(result)
}

#[derive(Resource, Default)]
struct ExtractedDraws(BTreeMap<Entity, CapturedDraw>);
fn extract_draws(mut commands: Commands, draws: Extract<Query<(Entity, &CapturedDraw)>>) {
    commands.insert_resource(ExtractedDraws(
        draws.iter().map(|(e, d)| (e, d.clone())).collect(),
    ));
}
struct SetCapturedDraw;
impl RenderCommand<Transparent3d> for SetCapturedDraw {
    type Param = SRes<ExtractedDraws>;
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &Transparent3d,
        _view: (),
        _query: Option<()>,
        draws: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(d) = draws.into_inner().0.get(&item.main_entity().id()) else {
            return RenderCommandResult::Skip;
        };
        let [x, y, w, h, min, max] = d.viewport;
        pass.set_viewport(x, y, w, h, min, max);
        let [x, y, w, h] = d.scissor;
        pass.set_scissor_rect(x, y, w, h);
        pass.set_blend_constant(LinearRgba::from_f32_array(d.blend_constant));
        RenderCommandResult::Success
    }
}
fn order_draws(
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    draws: Res<ExtractedDraws>,
    functions: Res<DrawFunctions<Transparent3d>>,
) {
    let id = functions.read().id::<(SetCapturedDraw, DrawMaterial)>();
    for phase in phases.0.values_mut() {
        for item in phase.items.values_mut() {
            if draws.0.contains_key(&item.main_entity().id()) {
                item.draw_function = id;
            }
        }
        phase.items.sort_by_key(|_, i| {
            draws
                .0
                .get(&i.main_entity().id())
                .map_or(u32::MAX, |d| d.ordinal)
        });
    }
}
/// Add after OriginalMaterialPlugin. A replay camera must disable indirect drawing
/// and every replay mesh must disable automatic batching and frustum culling.
pub struct CapturedWorldPlugin;
impl Plugin for CapturedWorldPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<ExtractedDraws>()
                .add_systems(bevy::render::ExtractSchedule, extract_draws)
                .add_render_command::<Transparent3d, (SetCapturedDraw, DrawMaterial)>()
                .add_systems(
                    Render,
                    order_draws
                        .in_set(RenderSystems::PhaseSort)
                        .after(sort_phase_system::<Transparent3d>),
                );
        }
    }
}
