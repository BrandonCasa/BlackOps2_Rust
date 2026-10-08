//! Authored surface attributes alongside Bevy's native ray scene. No second TLAS.
use super::{opaque, streaming};
use bevy::{
    asset::{AssetId, uuid_handle},
    diagnostic::FrameCount,
    mesh::VertexAttributeValues,
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        gpu_readback::{Readback, ReadbackComplete},
        render_asset::RenderAssets,
        render_resource::{binding_types::*, *},
        renderer::{RenderDevice, RenderQueue},
        storage::{GpuShaderBuffer, ShaderBuffer},
        texture::{FallbackImage, GpuImage},
    },
    shader::{Shader, ShaderImport},
};
use std::{collections::HashMap, num::NonZeroU32, sync::Arc};

const SHADER: Handle<Shader> = uuid_handle!("33199dcf-32d3-4e0c-a862-0c8bb4e96ea0");
const TEXTURES: u32 = 2048;
const NONE: u32 = u32::MAX;
const FEEDBACK_BYTES: u64 = (TEXTURES as u64 + 1) * 4;

#[derive(Clone, Copy, ShaderType)]
pub(super) struct Vertex {
    color: Vec4,
    uv: Vec4,
    uv3: Vec4,
    rotation1: Vec4,
    rotation2: Vec4,
}
#[derive(Clone, Copy, ShaderType)]
pub(super) struct Material {
    surface: opaque::SurfaceParameters,
    color: Vec4,
    emissive: Vec4,
    response: Vec4,
    alpha: Vec4,
    textures0: UVec4,
    textures1: UVec4,
    textures2: UVec4,
    textures3: UVec4,
}
#[derive(Resource, Clone, Default, ExtractResource)]
pub(super) struct Data {
    vertices: Arc<Vec<Vertex>>,
    materials: Arc<Vec<Material>>,
    textures: Arc<Vec<Handle<Image>>>,
    tails: Arc<Vec<Vec4>>,
    feedback: Option<Handle<ShaderBuffer>>,
    revision: u64,
    elapsed: f32,
}
#[derive(Resource, Default)]
pub(super) struct Registry {
    meshes: HashMap<AssetId<Mesh>, u32>,
    images: HashMap<AssetId<Image>, u32>,
}
#[derive(Resource)]
pub(super) struct Layout(pub BindGroupLayoutDescriptor);
#[derive(Resource, Default)]
pub(super) struct Prepared {
    pub group: Option<BindGroup>,
    revision: u64,
    views: Vec<TextureViewId>,
    samplers: Vec<SamplerId>,
    vertices: Option<Buffer>,
    materials: Option<Buffer>,
    tails: Option<Buffer>,
    feedback: Option<Buffer>,
    reset: Option<Buffer>,
    vertex_data: Option<Arc<Vec<Vertex>>>,
    material_data: Option<Arc<Vec<Material>>>,
    tail_data: Option<Arc<Vec<Vec4>>>,
    clock: Option<Buffer>,
}

#[derive(Resource, Default)]
struct Feedback {
    mips: Vec<u32>,
    frame: u32,
    updated: f64,
    live: bool,
}
#[derive(Resource, Default, serde::Serialize)]
pub(crate) struct RaySurfaceStats {
    feedback_frames: u64,
    hit_textures: usize,
}

#[derive(Component)]
pub(super) struct Usage {
    pub owner: Entity,
    pub textures: Vec<AssetId<Image>>,
}

impl Data {
    pub(super) fn textures_for(&self, index: usize) -> Vec<AssetId<Image>> {
        let material = self.materials[index];
        [
            material.textures0,
            material.textures1,
            material.textures2,
            material.textures3,
        ]
        .into_iter()
        .flat_map(|value| value.to_array())
        .filter(|id| *id != NONE)
        .map(|id| self.textures[id as usize].id())
        .collect()
    }
}

pub(super) struct RaySurfacePlugin;
impl Plugin for RaySurfacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Data>()
            .init_resource::<Registry>()
            .init_resource::<Feedback>()
            .init_resource::<RaySurfaceStats>()
            .add_plugins(ExtractResourcePlugin::<Data>::default())
            .add_systems(Startup, feedback_buffer)
            .add_systems(Update, shaders)
            .add_systems(
                PostUpdate,
                demands
                    .before(streaming::TextureStreamingSet)
                    .after(bevy::transform::TransformSystems::Propagate),
            );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<Prepared>()
                .add_systems(RenderStartup, layout)
                .add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups));
        }
    }
}

fn feedback_buffer(
    mut commands: Commands,
    mut buffers: ResMut<Assets<ShaderBuffer>>,
    mut data: ResMut<Data>,
) {
    let mut initial = vec![NONE; TEXTURES as usize + 1];
    initial[0] = 0; // Native frame stamp; remaining words are full-chain mip minima.
    let buffer = buffers.add(ShaderBuffer::from(initial));
    data.feedback = Some(buffer.clone());
    data.revision = data.revision.wrapping_add(1);
    commands
        .spawn((
            Readback::buffer(buffer),
            Name::new("Lighting texture mip feedback"),
        ))
        .observe(feedback_complete);
}

fn feedback_complete(
    event: On<ReadbackComplete>,
    time: Res<Time<Real>>,
    mut feedback: ResMut<Feedback>,
    mut stats: ResMut<RaySurfaceStats>,
) {
    if event.data.len() != FEEDBACK_BYTES as usize {
        return;
    }
    let frame = u32::from_le_bytes(event.data[..4].try_into().expect("Frame stamp"));
    // Native readbacks run every frame, including frames without lighting work.
    // Do not renew stale producer demand, or idle probes would pin detail forever.
    if frame == feedback.frame || frame.wrapping_sub(feedback.frame) > u32::MAX / 2 {
        return;
    }
    feedback.frame = frame;
    feedback.updated = time.elapsed_secs_f64();
    feedback.mips = event.data[4..]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| u32::from_le_bytes(*bytes))
        .collect();
    feedback.live |= feedback.mips.iter().any(|mip| *mip != NONE);
    stats.feedback_frames += 1;
    stats.hit_textures = feedback.mips.iter().filter(|mip| **mip != NONE).count();
}

fn shaders(mut shaders: ResMut<Assets<Shader>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some((id, source)) = shaders.iter().find_map(|(id, shader)| {
        (shader.import_path == ShaderImport::Custom("bevy_solari::scene_bindings".into()))
            .then(|| (id, shader.source.as_str().to_owned()))
    }) else {
        return;
    };
    // Tangent magnitudes carry a stable companion vertex ID only in ray meshes.
    // Restore handedness before native TBN construction. Raster meshes are untouched.
    assert_eq!(source.matches("        vertices[0].tangent.w,").count(), 1);
    shaders
        .insert(
            id,
            Shader::from_wgsl(
                source.replace(
                    "        vertices[0].tangent.w,",
                    "        sign(vertices[0].tangent.w),",
                ),
                "origins::native_ray_bindings",
            ),
        )
        .expect("Native scene shader");
    shaders
        .insert(
            SHADER.id(),
            Shader::from_wgsl(include_str!("ray_surface.wgsl"), "origins::ray_surface"),
        )
        .expect("Ray surface shader");
    *done = true;
}

impl Registry {
    pub(super) fn vertices(
        &mut self,
        mesh: &Mesh,
        id: AssetId<Mesh>,
        data: &mut Data,
        controls: bool,
    ) -> u32 {
        if data.vertices.is_empty() {
            Arc::make_mut(&mut data.vertices).push(Vertex {
                color: Vec4::ONE,
                uv: Vec4::ZERO,
                uv3: Vec4::ZERO,
                rotation1: Vec4::new(1., 0., 0., 1.),
                rotation2: Vec4::new(1., 0., 0., 1.),
            });
        }
        // Single-layer and tile UVs are already in native ray vertices.
        // Share one neutral record instead of retaining five extra vectors.
        if !controls {
            return u32::MAX;
        }
        if let Some(offset) = self.meshes.get(&id) {
            return *offset;
        }
        let count = mesh.count_vertices();
        let offset = data.vertices.len() as u32;
        assert!(
            offset as usize + count < (1 << 24),
            "Exact f32 ray vertex IDs"
        );
        let four = |attribute, index, fallback: Vec4| match mesh.attribute(attribute) {
            Some(VertexAttributeValues::Float32x4(values)) => values
                .get(index)
                .map_or(fallback, |value| Vec4::from_array(*value)),
            _ => fallback,
        };
        let vertices = Arc::make_mut(&mut data.vertices);
        for index in 0..count {
            let uv0 = match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
                Some(VertexAttributeValues::Float32x2(values)) => values
                    .get(index)
                    .map_or(Vec2::ZERO, |value| Vec2::from_array(*value)),
                _ => Vec2::ZERO,
            };
            let uv3 = match mesh.attribute(opaque::LAYER3_UV) {
                Some(VertexAttributeValues::Float32x2(values)) => values
                    .get(index)
                    .map_or(uv0, |value| Vec2::from_array(*value)),
                _ => uv0,
            };
            let color = match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
                Some(VertexAttributeValues::Float32x3(values)) => values
                    .get(index)
                    .map_or(Vec4::ONE, |value| Vec3::from_array(*value).extend(1.)),
                _ => four(Mesh::ATTRIBUTE_COLOR, index, Vec4::ONE),
            };
            vertices.push(Vertex {
                color,
                uv: four(
                    opaque::LAYER_UV,
                    index,
                    Vec4::new(uv0.x, uv0.y, uv0.x, uv0.y),
                ),
                uv3: uv3.extend(0.).extend(0.),
                rotation1: four(opaque::ROTATION1, index, Vec4::new(1., 0., 0., 1.)),
                rotation2: four(opaque::ROTATION2, index, Vec4::new(1., 0., 0., 1.)),
            });
        }
        self.meshes.insert(id, offset);
        data.revision = data.revision.wrapping_add(1);
        offset
    }

    pub(super) fn material(
        &mut self,
        base: &StandardMaterial,
        extension: Option<&opaque::OpaqueSurface>,
        images: &Assets<Image>,
        data: &mut Data,
    ) -> u32 {
        let first_handles = extension.map_or_else(
            || {
                [
                    base.normal_map_texture.clone(),
                    None,
                    None,
                    base.metallic_roughness_texture.clone(),
                    None,
                    None,
                    None,
                    base.base_color_texture.clone(),
                    None,
                    None,
                    None,
                    base.emissive_texture.clone(),
                ]
            },
            |surface| surface.ray_textures(base),
        );
        let mut handles = [const { None }; 16];
        handles[..12].clone_from_slice(&first_handles);
        // StandardMaterial's coating and occlusion are independent channels.
        // Extended surfaces evaluate their packed response alpha instead.
        if extension.is_none() {
            handles[12] = base.clearcoat_texture.clone();
            handles[13] = base.occlusion_texture.clone();
            handles[14] = base.clearcoat_roughness_texture.clone();
        }
        let mut ids = [NONE; 16];
        for (index, handle) in handles.into_iter().enumerate() {
            let Some(handle) = handle else {
                continue;
            };
            ids[index] = *self.images.entry(handle.id()).or_insert_with(|| {
                let id = data.textures.len() as u32;
                assert!(
                    id < TEXTURES,
                    "Ray material texture closure exceeds binding layout"
                );
                Arc::make_mut(&mut data.textures).push(handle);
                id
            });
        }
        let normal_encoding = base
            .normal_map_texture
            .as_ref()
            .and_then(|handle| images.get(handle))
            .map_or(0, |image| {
                super::pbr::NormalEncoding::for_image(image).flag(0)
            });
        let surface = extension.map_or(
            opaque::SurfaceParameters {
                response: Vec4::new(1., base.clearcoat_perceptual_roughness, 0., 1.),
                modes: UVec4::new(
                    u32::from(ids[0] != NONE) * 2 | (u32::from(base.unlit) << 6),
                    u32::from(ids[3] != NONE),
                    0,
                    normal_encoding,
                ),
                layers: UVec4::ONE,
                reveal: Vec4::ZERO,
                wetness: Vec4::ZERO,
                water: Vec4::ZERO,
                velocity: Vec4::ZERO,
                effect: opaque::EffectParameters::default(),
            },
            opaque::OpaqueSurface::ray_parameters,
        );
        let alpha = match base.alpha_mode {
            AlphaMode::Mask(cutoff) => cutoff,
            _ => -1.,
        };
        let index = data.materials.len() as u32;
        Arc::make_mut(&mut data.materials).push(Material {
            surface,
            color: base.base_color.to_linear().to_vec4(),
            emissive: base.emissive.to_vec3().extend(0.),
            response: Vec4::new(
                base.perceptual_roughness,
                base.metallic,
                base.reflectance,
                base.clearcoat,
            ),
            alpha: Vec4::new(
                alpha,
                u8::from(extension.is_some()) as f32,
                u8::from(base.flip_normal_map_y) as f32,
                u8::from(base.double_sided) as f32,
            ),
            textures0: UVec4::from_array(ids[..4].try_into().expect("Four channels")),
            textures1: UVec4::from_array(ids[4..8].try_into().expect("Four channels")),
            textures2: UVec4::from_array(ids[8..12].try_into().expect("Four channels")),
            textures3: UVec4::from_array(ids[12..].try_into().expect("Four channels")),
        });
        data.revision = data.revision.wrapping_add(1);
        index
    }
}

pub(super) fn encode_vertices(mesh: &mut Mesh, offset: u32) {
    if let Some(VertexAttributeValues::Float32x4(tangents)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_TANGENT)
    {
        for (index, tangent) in tangents.iter_mut().enumerate() {
            let id = if offset == u32::MAX {
                1
            } else {
                offset + index as u32 + 1
            };
            // A malformed zero handedness must not turn the sidecar ID into
            // zero and underflow the shader's one-based lookup.
            tangent[3] = if tangent[3] < 0. {
                -(id as f32)
            } else {
                id as f32
            };
        }
    }
}

fn demands(
    time: Res<Time<Real>>,
    streams: Res<streaming::TextureStreams>,
    images: Res<Assets<Image>>,
    mut requests: ResMut<streaming::TextureStreamingRequests>,
    mut data: ResMut<Data>,
    feedback: Res<Feedback>,
    camera: Query<&GlobalTransform, With<super::GameCamera>>,
    uses: Query<&Usage>,
    owners: Query<(
        &GlobalTransform,
        Option<&bevy::camera::primitives::Aabb>,
        &InheritedVisibility,
    )>,
) {
    data.elapsed = time.elapsed_secs();
    let mut tails = Vec::with_capacity(data.textures.len());
    for handle in data.textures.iter() {
        let descriptor = streams.mip_descriptor(handle.id());
        let (no_mips, anisotropy) =
            images
                .get(handle)
                .map_or((false, 1), |image| match &image.sampler {
                    bevy::image::ImageSampler::Descriptor(s) => {
                        (s.lod_max_clamp == 0., s.anisotropy_clamp.max(1))
                    }
                    _ => (false, 1),
                });
        tails.push(descriptor.map_or(
            Vec4::new(0., 0., anisotropy as f32, u8::from(no_mips) as f32),
            |d| {
                Vec4::new(
                    d.full_extent.x as f32,
                    d.full_extent.y as f32,
                    anisotropy as f32,
                    u8::from(no_mips) as f32,
                )
            },
        ));
    }
    if *data.tails != tails {
        data.tails = Arc::new(tails);
        data.revision = data.revision.wrapping_add(1);
    }
    if feedback.live {
        // GPU hits cover every cascade and the reflection cube. They refer to
        // full-chain levels and stable Image IDs, even while the live view is
        // only a streamed tail. One producer request serves all render passes.
        // Recent per-mip footprints bridge asynchronous readback and rolling
        // stochastic batches. Unobserved detail expires rather than being
        // erased by every next frame's sparse sample.
        let expires = feedback.updated + 2.;
        if expires > time.elapsed_secs_f64() {
            for (handle, mip) in data.textures.iter().zip(&feedback.mips) {
                if *mip != NONE {
                    requests.request(
                        handle.id(),
                        streaming::TextureConsumer::Lighting,
                        *mip as usize,
                        expires,
                    );
                }
            }
        }
        return;
    }
    let Ok(camera) = camera.single() else {
        return;
    };
    let mut wanted = HashMap::<(AssetId<Image>, streaming::TextureConsumer), usize>::new();
    for usage in &uses {
        let Ok((pose, bounds, visible)) = owners.get(usage.owner) else {
            continue;
        };
        if !visible.get() {
            continue;
        }
        let center = bounds.map_or(pose.translation(), |b| {
            pose.affine().transform_point3a(b.center).into()
        });
        let radius = bounds.map_or(1., |b| {
            (pose.affine().matrix3.abs() * b.half_extents).length()
        });
        let distance = (camera.translation().distance(center) - radius).max(0.);
        // Conservative nearest hit over the rolling near field and the cube.
        // More distant cascades have larger ray cones and cannot require a
        // sharper mip than this near-field bound.
        for (consumer, footprint) in [
            (
                streaming::TextureConsumer::StaticGi,
                (distance - 32.).max(2.) * 0.0625,
            ),
            (
                streaming::TextureConsumer::Reflection,
                distance.max(0.5) * 0.0078125,
            ),
        ] {
            for id in &usage.textures {
                if let Some(mip) = streams.mip_for_footprint(*id, footprint) {
                    wanted
                        .entry((*id, consumer))
                        .and_modify(|previous| *previous = (*previous).min(mip))
                        .or_insert(mip);
                }
            }
        }
    }
    for ((id, consumer), mip) in wanted {
        requests.request(id, consumer, mip, time.elapsed_secs_f64() + 2.);
    }
}

pub(super) fn layout(mut commands: Commands) {
    commands.insert_resource(Layout(BindGroupLayoutDescriptor::new(
        "Authored ray surfaces",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                storage_buffer_read_only::<Vec<Vertex>>(false),
                storage_buffer_read_only::<Vec<Material>>(false),
                texture_2d(TextureSampleType::Float { filterable: true })
                    .count(NonZeroU32::new(TEXTURES).expect("Textures")),
                sampler(SamplerBindingType::Filtering)
                    .count(NonZeroU32::new(TEXTURES).expect("Samplers")),
                storage_buffer_read_only::<Vec<Vec4>>(false),
                storage_buffer::<Vec<u32>>(false),
                storage_buffer_read_only::<Vec4>(false),
            ),
        ),
    )));
}

fn prepare(
    data: Res<Data>,
    images: Res<RenderAssets<GpuImage>>,
    buffers: Res<RenderAssets<GpuShaderBuffer>>,
    fallback: Res<FallbackImage>,
    layout: Res<Layout>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    frame: Res<FrameCount>,
    mut prepared: ResMut<Prepared>,
) {
    let Some(feedback) = data
        .feedback
        .as_ref()
        .and_then(|handle| buffers.get(handle))
    else {
        return;
    };
    let clock_bytes = data.elapsed.to_le_bytes();
    if let Some(clock) = &prepared.clock {
        queue.write_buffer(clock, 0, &clock_bytes);
    } else {
        let mut initial = [0u8; 16];
        initial[..4].copy_from_slice(&clock_bytes);
        prepared.clock = Some(device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Shared material effect time"),
            contents: &initial,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        }));
    }
    if prepared.reset.is_none() {
        let mut initial = vec![255u8; FEEDBACK_BYTES as usize];
        initial[..4].copy_from_slice(&0u32.to_le_bytes());
        prepared.reset = Some(device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Lighting mip feedback reset"),
            contents: &initial,
            usage: BufferUsages::COPY_SRC | BufferUsages::COPY_DST,
        }));
    }
    // Queue writes precede graph command submission. Only reset_feedback copies
    // this stamp into the actual result, and only when producer passes run.
    queue.write_buffer(
        prepared.reset.as_ref().expect("Feedback reset"),
        0,
        &frame.0.to_le_bytes(),
    );
    if data.vertices.is_empty()
        || data.materials.is_empty()
        || data.tails.len() != data.textures.len()
    {
        return;
    }
    let mut gpu: Vec<_> = data
        .textures
        .iter()
        .map(|handle| images.get(handle).unwrap_or(&fallback.d2))
        .collect();
    if gpu.is_empty() {
        // A texture-free material still needs a nonempty partially bound array.
        // Shader channels all use NONE and cannot address this sentinel slot.
        gpu.push(&fallback.d2);
    }
    let views: Vec<_> = gpu.iter().map(|image| image.texture_view.id()).collect();
    let sampler_ids: Vec<_> = gpu.iter().map(|image| image.sampler.id()).collect();
    if prepared.group.is_some()
        && prepared.revision == data.revision
        && prepared.views == views
        && prepared.samplers == sampler_ids
        && prepared
            .feedback
            .as_ref()
            .is_some_and(|old| old.id() == feedback.buffer.id())
    {
        return;
    }
    if prepared.revision != data.revision || prepared.group.is_none() {
        if prepared
            .vertex_data
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &data.vertices))
        {
            upload(
                data.vertices.as_ref(),
                &mut prepared.vertices,
                &device,
                &queue,
            );
            prepared.vertex_data = Some(data.vertices.clone());
        }
        if prepared
            .material_data
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &data.materials))
        {
            upload(
                data.materials.as_ref(),
                &mut prepared.materials,
                &device,
                &queue,
            );
            prepared.material_data = Some(data.materials.clone());
        }
        if prepared
            .tail_data
            .as_ref()
            .is_none_or(|old| !Arc::ptr_eq(old, &data.tails))
        {
            if data.tails.is_empty() {
                upload(&vec![Vec4::ZERO], &mut prepared.tails, &device, &queue);
            } else {
                upload(data.tails.as_ref(), &mut prepared.tails, &device, &queue);
            }
            prepared.tail_data = Some(data.tails.clone());
        }
    }
    let textures: Vec<_> = gpu.iter().map(|image| &*image.texture_view).collect();
    let samplers: Vec<_> = gpu.iter().map(|image| &*image.sampler).collect();
    prepared.group = Some(
        device.create_bind_group(
            "Authored ray surfaces",
            &cache.get_bind_group_layout(&layout.0),
            &BindGroupEntries::sequential((
                prepared
                    .vertices
                    .as_ref()
                    .expect("Ray vertices")
                    .as_entire_binding(),
                prepared
                    .materials
                    .as_ref()
                    .expect("Ray materials")
                    .as_entire_binding(),
                BindingResource::TextureViewArray(&textures),
                BindingResource::SamplerArray(&samplers),
                prepared
                    .tails
                    .as_ref()
                    .expect("Ray texture mip descriptors")
                    .as_entire_binding(),
                feedback.buffer.as_entire_binding(),
                prepared
                    .clock
                    .as_ref()
                    .expect("Effect clock")
                    .as_entire_binding(),
            )),
        ),
    );
    prepared.revision = data.revision;
    prepared.views = views;
    prepared.samplers = sampler_ids;
    prepared.feedback = Some(feedback.buffer.clone());
}

/// Record once immediately before all shared GI/reflection producers. Bevy's
/// render_system submits native readback copies after the complete render graph,
/// so ReadbackComplete observes all atomic minima from these passes.
pub(super) fn reset_feedback(prepared: &Prepared, encoder: &mut CommandEncoder) {
    if let (Some(reset), Some(feedback)) = (&prepared.reset, &prepared.feedback) {
        encoder.copy_buffer_to_buffer(reset, 0, feedback, 0, FEEDBACK_BYTES);
    }
}

fn upload<T: ShaderType + encase::internal::WriteInto>(
    data: &T,
    buffer: &mut Option<Buffer>,
    device: &RenderDevice,
    queue: &RenderQueue,
) {
    // The extracted Arc is the sole persistent CPU copy. StorageBuffer<T>
    // would retain another Vec plus its serialized scratch for every channel.
    let mut encoded = encase::StorageBuffer::new(Vec::new());
    encoded
        .write(data)
        .expect("Valid ray surface storage layout");
    let bytes = encoded.into_inner();
    if let Some(buffer) = buffer.as_ref().filter(|b| b.size() >= bytes.len() as u64) {
        queue.write_buffer(buffer, 0, &bytes);
    } else {
        *buffer = Some(device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("Authored ray surface data"),
            contents: &bytes,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        }));
    }
}
