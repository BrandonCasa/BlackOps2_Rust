//! Full-resolution source film/UI composition and final native display ramp.
//! Bevy owns the camera, its RGBA8 ViewTarget, UI lifecycle and output attachment.
use super::{
    frame::NativeEncodedScene,
    native_film::{NativeFilmPipeline, NativeFilmSettings, NativeFilmView},
    native_vision::NativeVision,
};
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    camera::{CameraOutputMode, visibility::RenderLayers},
    core_pipeline::{
        Core2dSystems, FullscreenShader,
        schedule::Core2d,
        tonemapping::{DebandDither, Tonemapping},
        upscaling::upscaling,
    },
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        camera::ExtractedCamera,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_asset::RenderAssets,
        render_resource::{
            binding_types::{texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        texture::GpuImage,
        view::{ViewTarget, prepare_view_attachments, prepare_view_targets},
    },
    ui_render::ui_pass,
};

/// Ordinary HUD/menu roots target this camera; 3D projection retains GameCamera.
#[derive(Component, Clone, Copy, ExtractComponent)]
#[extract_app(RenderApp)]
pub(crate) struct GameUiCamera;

/// Native renderer initialization uses .9; the recovered brightness setting is
/// clamped to [.5,1.5]. This changes our final output only, never desktop gamma.
#[derive(Resource)]
pub(crate) struct NativeDisplaySettings {
    pub brightness: f32,
}
impl Default for NativeDisplaySettings {
    fn default() -> Self {
        Self { brightness: 0.9 }
    }
}
fn source_brightness_offset(brightness: f32) -> Vec4 {
    assert!(
        brightness.is_finite(),
        "native display brightness must be finite"
    );
    Vec4::new(
        (brightness.clamp(0.5, 1.5) - 1.0) * 32.0 * (1.0 / 255.0),
        0.0,
        0.0,
        0.0,
    )
}

pub(crate) struct NativeDisplayPlugin;
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NativeDisplaySystems {
    Prepare,
}
impl Plugin for NativeDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NativeDisplaySettings>()
            .add_plugins(ExtractComponentPlugin::<GameUiCamera>::default())
            .add_systems(Startup, spawn_ui_camera);
        embedded_asset!(app, "display.wesl");
        let shader = load_embedded_asset!(app, "display.wesl");
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render
            .init_resource::<DisplayRequest>()
            .insert_resource(DisplayPipeline::new(shader))
            .init_resource::<SpecializedRenderPipelines<DisplayPipeline>>()
            .add_systems(ExtractSchedule, extract_display)
            .add_systems(
                Render,
                (
                    suppress_ui_blit
                        .after(prepare_view_attachments)
                        .before(prepare_view_targets)
                        .in_set(RenderSystems::PrepareViews),
                    prepare_display
                        .in_set(RenderSystems::PrepareBindGroups)
                        .in_set(NativeDisplaySystems::Prepare),
                ),
            )
            .add_systems(
                Core2d,
                (
                    render_film
                        .after(Core2dSystems::PostProcess)
                        .before(ui_pass),
                    render_display.after(ui_pass).after(upscaling),
                ),
            );
    }
}
fn spawn_ui_camera(mut commands: Commands) {
    commands.spawn((
        Name::new("Native film and UI camera"),
        GameUiCamera,
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        CompositingSpace::Linear,
        Tonemapping::None,
        DebandDither::Disabled,
        // World sprites/meshes do not belong to this ordinary UI command stream.
        RenderLayers::none(),
    ));
}

#[derive(Resource, Default)]
struct DisplayRequest {
    vision: Option<(NativeFilmSettings, AssetId<Image>)>,
    brightness_offset: Vec4,
}
fn extract_display(
    source: Extract<Option<Res<NativeVision>>>,
    settings: Extract<Res<NativeDisplaySettings>>,
    mut request: ResMut<DisplayRequest>,
) {
    request.vision = source
        .as_ref()
        .map(|vision| (vision.settings, vision.lut.id()));
    request.brightness_offset = source_brightness_offset(settings.brightness);
}
fn suppress_ui_blit(mut views: Query<&mut ExtractedCamera, With<GameUiCamera>>) {
    for mut camera in &mut views {
        // Keep MainWorld CameraOutputMode::Write so swapchain acquisition and
        // prepare_view_attachments run normally; suppress only the later blit.
        camera.output_mode = CameraOutputMode::Skip;
    }
}

#[derive(Resource)]
struct DisplayPipeline {
    shader: Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
    fullscreen: Option<FullscreenShader>,
    controls: UniformBuffer<Vec4>,
}
impl DisplayPipeline {
    fn new(shader: Handle<Shader>) -> Self {
        Self {
            shader,
            layout: BindGroupLayoutDescriptor::new(
                "native composed display input",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::FRAGMENT,
                    (
                        texture_2d(TextureSampleType::Float { filterable: false }),
                        uniform_buffer::<Vec4>(false),
                    ),
                ),
            ),
            fullscreen: None,
            controls: UniformBuffer::from(source_brightness_offset(0.9)),
        }
    }
}
impl SpecializedRenderPipeline for DisplayPipeline {
    type Key = TextureFormat;
    fn specialize(&self, format: TextureFormat) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("native source display ramp".into()),
            layout: vec![self.layout.clone()],
            vertex: self
                .fullscreen
                .as_ref()
                .expect("native display fullscreen initialized")
                .to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                entry_point: Some(
                    if format.is_srgb() {
                        "fragment_srgb"
                    } else {
                        "fragment_unorm"
                    }
                    .into(),
                ),
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
struct PreparedDisplay {
    film: NativeFilmView,
    output_pipeline: CachedRenderPipelineId,
    output_groups: [(TextureViewId, BindGroup); 2],
    controls_buffer: BufferId,
    film_complete: bool,
}

/// Root's NativeEncodedScene preparation precedes this ordinary bind-group set.
/// Missing source assets/scene/pipelines follow loading readiness, with no LUT
/// replacement or ungraded scene presented as a completed native output.
fn prepare_display(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    fullscreen: Res<FullscreenShader>,
    request: Res<DisplayRequest>,
    encoded: Option<Res<NativeEncodedScene>>,
    images: Res<RenderAssets<GpuImage>>,
    film_pipeline: Res<NativeFilmPipeline>,
    mut pipeline: ResMut<DisplayPipeline>,
    mut variants: ResMut<SpecializedRenderPipelines<DisplayPipeline>>,
    mut views: Query<(Entity, &ViewTarget, Option<&mut PreparedDisplay>), With<GameUiCamera>>,
) {
    if pipeline.fullscreen.is_none() {
        pipeline.fullscreen = Some(fullscreen.clone());
    }
    if pipeline.controls.buffer().is_none() || *pipeline.controls.get() != request.brightness_offset
    {
        pipeline.controls.set(request.brightness_offset);
        pipeline.controls.write_buffer(&device, &queue);
    }
    for (entity, target, previous) in &mut views {
        let Some((settings, lut)) = request.vision else {
            if previous.is_some() {
                commands.entity(entity).remove::<PreparedDisplay>();
            }
            continue;
        };
        let (Some(encoded), Some(authored), Some(format)) = (
            encoded.as_ref(),
            images.get(lut),
            target.out_texture_view_format(),
        ) else {
            if previous.is_some() {
                commands.entity(entity).remove::<PreparedDisplay>();
            }
            continue;
        };
        assert_eq!(
            target.main_texture_format(),
            TextureFormat::Rgba8Unorm,
            "native UI composition requires root's RGBA8_UNORM camera format hook"
        );
        let extent = target.main_texture().size();
        if encoded.size != UVec2::new(extent.width, extent.height) {
            // Camera/scene size ownership is settling during an ordinary resize.
            if previous.is_some() {
                commands.entity(entity).remove::<PreparedDisplay>();
            }
            continue;
        }
        let output_pipeline = variants.specialize(&cache, &pipeline, format);
        let views = [target.main_texture_view(), target.main_texture_other_view()];
        let buffer = pipeline
            .controls
            .buffer()
            .expect("uploaded native display controls")
            .id();
        if let Some(mut previous) = previous {
            previous.film_complete = false;
            previous
                .film
                .update(
                    &device,
                    &queue,
                    &film_pipeline,
                    &encoded.image.default_view,
                    encoded.size,
                    authored,
                    settings,
                )
                .unwrap_or_else(|error| panic!("required native film input: {error}"));
            if previous.output_pipeline == output_pipeline
                && previous.controls_buffer == buffer
                && views.iter().all(|view| {
                    previous
                        .output_groups
                        .iter()
                        .any(|(id, _)| *id == view.id())
                })
            {
                continue;
            }
            previous.output_pipeline = output_pipeline;
            previous.controls_buffer = buffer;
            previous.output_groups = output_groups(&device, &cache, &pipeline, views);
        } else {
            let film = NativeFilmView::new(
                &device,
                &queue,
                &film_pipeline,
                &encoded.image.default_view,
                encoded.size,
                authored,
                settings,
            )
            .unwrap_or_else(|error| panic!("required native film input: {error}"));
            commands.entity(entity).insert(PreparedDisplay {
                film,
                output_pipeline,
                output_groups: output_groups(&device, &cache, &pipeline, views),
                controls_buffer: buffer,
                film_complete: false,
            });
        }
    }
}
fn output_groups(
    device: &RenderDevice,
    cache: &PipelineCache,
    pipeline: &DisplayPipeline,
    views: [&TextureView; 2],
) -> [(TextureViewId, BindGroup); 2] {
    views.map(|view| {
        (
            view.id(),
            device.create_bind_group(
                "native composed UI display",
                &cache.get_bind_group_layout(&pipeline.layout),
                &BindGroupEntries::sequential((view, pipeline.controls.binding().unwrap())),
            ),
        )
    })
}

fn render_film(
    view: ViewQuery<(&ViewTarget, &mut PreparedDisplay), With<GameUiCamera>>,
    pipeline: Res<NativeFilmPipeline>,
    cache: Res<PipelineCache>,
    mut context: RenderContext,
) {
    let (target, mut prepared) = view.into_inner();
    // No ViewTarget swap is required: source film completely overwrites the
    // current RGBA8 target, which the following ordinary UI pass then loads.
    prepared.film_complete =
        prepared
            .film
            .render(&mut context, &pipeline, &cache, target.main_texture_view());
}
fn render_display(
    view: ViewQuery<(&ViewTarget, &PreparedDisplay), With<GameUiCamera>>,
    cache: Res<PipelineCache>,
    mut context: RenderContext,
) {
    let (target, prepared) = view.into_inner();
    if !prepared.film_complete {
        return;
    }
    let Some(pipeline) = cache.get_render_pipeline(prepared.output_pipeline) else {
        return;
    };
    let Some((_, group)) = prepared
        .output_groups
        .iter()
        .find(|(id, _)| *id == target.main_texture_view().id())
    else {
        return;
    };
    let Some(attachment) = target.out_texture_color_attachment(None) else {
        return;
    };
    let attachments = [Some(attachment)];
    let mut pass = context
        .command_encoder()
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("native scene and UI display ramp"),
            color_attachments: &attachments,
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_brightness_uses_native_initial_value_range_and_offset() {
        assert_eq!(NativeDisplaySettings::default().brightness, 0.9);
        assert_eq!(source_brightness_offset(1.), Vec4::ZERO);
        assert_eq!(
            source_brightness_offset(-10.),
            source_brightness_offset(0.5)
        );
        assert_eq!(source_brightness_offset(10.), source_brightness_offset(1.5));
        let offset = source_brightness_offset(0.9).x;
        assert!((offset - -0.012549022).abs() < 0.00000001);
        // Source gamma changes complete encoded pixels, including ordinary UI.
        let u = (0.5 + offset).clamp(0., 1.);
        let linear = u * u;
        assert!((linear - 0.23760848).abs() < 0.0000001);
        let displayed = 1.055 * linear.powf(1. / 2.4) - 0.055;
        assert!((displayed - 0.5246888).abs() < 0.000001);
    }
}
