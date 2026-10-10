use crate::{presentation::ViewModelCamera, rendering::GameCamera};
use bevy::{
    anti_alias::taa::{
        TemporalAntiAliasHistoryTextures, TemporalAntiAliasPipelineId, TemporalAntiAliasing,
        temporal_anti_alias,
    },
    asset::{embedded_asset, load_embedded_asset},
    camera::{NormalizedRenderTarget, Viewport},
    core_pipeline::{
        FullscreenShader,
        core_3d::{CORE_3D_DEPTH_FORMAT, prepare_prepass_textures},
        prepass::{MOTION_VECTOR_PREPASS_FORMAT, ViewPrepassTextures},
        schedule::{Core3d, Core3dSystems, camera_driver},
    },
    ecs::entity::EntityHashMap,
    prelude::*,
    render::{
        GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
        camera::ExtractedCamera,
        extract_component::ExtractComponentPlugin,
        render_resource::{
            BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
            CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction,
            DepthStencilState, Extent3d, FragmentState, PipelineCache, RenderPassDescriptor,
            RenderPipelineDescriptor, ShaderStages, StoreOp, TextureDescriptor, TextureDimension,
            TextureFormat, TextureId, TextureSampleType, TextureUsages,
            binding_types::{texture_2d, texture_depth_2d},
        },
        renderer::{RenderContext, RenderDevice, RenderGraph, ViewQuery},
        texture::{CachedTexture, ColorAttachment, DepthStencilAttachment},
        view::ViewTarget,
    },
};

pub(super) struct FirstPersonTemporalPlugin;

impl Plugin for FirstPersonTemporalPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "temporal.wgsl");
        app.add_plugins(ExtractComponentPlugin::<GameCamera>::default());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_gpu_resource::<TemporalTextures>()
                .add_systems(RenderStartup, init_pipeline)
                .add_systems(
                    Render,
                    (
                        reset_history
                            .after(RenderSystems::PrepareViews)
                            .before(RenderSystems::Prepare),
                        select_history_owner
                            .after(reset_history)
                            .before(RenderSystems::Prepare),
                        prepare_prepasses
                            .after(prepare_prepass_textures)
                            .in_set(RenderSystems::PrepareResources),
                        prepare_composition.in_set(RenderSystems::PrepareBindGroups),
                    ),
                )
                .add_systems(RenderGraph, select_resolve.before(camera_driver))
                .add_systems(
                    Core3d,
                    (
                        compose_inputs.before(temporal_anti_alias),
                        restore_inputs.after(temporal_anti_alias),
                    )
                        .in_set(Core3dSystems::EarlyPostProcess),
                );
        }
    }
}

#[derive(Resource)]
struct TemporalPipeline {
    layout: BindGroupLayoutDescriptor,
    id: CachedRenderPipelineId,
}

#[derive(Resource, Default)]
struct TemporalTextures(EntityHashMap<OwnedInputs>);

struct OwnedInputs {
    size: Extent3d,
    target: Option<NormalizedRenderTarget>,
    viewport: Option<Viewport>,
    model_depth: CachedTexture,
    model_motion: CachedTexture,
    combined_depth: CachedTexture,
    combined_motion: CachedTexture,
    history_valid: bool,
}

#[derive(Component)]
struct PreparedComposition {
    bind_group: BindGroup,
    depth: DepthStencilAttachment,
    motion: ColorAttachment,
    original: Option<OriginalInputs>,
}

struct OriginalInputs {
    depth: Option<DepthStencilAttachment>,
    motion: Option<ColorAttachment>,
    source: TextureId,
}

fn init_pipeline(
    mut commands: Commands,
    fullscreen: Res<FullscreenShader>,
    assets: Res<AssetServer>,
    cache: Res<PipelineCache>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "first_person_temporal_inputs",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_depth_2d(),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_depth_2d(),
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        ),
    );
    let id = cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some("first_person_temporal_inputs".into()),
        layout: vec![layout.clone()],
        vertex: fullscreen.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: load_embedded_asset!(assets.as_ref(), "temporal.wgsl"),
            targets: vec![Some(ColorTargetState {
                format: MOTION_VECTOR_PREPASS_FORMAT,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
            ..default()
        }),
        depth_stencil: Some(DepthStencilState {
            format: CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::Always),
            stencil: default(),
            bias: default(),
        }),
        ..default()
    });
    commands.insert_resource(TemporalPipeline { layout, id });
}

fn same_viewport(left: &Option<Viewport>, right: &Option<Viewport>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            left.physical_position == right.physical_position
                && left.physical_size == right.physical_size
                && left.depth == right.depth
        }
        _ => false,
    }
}

fn reset_history(
    textures: Res<TemporalTextures>,
    mut views: Query<(&ExtractedCamera, Entity, &mut TemporalAntiAliasing), With<ViewModelCamera>>,
    worlds: Query<
        (&ExtractedCamera, &TemporalAntiAliasing),
        (With<GameCamera>, Without<ViewModelCamera>),
    >,
) {
    for (camera, entity, mut taa) in &mut views {
        if textures.0.get(&entity).is_none_or(|inputs| {
            !inputs.history_valid
                || camera.physical_target_size
                    != Some(UVec2::new(inputs.size.width, inputs.size.height))
                || inputs.target != camera.target
                || !same_viewport(&inputs.viewport, &camera.viewport)
        }) || worlds
            .iter()
            .any(|(world, settings)| world.target == camera.target && settings.reset)
        {
            taa.reset = true;
        }
    }
}

fn select_history_owner(world: &mut World) {
    let targets: Vec<_> = world
        .query_filtered::<
            &ExtractedCamera,
            (With<ViewModelCamera>, With<TemporalAntiAliasing>),
        >()
        .iter(world)
        .map(|camera| camera.target.clone())
        .collect();
    let worlds: Vec<_> = world
        .query_filtered::<
            (Entity, &ExtractedCamera),
            (With<GameCamera>, Without<ViewModelCamera>),
        >()
        .iter(world)
        .filter(|(_, camera)| targets.contains(&camera.target))
        .map(|(entity, _)| entity)
        .collect();
    for entity in worlds {
        world.entity_mut(entity).remove::<(
            TemporalAntiAliasing,
            TemporalAntiAliasPipelineId,
            TemporalAntiAliasHistoryTextures,
        )>();
    }
}

fn new_texture(
    device: &RenderDevice,
    label: &'static str,
    size: Extent3d,
    format: TextureFormat,
    usage: TextureUsages,
) -> CachedTexture {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some(label),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    });
    let default_view = texture.create_view(&default());
    CachedTexture {
        texture,
        default_view,
    }
}

fn prepare_prepasses(
    device: Res<RenderDevice>,
    mut textures: ResMut<TemporalTextures>,
    mut views: Query<
        (Entity, &ExtractedCamera, &Msaa, &mut ViewPrepassTextures),
        (With<ViewModelCamera>, With<TemporalAntiAliasing>),
    >,
) {
    textures.0.retain(|entity, _| views.contains(*entity));
    for (entity, camera, msaa, mut prepass) in &mut views {
        if *msaa != Msaa::Off {
            continue;
        }
        let (Some(depth), Some(motion)) = (&prepass.depth, &prepass.motion_vectors) else {
            continue;
        };
        let depth_usage = depth.texture.texture.usage();
        let motion_usage = motion.texture.texture.usage();
        let size = prepass.size;
        let compatible = textures.0.get(&entity).is_some_and(|inputs| {
            inputs.size == size
                && inputs.model_depth.texture.usage() == depth_usage
                && inputs.model_motion.texture.usage() == motion_usage
        });
        if !compatible {
            let usage = TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING;
            textures.0.insert(
                entity,
                OwnedInputs {
                    size,
                    target: camera.target.clone(),
                    viewport: camera.viewport.clone(),
                    model_depth: new_texture(
                        &device,
                        "first_person_prepass_depth",
                        size,
                        CORE_3D_DEPTH_FORMAT,
                        depth_usage,
                    ),
                    model_motion: new_texture(
                        &device,
                        "first_person_prepass_motion",
                        size,
                        MOTION_VECTOR_PREPASS_FORMAT,
                        motion_usage,
                    ),
                    combined_depth: new_texture(
                        &device,
                        "first_person_combined_depth",
                        size,
                        CORE_3D_DEPTH_FORMAT,
                        usage,
                    ),
                    combined_motion: new_texture(
                        &device,
                        "first_person_combined_motion",
                        size,
                        MOTION_VECTOR_PREPASS_FORMAT,
                        usage,
                    ),
                    history_valid: false,
                },
            );
        }
        let inputs = textures.0.get_mut(&entity).unwrap();
        inputs.target = camera.target.clone();
        inputs.viewport = camera.viewport.clone();
        prepass.depth = Some(DepthStencilAttachment::new(
            inputs.model_depth.clone(),
            None,
            Some(0.0),
            None,
        ));
        prepass.motion_vectors = Some(ColorAttachment::new(
            inputs.model_motion.clone(),
            None,
            None,
            Some(LinearRgba::BLACK.into()),
        ));
    }
}

fn prepare_composition(
    mut commands: Commands,
    device: Res<RenderDevice>,
    pipeline: Res<TemporalPipeline>,
    cache: Res<PipelineCache>,
    textures: Res<TemporalTextures>,
    previous: Query<Entity, With<PreparedComposition>>,
    models: Query<
        (Entity, &ExtractedCamera, &ViewPrepassTextures, &ViewTarget, &Msaa),
        (With<ViewModelCamera>, With<TemporalAntiAliasing>),
    >,
    worlds: Query<
        (&ExtractedCamera, &ViewPrepassTextures, &ViewTarget, &Msaa),
        (With<GameCamera>, Without<ViewModelCamera>),
    >,
) {
    for entity in &previous {
        commands.entity(entity).remove::<PreparedComposition>();
    }
    for (entity, camera, prepass, target, msaa) in &models {
        let Some(inputs) = textures.0.get(&entity) else {
            continue;
        };
        if *msaa != Msaa::Off {
            continue;
        }
        let Some((_, world_prepass, _, _)) = worlds.iter().find(
            |(world, world_prepass, world_target, world_msaa)| {
                world.target == camera.target
                    && world.order < camera.order
                    && world.physical_target_size == camera.physical_target_size
                    && same_viewport(&world.viewport, &camera.viewport)
                    && world_prepass.size == prepass.size
                    && world_target.main_texture().id() == target.main_texture().id()
                    && **world_msaa == Msaa::Off
            },
        ) else {
            continue;
        };
        let (Some(world_depth), Some(world_motion), Some(model_depth), Some(model_motion)) = (
            &world_prepass.depth,
            &world_prepass.motion_vectors,
            &prepass.depth,
            &prepass.motion_vectors,
        ) else {
            continue;
        };
        if model_depth.texture.texture.id() != inputs.model_depth.texture.id()
            || model_motion.texture.texture.id() != inputs.model_motion.texture.id()
        {
            continue;
        }
        let bind_group = device.create_bind_group(
            "first_person_temporal_inputs",
            &cache.get_bind_group_layout(&pipeline.layout),
            &BindGroupEntries::sequential((
                &world_depth.texture.default_view,
                &world_motion.texture.default_view,
                &model_depth.texture.default_view,
                &model_motion.texture.default_view,
            )),
        );
        commands.entity(entity).insert(PreparedComposition {
            bind_group,
            depth: DepthStencilAttachment::new(
                inputs.combined_depth.clone(),
                None,
                Some(0.0),
                None,
            ),
            motion: ColorAttachment::new(
                inputs.combined_motion.clone(),
                None,
                None,
                Some(LinearRgba::BLACK.into()),
            ),
            original: None,
        });
    }
}

fn select_resolve(world: &mut World) {
    let ready = {
        let pipeline = world.resource::<TemporalPipeline>();
        world
            .resource::<PipelineCache>()
            .get_render_pipeline(pipeline.id)
            .is_some()
    };
    let models: Vec<_> = world
        .query_filtered::<
            (Entity, Has<PreparedComposition>),
            (With<ViewModelCamera>, With<TemporalAntiAliasing>),
        >()
        .iter(world)
        .collect();
    for (model, prepared) in models {
        if !ready || !prepared {
            world.entity_mut(model).remove::<TemporalAntiAliasPipelineId>();
            if let Some(inputs) = world.resource_mut::<TemporalTextures>().0.get_mut(&model) {
                inputs.history_valid = false;
            }
        }
    }
}

fn compose_inputs(
    view: ViewQuery<
        (&mut ViewPrepassTextures, &mut PreparedComposition, &ViewTarget),
        (With<ViewModelCamera>, With<TemporalAntiAliasPipelineId>),
    >,
    pipeline: Res<TemporalPipeline>,
    cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (mut prepass, mut prepared, target) = view.into_inner();
    let Some(pipeline) = cache.get_render_pipeline(pipeline.id) else {
        return;
    };
    {
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("first_person_temporal_inputs"),
            color_attachments: &[Some(prepared.motion.get_attachment())],
            depth_stencil_attachment: Some(prepared.depth.get_attachment(StoreOp::Store)),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(0, &prepared.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
    prepared.original = Some(OriginalInputs {
        depth: prepass.depth.replace(prepared.depth.clone()),
        motion: prepass.motion_vectors.replace(prepared.motion.clone()),
        source: target.main_texture().id(),
    });
}

fn restore_inputs(
    view: ViewQuery<
        (&mut ViewPrepassTextures, &mut PreparedComposition, &ViewTarget),
        With<ViewModelCamera>,
    >,
    mut textures: ResMut<TemporalTextures>,
) {
    let entity = view.entity();
    let (mut prepass, mut prepared, target) = view.into_inner();
    if let Some(original) = prepared.original.take() {
        prepass.depth = original.depth;
        prepass.motion_vectors = original.motion;
        if let Some(inputs) = textures.0.get_mut(&entity) {
            inputs.history_valid = target.main_texture().id() != original.source;
        }
    }
}
