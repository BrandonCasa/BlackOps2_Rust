//! Source scene attachments, reciprocal-depth resolve and final display transfer.
use super::native_effect::NativeEffectView;
use super::native_water::{PreparedWaterView, render_water};
use bevy::{
    asset::{RenderAssetUsages, embedded_asset, load_embedded_asset},
    camera::{CameraOutputMode, CameraUpdateSystems},
    core_pipeline::{
        Core3dSystems, FullscreenShader,
        core_3d::{AlphaMask3d, Opaque3d, Transparent3d, prepare_core_3d_depth_textures},
        schedule::Core3d,
        tonemapping::Tonemapping,
        upscaling::upscaling,
    },
    ecs::query::QueryItem,
    image::ToExtents,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        camera::{CameraMainPassTextureFormats, ExtractedCamera, extract_cameras},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_phase::{ViewBinnedRenderPhases, ViewSortedRenderPhases},
        render_resource::*,
        renderer::{RenderContext, RenderDevice, ViewQuery},
        sync_component::SyncComponent,
        sync_world::RenderEntity,
        texture::{CachedTexture, DepthStencilAttachment, GpuImage, TextureCache},
        view::{
            ExtractedView, ViewDepthStencilTexture, ViewTarget, prepare_view_attachments,
            prepare_view_targets,
        },
    },
    shader::ShaderDefVal,
};

/// Native 00428840 chooses D32_FLOAT_S8X24_UINT. All native scene pipelines
/// sharing this attachment must use this format and reverse-Z comparisons.
pub const NATIVE_DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32FloatStencil8;
pub(crate) const NATIVE_COLOR_FORMAT: TextureFormat = TextureFormat::Rgba8Unorm;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NativeMainPass;

/// Native blending always quantizes to UNORM8, including when DLSS uses HDR.
#[derive(Component)]
pub(crate) struct NativeSceneColor {
    pub image: CachedTexture,
    pub size: UVec2,
}

impl NativeSceneColor {
    pub(crate) fn attachment(&self) -> RenderPassColorAttachment<'_> {
        RenderPassColorAttachment {
            view: &self.image.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        }
    }
}

/// Full-resolution encoded E for the source film and later UI camera.
#[derive(Resource)]
pub(crate) struct NativeEncodedScene {
    pub image: CachedTexture,
    pub size: UVec2,
}

/// Camera owner marker, also present in the render world. The plugin maintains
/// its ordinary floatZ Image through NativeEffectView.
#[derive(Component, Clone, Copy)]
pub(crate) struct NativeRenderView;

#[derive(Component)]
pub struct NativeDepthImage {
    float_z: AssetId<Image>,
    resolved_scene: AssetId<Image>,
}

#[derive(Component)]
pub struct NativeSceneImage(pub(crate) Handle<Image>);

impl SyncComponent<RenderApp> for NativeRenderView {
    type Target = (
        Self,
        NativeDepthImage,
        NativeDepthAttachment,
        NativeSceneColor,
        PreparedNativeFrame,
    );
}

impl ExtractComponent<RenderApp> for NativeRenderView {
    type QueryData = (
        Option<&'static NativeEffectView>,
        Option<&'static NativeSceneImage>,
    );
    type QueryFilter = With<NativeRenderView>;
    type Out = (Self, NativeDepthImage);

    fn extract_component((depth, scene): QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
        Some((
            Self,
            NativeDepthImage {
                float_z: depth?.float_z.id(),
                resolved_scene: scene?.0.id(),
            },
        ))
    }
}

pub struct NativeFramePlugin;

impl Plugin for NativeFramePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<NativeRenderView>::default())
            .add_systems(PostUpdate, maintain_float_z.after(CameraUpdateSystems));
        embedded_asset!(app, "depth_resolve.wesl");
        embedded_asset!(app, "scene_output.wesl");
        let depth_shader = load_embedded_asset!(app, "depth_resolve.wesl");
        let output_shader = load_embedded_asset!(app, "scene_output.wesl");
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(
                ExtractSchedule,
                extract_native_target_format
                    .after(extract_cameras)
                    .before(bevy::ui_render::extract_ui_camera_view),
            )
            .insert_resource(NativeFramePipeline::new(depth_shader, output_shader))
            .init_resource::<SpecializedRenderPipelines<NativeFramePipeline>>()
            .add_systems(
                Render,
                (
                    prepare_native_blit
                        .after(prepare_view_attachments)
                        .before(prepare_view_targets)
                        .in_set(RenderSystems::PrepareViews),
                    prepare_source_depth
                        .after(prepare_core_3d_depth_textures)
                        .in_set(RenderSystems::PrepareResources),
                    prepare_source_color.in_set(RenderSystems::PrepareResources),
                    prepare_frame_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .configure_sets(
                Core3d,
                NativeMainPass
                    .after(Core3dSystems::MainPass)
                    .before(Core3dSystems::EarlyPostProcess),
            )
            .add_systems(
                Core3d,
                (
                    (
                        render_native_opaque,
                        super::native::render_ordered::<
                            { super::native::NativeWorldPass::QuasiOpaque as u8 },
                        >,
                        super::native_sky::render_sky,
                        super::native::render_ordered::<
                            { super::native::NativeWorldPass::EmissiveOpaque as u8 },
                        >,
                        resolve_float_z,
                        snapshot_scene,
                        render_water,
                        resolve_water_float_z,
                        super::native::render_ordered::<
                            { super::native::NativeWorldPass::Translucent as u8 },
                        >,
                        render_native_transparent,
                        render_native_distortion,
                    )
                        .chain()
                        .in_set(NativeMainPass),
                    encode_scene
                        .after(Core3dSystems::PostProcess)
                        .after(upscaling),
                ),
            );
    }
}

fn source_float_z_sampler() -> SamplerDescriptor<'static> {
    // Native sampler byte0x61 (0043d110 /00432ba0): point, ClampU/V, WrapW,
    // bias0. Source floatZ has one mip, so valid WGPU LOD bounds0..0 retain all
    // reads despite the native descriptor's unbounded LOD range.
    SamplerDescriptor {
        label: Some("T6 floatZ code sampler 0x61"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::Repeat,
        mag_filter: FilterMode::Nearest,
        min_filter: FilterMode::Nearest,
        mipmap_filter: MipmapFilterMode::Nearest,
        lod_min_clamp: 0.0,
        lod_max_clamp: 0.0,
        compare: None,
        anisotropy_clamp: 1,
        border_color: None,
    }
}

pub(crate) fn maintain_float_z(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut cameras: Query<
        (
            Entity,
            &mut Camera,
            &Projection,
            &mut Tonemapping,
            Option<&mut NativeEffectView>,
            Option<&NativeSceneImage>,
        ),
        With<NativeRenderView>,
    >,
) {
    for (entity, mut camera, projection, mut tone, input, scene) in &mut cameras {
        // Acquire the window output normally. The render-world preparation
        // suppresses only the built-in final blit after attachment creation.
        if matches!(camera.output_mode, CameraOutputMode::Skip) {
            camera.output_mode = CameraOutputMode::default();
        }
        if *tone != Tonemapping::None {
            *tone = Tonemapping::None;
        }
        let Some(size) = camera
            .physical_target_size()
            .filter(|size| size.x > 0 && size.y > 0)
        else {
            continue;
        };
        let Projection::Perspective(projection) = projection else {
            panic!("native Origins scene requires the recovered perspective projection");
        };
        let near = projection.near / crate::assets::SOURCE_UNIT_METERS;
        let z_near = Vec4::new(near, 0.0, 0.0, 0.0);
        let render_target_size = Vec4::new(
            size.x as f32,
            size.y as f32,
            1.0 / size.x as f32,
            1.0 / size.y as f32,
        );
        let extent = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };
        if let Some(scene) = scene {
            let image = images
                .get(scene.0.id())
                .expect("owned native resolved scene Image");
            if image.texture_descriptor.size != extent {
                images
                    .get_mut(scene.0.id())
                    .expect("owned native resolved scene Image")
                    .resize(extent);
            }
        } else {
            let mut image = Image::new_uninit(
                extent,
                TextureDimension::D2,
                NATIVE_COLOR_FORMAT,
                RenderAssetUsages::default(),
            );
            image.texture_descriptor.label = Some("T6 resolved scene / resolvedPostSun alias");
            image.texture_descriptor.usage =
                TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
            commands
                .entity(entity)
                .insert(NativeSceneImage(images.add(image)));
        }
        if let Some(mut input) = input {
            let image = images
                .get(input.float_z.id())
                .expect("owned native floatZ Image");
            assert!(
                image.texture_descriptor.format == TextureFormat::R32Float
                    && image.texture_descriptor.mip_level_count == 1
                    && image.texture_descriptor.sample_count == 1,
                "native floatZ requires a one-level single-sample R32_FLOAT Image"
            );
            if image.texture_descriptor.size != extent {
                images
                    .get_mut(input.float_z.id())
                    .expect("owned native floatZ Image")
                    .resize(extent);
            }
            if input.z_near != z_near || input.render_target_size != render_target_size {
                input.z_near = z_near;
                input.render_target_size = render_target_size;
            }
        } else {
            let mut image = Image::new_uninit(
                extent,
                TextureDimension::D2,
                TextureFormat::R32Float,
                RenderAssetUsages::default(),
            );
            image.texture_descriptor.label = Some("native resolved floatZ");
            image.texture_descriptor.usage =
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
            let float_z = images.add(image);
            commands.entity(entity).insert(NativeEffectView {
                float_z,
                float_z_sampler: source_float_z_sampler(),
                z_near,
                render_target_size,
            });
        }
    }
}

// The native 3D shaders produce encoded RGB directly. CompositingSpace is
// restricted to Bevy's 2D path; set our format before UI derives its subview.
fn extract_native_target_format(
    cameras: Extract<
        Query<
            (RenderEntity, Has<bevy::anti_alias::dlss::Dlss>),
            Or<(With<NativeRenderView>, With<super::display::GameUiCamera>)>,
        >,
    >,
    mut commands: Commands,
    mut formats: ResMut<CameraMainPassTextureFormats>,
) {
    for (entity, dlss) in &cameras {
        let format = if dlss {
            TextureFormat::Rgba16Float
        } else {
            TextureFormat::Rgba8Unorm
        };
        if formats.0.contains_key(&entity) {
            formats.0.insert(entity, format);
            // Extraction defers camera insertion until the render world is ready.
            // Queue this after that insertion while publishing the UI format now.
            commands.queue(move |world: &mut World| {
                if let Some(mut view) = world.get_mut::<ExtractedView>(entity) {
                    view.target_format = format;
                }
            });
        }
    }
}

fn prepare_native_blit(mut views: Query<&mut ExtractedCamera, With<NativeRenderView>>) {
    for mut camera in &mut views {
        // prepare_windows and prepare_view_attachments require Write to acquire
        // the swapchain and output attachment. After those owners finish, Skip
        // suppresses Bevy's blit while our encoded output pass uses the attachment.
        camera.output_mode = CameraOutputMode::Skip;
    }
}

fn prepare_source_color(
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut cache: ResMut<TextureCache>,
    encoded: Option<Res<NativeEncodedScene>>,
    views: Query<
        (Entity, &ExtractedCamera, &Msaa, Option<&NativeSceneColor>),
        With<NativeRenderView>,
    >,
) {
    let Ok((entity, camera, msaa, previous)) = views.single() else {
        assert!(views.is_empty(), "Origins requires one native game view");
        commands.remove_resource::<NativeEncodedScene>();
        return;
    };
    assert_eq!(
        msaa.samples(),
        1,
        "Native source attachments require Msaa::Off"
    );
    let Some(size) = camera.physical_target_size else {
        return;
    };
    let descriptor = TextureDescriptor {
        label: Some("T6 UNORM8 scene accumulation"),
        size: size.to_extents(),
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: NATIVE_COLOR_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_SRC,
        view_formats: &[],
    };
    if previous.is_none_or(|previous| previous.size != size) {
        commands.entity(entity).insert(NativeSceneColor {
            image: cache.get(&device, descriptor.clone()),
            size,
        });
    }
    if encoded.as_ref().is_none_or(|encoded| encoded.size != size) {
        commands.insert_resource(NativeEncodedScene {
            image: cache.get(
                &device,
                TextureDescriptor {
                    label: Some("T6 encoded film input"),
                    ..descriptor
                },
            ),
            size,
        });
    }
}

pub(crate) fn render_native_opaque(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &NativeSceneColor,
        &ViewDepthStencilTexture,
        Option<&bevy::camera::MainPassResolutionOverride>,
    )>,
    opaque: Res<ViewBinnedRenderPhases<Opaque3d>>,
    masked: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    clear: Res<ClearColor>,
    mut ctx: RenderContext,
) {
    let entity = view.entity();
    let (camera, view, color, depth, resolution) = view.into_inner();
    let clear = match camera.clear_color {
        ClearColorConfig::Custom(color) => color,
        ClearColorConfig::None => Color::BLACK,
        ClearColorConfig::Default => clear.0,
    };
    let attachments = [Some(RenderPassColorAttachment {
        view: &color.image.default_view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(clear.to_linear().into()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 UNORM8 opaque scene"),
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
    if let Some(phase) = opaque.get(&view.retained_view_entity) {
        phase
            .render(&mut pass, world, entity)
            .expect("native opaque phase");
    }
    if let Some(phase) = masked.get(&view.retained_view_entity) {
        phase
            .render(&mut pass, world, entity)
            .expect("native alpha-mask phase");
    }
}

fn render_native_transparent(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &NativeSceneColor,
        &ViewDepthStencilTexture,
        Option<&bevy::camera::MainPassResolutionOverride>,
    )>,
    phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    mut ctx: RenderContext,
) {
    let entity = view.entity();
    let (camera, view, color, depth, resolution) = view.into_inner();
    let Some(phase) = phases
        .get(&view.retained_view_entity)
        .filter(|phase| !phase.items.is_empty())
    else {
        return;
    };
    let attachments = [Some(color.attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 UNORM8 transparent effects"),
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
    phase
        .render(&mut pass, world, entity)
        .expect("native transparent phase");
}

fn render_native_distortion(
    world: &World,
    view: ViewQuery<(
        &ExtractedCamera,
        &ExtractedView,
        &NativeSceneColor,
        &ViewDepthStencilTexture,
        Option<&bevy::camera::MainPassResolutionOverride>,
    )>,
    phases: Res<super::native_effect::DistortionPhases>,
    mut ctx: RenderContext,
) {
    let entity = view.entity();
    let (camera, view, color, depth, resolution) = view.into_inner();
    let Some(phase) = phases
        .0
        .get(&view.retained_view_entity)
        .filter(|phase| !phase.items.is_empty())
    else {
        return;
    };
    let attachments = [Some(color.attachment())];
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 source particle distortion"),
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
    phase
        .render(&mut pass, world, entity)
        .expect("Original distortion phase");
}

#[derive(Component)]
pub struct NativeDepthAttachment {
    size: Extent3d,
    samples: u32,
    attachment: DepthStencilAttachment,
}

fn prepare_source_depth(
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut cache: ResMut<TextureCache>,
    views: Query<
        (
            Entity,
            &ExtractedCamera,
            &Msaa,
            Option<&NativeDepthAttachment>,
        ),
        With<NativeRenderView>,
    >,
) {
    assert!(
        device
            .features()
            .contains(WgpuFeatures::DEPTH32FLOAT_STENCIL8),
        "native scene rendering requires DEPTH32FLOAT_STENCIL8"
    );
    for (entity, camera, msaa, previous) in &views {
        let Some(size) = camera.physical_target_size else {
            continue;
        };
        let size = Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        };
        if let Some(previous) =
            previous.filter(|previous| previous.size == size && previous.samples == msaa.samples())
        {
            // Retain views and bind groups across frames. The stock preparation
            // inserts its own depth component, so restore this native attachment.
            previous.attachment.prepare_for_new_frame();
            commands.entity(entity).insert(ViewDepthStencilTexture {
                attachment: previous.attachment.clone(),
            });
            continue;
        }
        let texture = cache.get(
            &device,
            TextureDescriptor {
                label: Some("native reverse-Z depth and stencil"),
                size,
                mip_level_count: 1,
                sample_count: msaa.samples(),
                dimension: TextureDimension::D2,
                format: NATIVE_DEPTH_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let depth = ViewDepthStencilTexture::new(texture, Some(0.0), Some(0));
        let attachment = depth.attachment.clone();
        commands.entity(entity).insert((
            depth,
            NativeDepthAttachment {
                size,
                samples: msaa.samples(),
                attachment,
            },
        ));
    }
}

#[derive(Resource)]
struct NativeFramePipeline {
    depth_shader: Handle<Shader>,
    output_shader: Handle<Shader>,
    depth_layouts: [BindGroupLayoutDescriptor; 2],
    output_layout: BindGroupLayoutDescriptor,
    fullscreen: Option<FullscreenShader>,
}

impl NativeFramePipeline {
    fn new(depth_shader: Handle<Shader>, output_shader: Handle<Shader>) -> Self {
        Self {
            depth_shader,
            output_shader,
            depth_layouts: [false, true].map(|multisampled| {
                BindGroupLayoutDescriptor::new(
                    "native depth resolve",
                    &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled,
                        },
                        count: None,
                    }],
                )
            }),
            output_layout: BindGroupLayoutDescriptor::new(
                "native encoded scene output",
                &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            ),
            fullscreen: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum NativeFrameKey {
    Depth { multisampled: bool },
    Encode { linear: bool },
}

impl SpecializedRenderPipeline for NativeFramePipeline {
    type Key = NativeFrameKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let (layout, shader, entry, format, shader_defs) = match key {
            NativeFrameKey::Depth { multisampled } => (
                &self.depth_layouts[usize::from(multisampled)],
                &self.depth_shader,
                "fragment",
                TextureFormat::R32Float,
                vec![ShaderDefVal::Bool("MULTISAMPLED".into(), multisampled)],
            ),
            NativeFrameKey::Encode { linear } => (
                &self.output_layout,
                &self.output_shader,
                if linear {
                    "fragment_reencode"
                } else {
                    "fragment"
                },
                NATIVE_COLOR_FORMAT,
                vec![],
            ),
        };
        RenderPipelineDescriptor {
            label: Some("native frame transfer".into()),
            layout: vec![layout.clone()],
            vertex: self
                .fullscreen
                .as_ref()
                .expect("native fullscreen pipeline initialized")
                .to_vertex_state(),
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some(entry.into()),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct PreparedNativeFrame {
    depth_source: TextureViewId,
    depth_target: TextureViewId,
    depth_bind_group: BindGroup,
    depth_pipeline: CachedRenderPipelineId,
    output_pipeline: CachedRenderPipelineId,
    output_groups: [(TextureViewId, BindGroup); 2],
    linear: bool,
}

fn prepare_frame_groups(
    mut commands: Commands,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    mut pipeline: ResMut<NativeFramePipeline>,
    mut variants: ResMut<SpecializedRenderPipelines<NativeFramePipeline>>,
    images: Res<RenderAssets<GpuImage>>,
    views: Query<
        (
            Entity,
            &NativeDepthImage,
            &ViewDepthStencilTexture,
            &ViewTarget,
            &NativeSceneColor,
            Option<&PreparedNativeFrame>,
            Has<bevy::anti_alias::dlss::Dlss>,
        ),
        With<NativeRenderView>,
    >,
) {
    if pipeline.fullscreen.is_none() {
        pipeline.fullscreen = Some(fullscreen.clone());
    }
    for (entity, float_z, depth, target, color, previous, dlss) in &views {
        let Some(image) = images.get(float_z.float_z) else {
            continue;
        };
        assert!(
            depth.texture().size() == image.texture_descriptor.size,
            "native floatZ must match the full-resolution source depth texture"
        );
        let depth_view = depth
            .attachment
            .depth_stencil_views()
            .depth_only_view()
            .expect("native scene depth aspect");
        let multisampled = depth.texture().sample_count() > 1;
        let depth_pipeline =
            variants.specialize(&cache, &pipeline, NativeFrameKey::Depth { multisampled });
        let output_pipeline =
            variants.specialize(&cache, &pipeline, NativeFrameKey::Encode { linear: dlss });
        let views = if dlss {
            [target.main_texture_view(), target.main_texture_other_view()]
        } else {
            [&color.image.default_view, &color.image.default_view]
        };
        if previous.is_some_and(|previous| {
            previous.depth_source == depth_view.id()
                && previous.depth_target == image.texture_view.id()
                && previous.depth_pipeline == depth_pipeline
                && previous.output_pipeline == output_pipeline
                && views.iter().all(|view| {
                    previous
                        .output_groups
                        .iter()
                        .any(|(id, _)| *id == view.id())
                })
        }) {
            continue;
        }
        let depth_bind_group = device.create_bind_group(
            "native floatZ resolve",
            &cache.get_bind_group_layout(&pipeline.depth_layouts[usize::from(multisampled)]),
            &BindGroupEntries::single(depth_view),
        );
        let output_groups = views.map(|view| {
            (
                view.id(),
                device.create_bind_group(
                    "native encoded scene",
                    &cache.get_bind_group_layout(&pipeline.output_layout),
                    &BindGroupEntries::single(view),
                ),
            )
        });
        commands.entity(entity).insert(PreparedNativeFrame {
            depth_source: depth_view.id(),
            depth_target: image.texture_view.id(),
            depth_bind_group,
            depth_pipeline,
            output_pipeline,
            output_groups,
            linear: dlss,
        });
    }
}

fn resolve_float_z(
    view: ViewQuery<(&NativeDepthImage, &PreparedNativeFrame), With<NativeRenderView>>,
    images: Res<RenderAssets<GpuImage>>,
    cache: Res<PipelineCache>,
    ctx: RenderContext,
) {
    let (float_z, prepared) = view.into_inner();
    resolve_depth(float_z, prepared, &images, &cache, ctx);
}

fn resolve_water_float_z(
    view: ViewQuery<(
        &NativeDepthImage,
        &PreparedNativeFrame,
        Option<&PreparedWaterView>,
    )>,
    images: Res<RenderAssets<GpuImage>>,
    cache: Res<PipelineCache>,
    ctx: RenderContext,
) {
    let (float_z, prepared, water) = view.into_inner();
    if water.is_some_and(PreparedWaterView::has_draws) {
        // Native00424ff0 resolves depth again only when post-resolve geometry
        // ran, making water's depth available to the subsequent FX pass.
        resolve_depth(float_z, prepared, &images, &cache, ctx);
    }
}

fn resolve_depth(
    float_z: &NativeDepthImage,
    prepared: &PreparedNativeFrame,
    images: &RenderAssets<GpuImage>,
    cache: &PipelineCache,
    mut ctx: RenderContext,
) {
    let (Some(image), Some(pipeline)) = (
        images.get(float_z.float_z),
        cache.get_render_pipeline(prepared.depth_pipeline),
    ) else {
        return;
    };
    let attachments = [Some(RenderPassColorAttachment {
        view: &image.texture_view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(Default::default()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("T6 reciprocal-depth resolve"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, &prepared.depth_bind_group, &[]);
    pass.draw(0..3, 0..1);
}

fn snapshot_scene(
    view: ViewQuery<(
        &NativeSceneColor,
        &NativeDepthImage,
        Option<&PreparedWaterView>,
    )>,
    images: Res<RenderAssets<GpuImage>>,
    mut ctx: RenderContext,
) {
    let (target, frame, water) = view.into_inner();
    if !water.is_some_and(PreparedWaterView::has_draws) {
        return;
    }
    let Some(resolved) = images.get(frame.resolved_scene) else {
        return;
    };
    let source = &target.image.texture;
    assert!(
        source.sample_count() == 1 && source.size() == resolved.texture_descriptor.size,
        "Original resolved scene must match the resolved main attachment"
    );
    // Native004549a0/00453ea0 alias logical code9 resolvedPostSun and code10
    // resolvedScene. 00468f50 updates that image before opaque water draws.
    ctx.command_encoder().copy_texture_to_texture(
        TexelCopyTextureInfo {
            texture: source,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        TexelCopyTextureInfo {
            texture: &resolved.texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        resolved.texture_descriptor.size,
    );
}

fn encode_scene(
    view: ViewQuery<(&ViewTarget, &NativeSceneColor, &PreparedNativeFrame), With<NativeRenderView>>,
    encoded: Option<Res<NativeEncodedScene>>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (target, color, prepared) = view.into_inner();
    let Some(encoded) = encoded else { return };
    let source_id = if prepared.linear {
        target.main_texture_view().id()
    } else {
        color.image.default_view.id()
    };
    let Some(pipeline) = cache.get_render_pipeline(prepared.output_pipeline) else {
        return;
    };
    let Some((_, bind_group)) = prepared
        .output_groups
        .iter()
        .find(|(id, _)| *id == source_id)
    else {
        return;
    };
    let attachments = [Some(RenderPassColorAttachment {
        view: &encoded.image.default_view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(default()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = ctx
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("T6 encoded film input"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
