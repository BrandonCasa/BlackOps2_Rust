//! Authored GfxWorld sky model with the original skycubemaphdr program.
use std::{path::Path, sync::Arc};

use super::native::{NativeWorldGeometry, NativeWorldScene, PreparedNativeGeometry};
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        camera::ExtractedCamera,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{sampler, texture_cube, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        texture::GpuImage,
        view::{ViewDepthStencilTexture, ViewTarget},
    },
    shader::load_shader_library,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct SkyDefinition {
    model: String,
}

pub(crate) struct LoadedSky {
    geometry: NativeWorldGeometry,
    image: String,
    rotation_size: Vec4,
    color_parms: Vec4,
    vertex_byte_offset: u64,
    first_index: u32,
    index_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Model {
    bone_names: Vec<String>,
    vertices_file: String,
    indices_file: String,
    bind_file: String,
    lods: Vec<Lod>,
}

#[derive(Deserialize)]
struct Lod {
    surfaces: Vec<Surface>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Surface {
    material_file: String,
    vertex_byte_offset: u64,
    first_index: u32,
    vertex_count: u32,
    tri_count: u32,
    blend_vertex_counts: [u32; 4],
}

impl SkyDefinition {
    pub(crate) fn load(&self, root: &Path) -> LoadedSky {
        let read = |path: &str| {
            std::fs::read(root.join(path))
                .unwrap_or_else(|error| panic!("Read original sky {path}: {error}"))
        };
        let model: Model =
            serde_json::from_slice(&read(&format!("model_export/{}.native_model", self.model)))
                .expect("Original sky model metadata");
        assert_eq!(model.bone_names, ["t6_skybox"], "Original sky rigid root");
        assert_eq!(model.lods.len(), 1, "Original sky LOD count");
        assert_eq!(
            model.lods[0].surfaces.len(),
            1,
            "Original sky surface count"
        );
        let surface = &model.lods[0].surfaces[0];
        assert_eq!(surface.blend_vertex_counts, [0; 4], "Original sky is rigid");
        let bind = read(&model.bind_file);
        let bind: Vec<_> = bind
            .chunks_exact(4)
            .map(|bytes| f32::from_le_bytes(bytes.try_into().unwrap()))
            .collect();
        assert_eq!(
            bind,
            [0., 0., 0., 1., 0., 0., 0., 2.],
            "Original sky bind pose is identity"
        );
        let material: serde_json::Value = serde_json::from_slice(&read(&surface.material_file))
            .expect("Original sky material metadata");
        assert_eq!(material["techniqueSet"], "mc_skycubemaphdr_1f117w7u");
        assert_eq!(material["cameraRegion"], "emissiveOpaque");
        assert_eq!(material["sortKey"], 4);
        assert_eq!(material["stateBitsEntry"][2], 0);
        assert_eq!(material["stateBitsEntry"][3], 0);
        let state = &material["stateBits"][0];
        for (field, value) in [
            ("srcBlendRgb", "one"),
            ("dstBlendRgb", "zero"),
            ("blendOpRgb", "disabled"),
            ("srcBlendAlpha", "one"),
            ("dstBlendAlpha", "zero"),
            ("blendOpAlpha", "disabled"),
            ("alphaTest", "disabled"),
            ("cullFace", "back"),
            ("depthTest", "less_equal"),
            ("polygonOffset", "offset0"),
        ] {
            assert_eq!(state[field], value, "Original sky {field}");
        }
        assert_eq!(state["depthWrite"], true);
        assert_eq!(state["colorWriteRgb"], true);
        assert_eq!(state["colorWriteAlpha"], true);
        assert_eq!(state["polymodeLine"], false);
        let constant = |name| {
            let matches: Vec<_> = material["constants"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|constant| constant["name"] == name)
                .collect();
            assert_eq!(matches.len(), 1, "Original sky constant {name}");
            let values: [f32; 4] =
                serde_json::from_value(matches[0]["literal"].clone()).expect("Original sky float4");
            assert!(values.iter().all(|value| value.is_finite()));
            Vec4::from_array(values)
        };
        let textures = material["textures"].as_array().unwrap();
        assert_eq!(textures.len(), 1, "Original sky cube binding");
        assert_eq!(textures[0]["name"], "colorMap");
        let sampler = &textures[0]["samplerState"];
        for field in ["clampU", "clampV", "clampW"] {
            assert_eq!(sampler[field], true);
        }
        assert_eq!(sampler["filter"], "linear");
        assert_eq!(sampler["mipMap"], "disabled");
        let primary: Arc<[u8]> = read(&model.vertices_file).into();
        let indices: Arc<[u8]> = read(&model.indices_file).into();
        assert!(
            surface.vertex_byte_offset + u64::from(surface.vertex_count) * 32
                <= primary.len() as u64
        );
        let first = surface.first_index as usize * 2;
        let end = first + surface.tri_count as usize * 6;
        assert!(end <= indices.len());
        assert!(
            indices[first..end]
                .chunks_exact(2)
                .all(
                    |bytes| u32::from(u16::from_le_bytes(bytes.try_into().unwrap()))
                        < surface.vertex_count
                )
        );
        LoadedSky {
            geometry: NativeWorldGeometry {
                primary,
                auxiliary: Vec::new(),
                indices,
                vertex_stride: 32,
            },
            image: textures[0]["image"]
                .as_str()
                .expect("Original sky cube name")
                .to_owned(),
            rotation_size: constant("skyBoxRotationSize"),
            color_parms: constant("skyColorParms"),
            vertex_byte_offset: surface.vertex_byte_offset,
            first_index: surface.first_index,
            index_count: surface.tri_count * 3,
        }
    }
}

impl LoadedSky {
    /// The returned image participates in the world's ordinary asset readiness.
    pub(crate) fn install(
        self,
        commands: &mut Commands,
        server: &AssetServer,
        geometries: &mut Assets<NativeWorldGeometry>,
    ) -> Handle<Image> {
        let image = super::material::image(server, &self.image);
        commands.insert_resource(NativeSky {
            geometry: geometries.add(self.geometry),
            image: image.clone(),
            rotation_size: self.rotation_size,
            color_parms: self.color_parms,
            vertex_byte_offset: self.vertex_byte_offset,
            first_index: self.first_index,
            index_count: self.index_count,
            session: None,
            rotation_tick: 0,
            rotation_degrees: 360.,
        });
        image
    }
}

#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub(crate) struct NativeSky {
    geometry: Handle<NativeWorldGeometry>,
    image: Handle<Image>,
    rotation_size: Vec4,
    color_parms: Vec4,
    vertex_byte_offset: u64,
    first_index: u32,
    index_count: u32,
    session: Option<crate::session::SessionId>,
    rotation_tick: u32,
    rotation_degrees: f32,
}

#[derive(ShaderType)]
struct SkyUniform {
    scene: NativeWorldScene,
    world_rows: [Vec4; 4],
    rotation_size: Vec4,
    color_parms: Vec4,
}

/// 004189a0 builds quaternion z=sin(dvar*radians), w=cos(dvar*radians),
/// using the full angle. Preserve this original pose rather than Euler yaw.
fn rotation_rows(degrees: f32) -> [Vec4; 4] {
    let angle = degrees * 0.017453292;
    let (sin, cos) = angle.sin_cos();
    let matrix = Mat4::from_quat(Quat::from_xyzw(0., 0., sin, cos));
    [matrix.row(0), matrix.row(1), matrix.row(2), matrix.row(3)]
}

fn advance_rotation(degrees: &mut f32, ticks: u32) {
    for _ in 0..ticks {
        *degrees -= 0.025;
        if *degrees < 0. {
            *degrees += 360.;
        }
    }
}

fn update_rotation(
    sky: Option<ResMut<NativeSky>>,
    session: Res<crate::session::SessionId>,
    clock: Res<crate::session::SessionClock>,
    time: Res<Time<Virtual>>,
) {
    let Some(mut sky) = sky else {
        return;
    };
    // GSC rotate_skydome decrements before its first wait0.1. Repeated f32
    // subtraction is observable over long sessions, so do not replace it with
    // an ideal angular-velocity formula. No ticks advance while paused.
    let tick = clock.milliseconds(&time) as u32 / 100 + 1;
    if sky.session == Some(*session) && sky.rotation_tick == tick {
        return;
    }
    if sky.session != Some(*session) || sky.rotation_tick > tick {
        sky.session = Some(*session);
        sky.rotation_tick = 0;
        sky.rotation_degrees = 360.;
    }
    let elapsed = tick - sky.rotation_tick;
    advance_rotation(&mut sky.rotation_degrees, elapsed);
    sky.rotation_tick = tick;
}

pub(crate) struct NativeSkyPlugin;
impl Plugin for NativeSkyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<NativeSky>::default())
            .add_systems(PostUpdate, update_rotation);
        load_shader_library!(app, "native_sky.wesl");
        embedded_asset!(app, "native_sky_entry.wesl");
        let shader = load_embedded_asset!(app, "native_sky_entry.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .insert_resource(SkyPipeline {
                shader,
                layout: BindGroupLayoutDescriptor::new(
                    "T6 sky constants and cube",
                    &BindGroupLayoutEntries::sequential(
                        ShaderStages::VERTEX_FRAGMENT,
                        (
                            uniform_buffer::<SkyUniform>(false),
                            texture_cube(TextureSampleType::Float { filterable: true }),
                            sampler(SamplerBindingType::Filtering),
                        ),
                    ),
                ),
            })
            .init_resource::<SpecializedRenderPipelines<SkyPipeline>>()
            .add_systems(Render, prepare_sky.in_set(RenderSystems::PrepareBindGroups));
    }
}

#[derive(Resource)]
struct SkyPipeline {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
}
impl SpecializedRenderPipeline for SkyPipeline {
    type Key = (TextureFormat, u32);
    fn specialize(&self, (format, samples): Self::Key) -> RenderPipelineDescriptor {
        assert!(
            !format.is_srgb(),
            "Source sky writes gamma2-encoded scene RGB"
        );
        RenderPipelineDescriptor {
            label: Some("T6 skycubemaphdr2306aa25".into()),
            layout: vec![self.layout.clone()],
            vertex: VertexState {
                shader: self.shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: 32,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![VertexAttribute {
                        format: VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    }],
                }],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                front_face: FrontFace::Cw,
                cull_mode: Some(Face::Back),
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: super::frame::NATIVE_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: samples,
                ..default()
            },
            ..default()
        }
    }
}

#[derive(Component)]
pub(crate) struct PreparedSky {
    uniform: UniformBuffer<SkyUniform>,
    buffer_id: BufferId,
    image_view: TextureViewId,
    group: BindGroup,
    pipeline: CachedRenderPipelineId,
}

fn sky_sampler() -> SamplerDescriptor<'static> {
    SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: MipmapFilterMode::Nearest,
        lod_min_clamp: 0.,
        lod_max_clamp: f32::MAX,
        anisotropy_clamp: 1,
        ..default()
    }
}

fn prepare_sky(
    mut commands: Commands,
    sky: Option<Res<NativeSky>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<SkyPipeline>,
    mut variants: ResMut<SpecializedRenderPipelines<SkyPipeline>>,
    images: Res<RenderAssets<GpuImage>>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    mut views: Query<(
        Entity,
        &NativeWorldScene,
        &ViewTarget,
        &Msaa,
        Option<&mut PreparedSky>,
    )>,
) {
    let Some(sky) = sky else {
        return;
    };
    let Some(image) = images.get(sky.image.id()) else {
        return;
    };
    if geometries.get(sky.geometry.id()).is_none() {
        return;
    }
    for (entity, scene, target, msaa, prepared) in &mut views {
        let constants = SkyUniform {
            scene: *scene,
            world_rows: rotation_rows(sky.rotation_degrees),
            rotation_size: sky.rotation_size,
            color_parms: sky.color_parms,
        };
        let id = variants.specialize(
            &cache,
            &pipeline,
            (target.main_texture_format(), msaa.samples()),
        );
        let mut prepared = match prepared {
            Some(prepared) => prepared,
            None => {
                let mut uniform = UniformBuffer::from(constants);
                uniform.write_buffer(&device, &queue);
                let sampler = device.create_sampler(&sky_sampler());
                let group = device.create_bind_group(
                    "T6 sky cube",
                    &cache.get_bind_group_layout(&pipeline.layout),
                    &BindGroupEntries::sequential((
                        uniform.binding().unwrap(),
                        &image.texture_view,
                        &sampler,
                    )),
                );
                commands.entity(entity).insert(PreparedSky {
                    buffer_id: uniform.buffer().unwrap().id(),
                    uniform,
                    image_view: image.texture_view.id(),
                    group,
                    pipeline: id,
                });
                continue;
            }
        };
        prepared.uniform.set(constants);
        prepared.uniform.write_buffer(&device, &queue);
        prepared.pipeline = id;
        let buffer_id = prepared.uniform.buffer().unwrap().id();
        if prepared.buffer_id != buffer_id || prepared.image_view != image.texture_view.id() {
            let sampler = device.create_sampler(&sky_sampler());
            prepared.group = device.create_bind_group(
                "T6 sky cube",
                &cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((
                    prepared.uniform.binding().unwrap(),
                    &image.texture_view,
                    &sampler,
                )),
            );
            prepared.buffer_id = buffer_id;
            prepared.image_view = image.texture_view.id();
        }
    }
}

/// Source cameraRegion emissiveOpaque, sortKey4: after quasi-opaque, before
/// ordinary emissive geometry and before the water's scene/floatZ snapshots.
pub(crate) fn render_sky(
    view: ViewQuery<(
        &ViewTarget,
        &ViewDepthStencilTexture,
        &ExtractedCamera,
        &PreparedSky,
        Option<&bevy::camera::MainPassResolutionOverride>,
    )>,
    sky: Option<Res<NativeSky>>,
    cache: Res<PipelineCache>,
    geometries: Res<RenderAssets<PreparedNativeGeometry>>,
    mut ctx: RenderContext,
) {
    let Some(sky) = sky else {
        return;
    };
    let (target, depth, camera, prepared, resolution) = view.into_inner();
    let (Some(pipeline), Some(geometry)) = (
        cache.get_render_pipeline(prepared.pipeline),
        geometries.get(sky.geometry.id()),
    ) else {
        return;
    };
    let attachments = [Some(target.get_color_attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 sky emissiveOpaque sortKey4"),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    if let Some(viewport) =
        bevy::camera::Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution)
    {
        pass.set_camera_viewport(&viewport);
    }
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &prepared.group, &[]);
    pass.set_vertex_buffer(0, geometry.primary.slice(sky.vertex_byte_offset..));
    pass.set_index_buffer(geometry.indices.slice(..), IndexFormat::Uint16);
    pass.draw_indexed(sky.first_index..sky.first_index + sky.index_count, 0, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sky_session_rotation_preserves_source_steps_and_quaternion() {
        let angle = |degrees| {
            rotation_rows(degrees)[0]
                .y
                .atan2(rotation_rows(degrees)[0].x)
        };
        let mut degrees = 360.;
        advance_rotation(&mut degrees, 1);
        assert_eq!(degrees, 359.975);
        assert!((angle(degrees) - 0.05_f32.to_radians()).abs() < 1e-6);
        advance_rotation(&mut degrees, 1);
        assert!((angle(degrees) - 0.1_f32.to_radians()).abs() < 1e-6);
        advance_rotation(&mut degrees, 14399);
        assert_eq!(degrees, 0.026138125);
        assert!((angle(45.) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn original_world_sky_linkage_loads_packed_source() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let world: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("world.json")).unwrap()).unwrap();
        let definition: SkyDefinition = serde_json::from_value(world["sky"].clone()).unwrap();
        let loaded = definition.load(&root);
        assert_eq!(loaded.geometry.primary.len(), 316 * 32);
        assert_eq!(loaded.index_count, 448 * 3);
        assert_eq!(loaded.image, "skybox_zm_tomb_ft");
        assert_eq!(loaded.color_parms, Vec4::ONE);
    }
}
