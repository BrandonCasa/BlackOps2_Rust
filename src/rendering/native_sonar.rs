//! MMS actor draw list and the original half-resolution color/depth attachments.
use super::{
    animated_models::{AnimatedModel, AnimatedModelInstance},
    frame::{NATIVE_DEPTH_FORMAT, NativeRenderView},
    native::{NativeWorldScene, NativeWorldState},
    native_model::{ModelGeometry, NativeModelSurface},
    skinning::{PreparedAnimatedModel, PreparedSkin},
};
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{Core3dSystems, core_3d::main_opaque_pass_3d, schedule::Core3d},
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    mesh::VertexBufferLayout,
    platform::collections::HashMap,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_resource::{
            binding_types::{sampler, texture_2d, uniform_buffer},
            *,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
        sync_world::RenderEntity,
        texture::GpuImage,
        view::ViewTarget,
    },
    shader::load_shader_library,
    transform::TransformSystems,
};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SonarAttachment {
    pub mms_weapon: bool,
    pub mms_in_scope: bool,
    #[serde(rename = "mmsFOV")]
    pub mms_fov: f32,
    pub mms_aspect: f32,
    pub mms_max_dist: f32,
}
#[derive(Resource, Deserialize)]
pub(crate) struct SonarAttachmentSources(pub BTreeMap<String, SonarAttachment>);

/// Source renderflag0x1000000 owner. Used only to keep its skin/LOD available
/// for the independent sonar draw list when ordinary world DPVS rejects it.
#[derive(Component)]
pub(crate) struct NativeSonarActor {
    pub owner: Entity,
    pub active: bool,
}
#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct NativeSonarView {
    pub viewport: Vec4,
    pub thermal: bool,
}
#[derive(Clone, Copy, Default)]
struct Contact {
    script7: Vec4,
    intensity: f32,
}
struct MotionHistory {
    previous: Vec3,
    ratios: [f32; 60],
    cursor: usize,
    contact: Contact,
}
impl Default for MotionHistory {
    fn default() -> Self {
        Self {
            previous: Vec3::ZERO,
            ratios: [0.; 60],
            cursor: 0,
            contact: Contact::default(),
        }
    }
}
impl MotionHistory {
    fn sample(&mut self, eye: Vec3, position: Vec3, max_distance: f32) -> Contact {
        // 004bd0d0: source inches per client frame, not velocity per second.
        let distance = (eye.distance(position) / max_distance).clamp(0., 1.);
        let threshold = 2.200000047683716 + (1. - distance) * 3.;
        self.ratios[self.cursor] = position.distance(self.previous) / threshold;
        self.previous = position;
        // cg_sonarAttachmentSpeedDelay=.8; original multiplies by literal60.
        let mut sum = 0.;
        let mut cursor = self.cursor;
        for _ in 0..48 {
            sum += self.ratios[cursor];
            cursor = if cursor == 0 { 59 } else { cursor - 1 };
        }
        self.cursor = (self.cursor + 1) % 60;
        let intensity = (1. - sum / 48.).clamp(0., 1.);
        // Raw004bd401..41a proves the actual stack order for scriptVector7.
        self.contact = Contact {
            script7: Vec4::new(0., 0., intensity, 1. - distance),
            intensity,
        };
        self.contact
    }
}
#[derive(Resource, Default)]
struct Contacts(HashMap<Entity, MotionHistory>);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SonarProgram {
    Overlay,
    Thermal,
    ThermalChar,
    ThermalSolid,
}
#[derive(Asset, TypePath, Clone)]
pub(crate) struct SonarMaterial {
    pub program: SonarProgram,
    pub constant: Vec4,
    pub textures: Vec<(Handle<Image>, SamplerDescriptor<'static>)>,
    pub state: NativeWorldState,
}
impl SonarMaterial {
    fn solid() -> Self {
        // Actual code material mc/infrared_white, VSf24ed319 / PS9e710634.
        Self {
            program: SonarProgram::ThermalSolid,
            constant: Vec4::new(
                0.20000000298023224,
                0.4000000059604645,
                0.20000000298023224,
                1.,
            ),
            textures: Vec::new(),
            state: thermal_state(),
        }
    }
    fn overlay() -> Self {
        Self {
            program: SonarProgram::Overlay,
            constant: Vec4::new(0.009999999776482582, 0., 0., 0.),
            textures: Vec::new(),
            state: NativeWorldState {
                blend: Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::One,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Max,
                    },
                    alpha: BlendComponent {
                        src_factor: BlendFactor::OneMinusDstAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                }),
                depth_compare: CompareFunction::Always,
                depth_write: false,
                ..thermal_state()
            },
        }
    }
}
fn thermal_state() -> NativeWorldState {
    NativeWorldState {
        blend: Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Max,
            },
        }),
        alpha_test: false,
        depth_compare: CompareFunction::GreaterEqual,
        depth_write: true,
        polygon_offset: 0,
        cull_mode: Some(Face::Back),
        front_face: FrontFace::Cw,
        color_write: ColorWrites::ALL,
    }
}
#[derive(Component)]
struct SonarSurface {
    actor: Entity,
    thermal: Handle<SonarMaterial>,
    overlay: Handle<SonarMaterial>,
}

#[derive(Clone, Copy, ShaderType)]
struct SonarDraw {
    world_rows: [Vec4; 4],
    script7: Vec4,
    weapon0: Vec4,
    material: Vec4,
}
struct ExtractedDraw {
    model: AssetId<AnimatedModel>,
    instance: Entity,
    lod: usize,
    surface: usize,
    first_index: u32,
    index_count: u32,
    world_rows: [Vec4; 4],
    contact: Contact,
    thermal: AssetId<SonarMaterial>,
    overlay: AssetId<SonarMaterial>,
    sort: (u64, u32),
}
#[derive(Resource, Default)]
struct SonarRequest {
    views: HashMap<Entity, NativeSonarView>,
    draws: Vec<ExtractedDraw>,
}
pub(crate) struct PreparedSonarMaterial {
    program: SonarProgram,
    constant: Vec4,
    state: NativeWorldState,
    group: Option<BindGroup>,
}
impl RenderAsset for PreparedSonarMaterial {
    type SourceAsset = SonarMaterial;
    type Param = (
        SRes<RenderDevice>,
        SRes<PipelineCache>,
        SRes<SonarPipeline>,
        SRes<RenderAssets<GpuImage>>,
    );
    fn prepare_asset(
        source: SonarMaterial,
        _: AssetId<SonarMaterial>,
        (device, cache, pipeline, images): &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<SonarMaterial>> {
        let Some(gpu): Option<Vec<_>> = source
            .textures
            .iter()
            .map(|(image, _)| images.get(image.id()))
            .collect()
        else {
            return Err(PrepareAssetError::RetryNextUpdate(source));
        };
        let samplers: Vec<_> = source
            .textures
            .iter()
            .map(|(_, desc)| device.create_sampler(desc))
            .collect();
        let mut bindings = Vec::new();
        for (index, (image, sampler)) in gpu.iter().zip(&samplers).enumerate() {
            assert!(
                !image.texture_descriptor.format.is_srgb(),
                "MMS thermal source textures require original UNORM samples"
            );
            bindings.push(BindGroupEntry {
                binding: (index * 2) as u32,
                resource: BindingResource::TextureView(&image.texture_view),
            });
            bindings.push(BindGroupEntry {
                binding: (index * 2 + 1) as u32,
                resource: BindingResource::Sampler(sampler),
            });
        }
        let group = (!bindings.is_empty()).then(|| {
            device.create_bind_group(
                "T6 MMS authored thermal inputs",
                &cache.get_bind_group_layout(&pipeline.textures),
                &bindings,
            )
        });
        Ok(Self {
            program: source.program,
            constant: source.constant,
            state: source.state,
            group,
        })
    }
}

#[derive(Resource)]
pub(crate) struct SonarPipeline {
    shaders: [Handle<Shader>; 4],
    scene: BindGroupLayoutDescriptor,
    draw: BindGroupLayoutDescriptor,
    textures: BindGroupLayoutDescriptor,
}
impl SonarPipeline {
    fn new(shaders: [Handle<Shader>; 4]) -> Self {
        let scene = BindGroupLayoutDescriptor::new(
            "MMS source scene",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<NativeWorldScene>(false),
            ),
        );
        let draw = BindGroupLayoutDescriptor::new(
            "MMS source draw",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<SonarDraw>(true),
            ),
        );
        let textures = BindGroupLayoutDescriptor::new(
            "MMS thermal source maps",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        Self {
            shaders,
            scene,
            draw,
            textures,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct SonarKey {
    program: SonarProgram,
    state: NativeWorldState,
}
impl SpecializedRenderPipeline for SonarPipeline {
    type Key = SonarKey;
    fn specialize(&self, key: SonarKey) -> RenderPipelineDescriptor {
        let shader = &self.shaders[match key.program {
            SonarProgram::Overlay => 0,
            SonarProgram::Thermal => 1,
            SonarProgram::ThermalChar => 2,
            SonarProgram::ThermalSolid => 3,
        }];
        let mut layout = vec![self.scene.clone(), self.draw.clone()];
        if matches!(
            key.program,
            SonarProgram::Thermal | SonarProgram::ThermalChar
        ) {
            layout.push(self.textures.clone());
        }
        RenderPipelineDescriptor {
            label: Some("T6 MMS actual actor/thermal draw".into()),
            layout,
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![VertexBufferLayout {
                    array_stride: 32,
                    step_mode: VertexStepMode::Vertex,
                    attributes: vec![
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: 0,
                            shader_location: 0,
                        },
                        VertexAttribute {
                            format: VertexFormat::Float16x2,
                            offset: 20,
                            shader_location: 2,
                        },
                        VertexAttribute {
                            format: VertexFormat::Unorm10_10_10_2,
                            offset: 24,
                            shader_location: 3,
                        },
                    ],
                }],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: key.state.blend,
                    write_mask: key.state.color_write,
                })],
                ..default()
            }),
            primitive: PrimitiveState {
                cull_mode: key.state.cull_mode,
                front_face: key.state.front_face,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: NATIVE_DEPTH_FORMAT,
                depth_write_enabled: Some(key.state.depth_write),
                depth_compare: Some(key.state.depth_compare),
                bias: key.state.depth_bias(),
                stencil: default(),
            }),
            ..default()
        }
    }
}
struct PreparedDraw {
    model: AssetId<AnimatedModel>,
    instance: Entity,
    lod: usize,
    surface: usize,
    first_index: u32,
    index_count: u32,
    material: AssetId<SonarMaterial>,
    pipeline: CachedRenderPipelineId,
    uniform_offset: u32,
}
/// Direct SRVs of RT0x39's real RGBA8 color and depth attachment. The depth view
/// uses DepthOnly, sampled as texture_depth_2d with the ordinary source sampler.
#[derive(Component)]
pub(crate) struct PreparedSonarView {
    _color: Texture,
    _depth: Texture,
    attachment: TextureView,
    pub color: TextureView,
    pub depth: TextureView,
    pub sampler: Sampler,
    pub viewport: Vec4,
    size: UVec2,
    scene: UniformBuffer<NativeWorldScene>,
    scene_group: BindGroup,
    draws: Vec<PreparedDraw>,
    draw_uniforms: DynamicUniformBuffer<SonarDraw>,
    draw_group: Option<(BufferId, BindGroup)>,
}

pub(crate) struct NativeSonarPlugin;
impl Plugin for NativeSonarPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SonarMaterial>()
            .init_resource::<Contacts>()
            .add_plugins(RenderAssetPlugin::<PreparedSonarMaterial>::default())
            .add_systems(Startup, load_sources)
            .add_systems(
                PostUpdate,
                (attach_actors, update_contacts)
                    .chain()
                    .after(TransformSystems::Propagate)
                    .before(super::animated_models::select_lods),
            )
            .add_systems(
                PostUpdate,
                bind_surfaces.after(super::animated_models::bind_animated_draws),
            );
        load_shader_library!(app, "native_sonar_common.wesl");
        embedded_asset!(app, "native_sonar_overlay.wesl");
        embedded_asset!(app, "native_sonar_thermal.wesl");
        embedded_asset!(app, "native_sonar_thermal_char.wesl");
        embedded_asset!(app, "native_sonar_thermal_solid.wesl");
        let pipeline = SonarPipeline::new([
            load_embedded_asset!(app, "native_sonar_overlay.wesl"),
            load_embedded_asset!(app, "native_sonar_thermal.wesl"),
            load_embedded_asset!(app, "native_sonar_thermal_char.wesl"),
            load_embedded_asset!(app, "native_sonar_thermal_solid.wesl"),
        ]);
        let Some(render) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render
            .insert_resource(pipeline)
            .init_resource::<SonarRequest>()
            .init_resource::<SpecializedRenderPipelines<SonarPipeline>>()
            .add_systems(ExtractSchedule, extract_sonar)
            .add_systems(
                Render,
                prepare_sonar_views.in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(
                Core3d,
                render_sonar
                    .before(main_opaque_pass_3d)
                    .in_set(Core3dSystems::MainPass),
            );
    }
}
fn load_sources(mut commands: Commands, options: Res<crate::assets::LaunchOptions>) {
    let sources: SonarAttachmentSources =
        crate::origins::read(&options.asset_root.join("source-attachments.json"));
    let mms = sources
        .0
        .get("mms")
        .expect("Export original MMS attachment");
    assert!(
        mms.mms_weapon && mms.mms_in_scope && mms.mms_max_dist > 0. && mms.mms_aspect > 0.,
        "Original MMS attachment fields required"
    );
    commands.insert_resource(sources);
}
fn attach_actors(
    mut commands: Commands,
    instances: Query<Entity, (With<AnimatedModelInstance>, Without<NativeSonarActor>)>,
    parents: Query<&ChildOf>,
    actors: Query<(), With<crate::enemies::Zombie>>,
) {
    for entity in &instances {
        if let Some(owner) = std::iter::once(entity)
            .chain(parents.iter_ancestors(entity))
            .find(|owner| actors.contains(*owner))
        {
            commands.entity(entity).insert(NativeSonarActor {
                owner,
                active: false,
            });
        }
    }
}
fn update_contacts(
    mut commands: Commands,
    sources: Res<SonarAttachmentSources>,
    inventories: Query<&crate::weapons::WeaponInventory, With<crate::player::Player>>,
    cameras: Query<(Entity, &GlobalTransform, Option<&NativeSonarView>), With<NativeRenderView>>,
    actors: Query<(Entity, &GlobalTransform), With<crate::enemies::Zombie>>,
    mut instances: Query<&mut NativeSonarActor>,
    mut contacts: ResMut<Contacts>,
) {
    let attachment = inventories
        .single()
        .ok()
        .and_then(|inventory| sources.0.get(inventory.attachment()))
        .filter(|attachment| attachment.mms_weapon);
    contacts.0.retain(|entity, _| actors.contains(*entity));
    for (entity, camera, previous) in &cameras {
        let Some(attachment) = attachment else {
            if previous.is_some() {
                commands.entity(entity).remove::<NativeSonarView>();
            }
            for mut instance in &mut instances {
                instance.active = false;
            }
            continue;
        };
        assert!(
            attachment.mms_max_dist.is_finite() && attachment.mms_max_dist > 0.,
            "Original MMS attachment mmsMaxDist required"
        );
        // Source004cd110 single local view +0052c160. FOV=0 takes the original
        // centered crop path00418dab; attachment aspect is used only by FOV path.
        assert_eq!(
            attachment.mms_fov, 0.,
            "Port original nonzero MMS projection before binding this attachment"
        );
        let view = NativeSonarView {
            viewport: Vec4::new(0.25, 0.25, 0.5, 0.5),
            thermal: true,
        };
        if previous != Some(&view) {
            commands.entity(entity).insert(view);
        }
        let source_from_world = super::world::source_frame().inverse();
        let eye = source_from_world.transform_point3(camera.translation());
        for (actor, transform) in &actors {
            contacts.0.entry(actor).or_default().sample(
                eye,
                source_from_world.transform_point3(transform.translation()),
                attachment.mms_max_dist,
            );
        }
        for mut instance in &mut instances {
            instance.active = contacts
                .0
                .get(&instance.owner)
                .is_some_and(|history| history.contact.intensity > 0.);
        }
    }
}
fn bind_surfaces(
    mut commands: Commands,
    server: Res<AssetServer>,
    models: Res<Assets<AnimatedModel>>,
    mut materials: ResMut<Assets<SonarMaterial>>,
    surfaces: Query<(Entity, &NativeModelSurface), Without<SonarSurface>>,
    actors: Query<&NativeSonarActor>,
    mut shared: Local<Option<(Handle<SonarMaterial>, Handle<SonarMaterial>)>>,
    mut cache: Local<HashMap<(AssetId<AnimatedModel>, String), Handle<SonarMaterial>>>,
) {
    let (solid, overlay) = shared.get_or_insert_with(|| {
        (
            materials.add(SonarMaterial::solid()),
            materials.add(SonarMaterial::overlay()),
        )
    });
    for (entity, surface) in &surfaces {
        let ModelGeometry::Animated {
            model,
            instance,
            lod,
            surface: index,
        } = &surface.geometry
        else {
            continue;
        };
        let Ok(actor) = actors.get(*instance) else {
            continue;
        };
        let Some(model_asset) = models.get(model) else {
            continue;
        };
        let source = &model_asset.definition.lods[*lod].surfaces[*index];
        let original = &model_asset.materials[&source.material_file];
        let key = (model.id(), source.material_file.clone());
        let thermal = cache
            .entry(key)
            .or_insert_with(|| {
                original
                    .sonar_material(&server)
                    .unwrap_or_else(|e| panic!("MMS thermal material {}: {e}", source.material))
                    .map_or_else(|| solid.clone(), |material| materials.add(material))
            })
            .clone();
        commands.entity(entity).insert(SonarSurface {
            actor: actor.owner,
            thermal,
            overlay: overlay.clone(),
        });
    }
}
fn extract_sonar(
    mut request: ResMut<SonarRequest>,
    views: Extract<Query<(RenderEntity, &NativeSonarView, &GlobalTransform)>>,
    surfaces: Extract<
        Query<(
            &NativeModelSurface,
            &SonarSurface,
            &GlobalTransform,
            &InheritedVisibility,
        )>,
    >,
    entities: Extract<Query<RenderEntity>>,
    contacts: Extract<Res<Contacts>>,
) {
    request.views.clear();
    request.draws.clear();
    for (render, view, camera) in &views {
        request.views.insert(render.into(), *view);
        let to_eye = Mat4::from_translation(
            -super::world::source_frame()
                .inverse()
                .transform_point3(camera.translation()),
        ) * super::world::source_frame().inverse();
        for (source, surface, transform, visibility) in &surfaces {
            if !visibility.get() {
                continue;
            }
            let Some(contact) = contacts
                .0
                .get(&surface.actor)
                .map(|h| h.contact)
                .filter(|c| c.intensity > 0.)
            else {
                continue;
            };
            let ModelGeometry::Animated {
                model,
                instance,
                lod,
                surface: index,
            } = &source.geometry
            else {
                continue;
            };
            let Ok(render_instance) = entities.get(*instance) else {
                continue;
            };
            let world = to_eye * transform.to_matrix();
            request.draws.push(ExtractedDraw {
                model: model.id(),
                instance: render_instance.into(),
                lod: *lod,
                surface: *index,
                first_index: source.first_index,
                index_count: source.index_count,
                world_rows: std::array::from_fn(|i| world.row(i)),
                contact,
                thermal: surface.thermal.id(),
                overlay: surface.overlay.id(),
                sort: source.sort_key,
            });
        }
    }
    request.draws.sort_by_key(|draw| draw.sort);
}
pub(crate) fn prepare_sonar_views(
    mut commands: Commands,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    pipeline: Res<SonarPipeline>,
    mut specialized: ResMut<SpecializedRenderPipelines<SonarPipeline>>,
    request: Res<SonarRequest>,
    materials: Res<RenderAssets<PreparedSonarMaterial>>,
    mut views: Query<
        (
            Entity,
            &ViewTarget,
            &NativeWorldScene,
            Option<&mut PreparedSonarView>,
        ),
        With<NativeRenderView>,
    >,
) {
    for (entity, target, scene, prepared) in &mut views {
        let Some(view) = request.views.get(&entity) else {
            if prepared.is_some() {
                commands.entity(entity).remove::<PreparedSonarView>();
            }
            continue;
        };
        let extent = target.main_texture().size();
        let size = UVec2::new((extent.width >> 1).max(1), (extent.height >> 1).max(1));
        let mut replacement;
        let prepared = if let Some(prepared) = prepared.filter(|p| p.size == size) {
            prepared.into_inner()
        } else {
            let color = device.create_texture(&TextureDescriptor {
                label: Some("T6 $sonar RT039 RGBA8"),
                size: Extent3d {
                    width: size.x,
                    height: size.y,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let depth = device.create_texture(&TextureDescriptor {
                label: Some("T6 $sonar_depth RT039 depth alias"),
                format: NATIVE_DEPTH_FORMAT,
                ..TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: size.x,
                        height: size.y,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: NATIVE_DEPTH_FORMAT,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                }
            });
            let mut uniform = UniformBuffer::from(*scene);
            uniform.write_buffer(&device, &queue);
            let group = device.create_bind_group(
                "MMS scene",
                &cache.get_bind_group_layout(&pipeline.scene),
                &BindGroupEntries::single(uniform.binding().unwrap()),
            );
            replacement = Some(PreparedSonarView {
                color: color.create_view(&default()),
                depth: depth.create_view(&TextureViewDescriptor {
                    aspect: TextureAspect::DepthOnly,
                    ..default()
                }),
                attachment: depth.create_view(&default()),
                _color: color,
                _depth: depth,
                sampler: device.create_sampler(&SamplerDescriptor {
                    label: Some("source code sampler0x72"),
                    address_mode_u: AddressMode::ClampToEdge,
                    address_mode_v: AddressMode::ClampToEdge,
                    mag_filter: FilterMode::Linear,
                    min_filter: FilterMode::Linear,
                    mipmap_filter: MipmapFilterMode::Linear,
                    ..default()
                }),
                viewport: view.viewport,
                size,
                scene: uniform,
                scene_group: group,
                draws: Vec::new(),
                draw_uniforms: DynamicUniformBuffer::new_with_alignment(u64::from(
                    device.limits().min_uniform_buffer_offset_alignment,
                )),
                draw_group: None,
            });
            let prepared = replacement.as_mut().unwrap();
            prepare_draws(
                prepared,
                view,
                scene,
                &request,
                &materials,
                &device,
                &queue,
                &cache,
                &pipeline,
                &mut specialized,
            );
            commands.entity(entity).insert(replacement.take().unwrap());
            continue;
        };
        prepare_draws(
            prepared,
            view,
            scene,
            &request,
            &materials,
            &device,
            &queue,
            &cache,
            &pipeline,
            &mut specialized,
        );
    }
}
fn prepare_draws(
    prepared: &mut PreparedSonarView,
    view: &NativeSonarView,
    scene: &NativeWorldScene,
    request: &SonarRequest,
    materials: &RenderAssets<PreparedSonarMaterial>,
    device: &RenderDevice,
    queue: &RenderQueue,
    cache: &PipelineCache,
    pipeline: &SonarPipeline,
    specialized: &mut SpecializedRenderPipelines<SonarPipeline>,
) {
    let mut scene = *scene;
    // Original00418ddf..dfe zooms only copied X/Y projection axes. Centered
    // single-view offsets cancel; the lens uses the same normalized crop.
    scene.view_projection_rows[0] *= 2.;
    scene.view_projection_rows[1] *= 2.;
    prepared.scene.set(scene);
    prepared.scene.write_buffer(device, queue);
    prepared.viewport = view.viewport;
    prepared.draws.clear();
    prepared.draw_uniforms.clear();
    for draw in &request.draws {
        let material = if view.thermal {
            draw.thermal
        } else {
            draw.overlay
        };
        let Some(source) = materials.get(material) else {
            continue;
        };
        let uniform_offset = prepared.draw_uniforms.push(&SonarDraw {
            world_rows: draw.world_rows,
            script7: draw.contact.script7,
            weapon0: Vec4::new(draw.contact.intensity, 0., 0., 0.),
            material: source.constant,
        });
        prepared.draws.push(PreparedDraw {
            model: draw.model,
            instance: draw.instance,
            lod: draw.lod,
            surface: draw.surface,
            first_index: draw.first_index,
            index_count: draw.index_count,
            material,
            pipeline: specialized.specialize(
                cache,
                pipeline,
                SonarKey {
                    program: source.program,
                    state: source.state,
                },
            ),
            uniform_offset,
        });
    }
    if !prepared.draws.is_empty() {
        prepared.draw_uniforms.write_buffer(device, queue);
        let buffer = prepared.draw_uniforms.buffer().unwrap().id();
        if prepared
            .draw_group
            .as_ref()
            .is_none_or(|(previous, _)| *previous != buffer)
        {
            prepared.draw_group = Some((
                buffer,
                device.create_bind_group(
                    "MMS actor draws",
                    &cache.get_bind_group_layout(&pipeline.draw),
                    &BindGroupEntries::single(prepared.draw_uniforms.binding().unwrap()),
                ),
            ));
        }
    }
}
fn render_sonar(
    view: ViewQuery<&PreparedSonarView>,
    cache: Res<PipelineCache>,
    models: Res<RenderAssets<PreparedAnimatedModel>>,
    skins: Query<&PreparedSkin>,
    materials: Res<RenderAssets<PreparedSonarMaterial>>,
    mut ctx: RenderContext,
) {
    let view = view.into_inner();
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("T6 MMS actor draw list120"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &view.color,
            resolve_target: None,
            depth_slice: None,
            ops: Operations {
                load: LoadOp::Clear(LinearRgba::NONE.into()),
                store: StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &view.attachment,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.),
                store: StoreOp::Store,
            }),
            stencil_ops: Some(Operations {
                load: LoadOp::Clear(0),
                store: StoreOp::Store,
            }),
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    pass.set_bind_group(0, &view.scene_group, &[]);
    let Some((_, draw_group)) = &view.draw_group else {
        return;
    };
    for draw in &view.draws {
        let (Some(model), Ok(skin), Some(material), Some(pipeline)) = (
            models.get(draw.model),
            skins.get(draw.instance),
            materials.get(draw.material),
            cache.get_render_pipeline(draw.pipeline),
        ) else {
            continue;
        };
        if !skin.ready || !skin.matches(model, draw.lod) {
            continue;
        }
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(1, draw_group, &[draw.uniform_offset]);
        if let Some(group) = &material.group {
            pass.set_bind_group(2, group, &[]);
        }
        pass.set_vertex_buffer(
            0,
            skin.vertices
                .slice(model.lods[draw.lod].surface_offsets[draw.surface]..),
        );
        pass.set_index_buffer(model.indices.slice(..), IndexFormat::Uint16);
        pass.draw_indexed(
            draw.first_index..draw.first_index + draw.index_count,
            0,
            0..1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_motion_window_suppresses_fast_actor_and_recovers_over_48_samples() {
        let mut motion = MotionHistory::default();
        let mut position = Vec3::ZERO;
        for _ in 0..60 {
            position.x += 6.;
            motion.sample(Vec3::ZERO, position, 100.);
        }
        assert_eq!(motion.contact.intensity, 0.);
        for _ in 0..47 {
            motion.sample(Vec3::ZERO, position, 100.);
        }
        assert!(motion.contact.intensity < 1.);
        assert_eq!(motion.sample(Vec3::ZERO, position, 100.).intensity, 1.);
        assert_eq!(motion.contact.script7, Vec4::new(0., 0., 1., 0.));
    }
}
