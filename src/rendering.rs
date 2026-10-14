use crate::{
    GameState,
    assets::{GameDefinition, WorldCollider},
    player::Player,
};
use bevy::{
    camera::{Hdr, primitives::Aabb},
    core_pipeline::{prepass::DepthPrepass, tonemapping::Tonemapping},
    gltf::GltfMaterialName,
    prelude::*,
};
pub(crate) mod atmosphere;
mod animated_surfaces;
mod effect_models;
mod eyes;
pub(crate) mod grading;
mod lighting_inspection;
mod lod;
pub(crate) mod opaque;
mod pbr;
pub(crate) mod postprocess;
pub(crate) mod preparation;
mod preprocessing;
pub(crate) mod profile;
mod readback_shutdown;
mod residency;
mod shadows;
mod sky;
mod solari;
mod viewport;
pub(crate) use opaque::NativeArtInputs;
pub(crate) use opaque::OpaqueSurfaceStatus;
#[cfg(feature = "dev-tools")]
pub(crate) use opaque::weapon_material_snapshot;
pub(crate) use pbr::PbrReady;
pub(crate) use pbr::validate as validate_surface;
pub(crate) use preprocessing::PreprocessingStatus;

pub(crate) const BASE_FOV_DEGREES: f32 = 55.;

#[derive(Component, Clone, bevy::render::extract_component::ExtractComponent)]
#[extract_app(bevy::render::RenderApp)]
pub struct GameCamera;

#[derive(Component)]
struct SurfaceReady;
#[derive(Component)]
pub(crate) struct DynamicSurface;

#[derive(Component)]
pub(crate) struct FirstPersonSurface;

#[derive(Component)]
pub(crate) struct ActorSurface;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SurfacePreparation;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MaterialAnimation;

#[derive(Component)]
struct LanternLight;

#[derive(Component)]
pub(crate) struct FixtureEmitter(pub String);

#[cfg(feature = "dev-tools")]
#[derive(Resource)]
pub(crate) struct ValidationView(pub Transform);

impl FixtureEmitter {
    fn radius(&self) -> f32 {
        match self.0.as_str() {
            "p6_zm_tm_lantern_01_on" => 0.025,
            "p6_zm_tm_hanging_light_fixture_on" => 0.08,
            "p6_zm_tm_robot_lamp" => 0.06,
            "p6_zm_tm_portable_lantern_on" | "p6_zm_tm_gas_lamp_straight_on" => 0.02,
            model => panic!("Unsupported fixture emitter: {model}"),
        }
    }
}

#[derive(Component)]
struct FixtureLightReady;

pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        preparation::install(app);
        effect_models::install(app);
        pbr::install(app);
        animated_surfaces::install(app);
        app.add_plugins(solari::OriginsSolariPlugin)
            .add_plugins(lod::PropLodPlugin)
            .add_plugins(preprocessing::PreprocessingPlugin)
            .add_plugins(residency::ResidencyPlugin)
            .init_resource::<profile::VisualProfile>()
            .add_plugins(atmosphere::AtmospherePlugin)
            .add_plugins(lighting_inspection::LightingInspectionPlugin)
            .add_plugins(readback_shutdown::ReadbackShutdownPlugin)
            .init_resource::<OpaqueSurfaceStatus>()
            .init_resource::<opaque::MaterialTime>()
            .init_resource::<opaque::NativeClock>()
            .add_systems(Startup, setup)
            .add_systems(
                OnEnter(GameState::Loading),
                opaque::initialize.after(crate::assets::begin_load),
            )
            .add_systems(
                OnEnter(GameState::Loading),
                eyes::reset.after(opaque::initialize),
            )
            .add_systems(
                OnEnter(GameState::MainMenu),
                (opaque::shutdown_native_time, eyes::reset),
            )
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                opaque::begin_native_time.after(crate::session::SessionSetup::Initialize),
            )
            .add_systems(
                Update,
                (
                    opaque::prepare_native_time_owners,
                    opaque::advance_native_time,
                )
                    .chain()
                    .before(SurfacePreparation),
            )
            .add_systems(
                PostUpdate,
                pbr::prepare_image_dimensions.after(bevy::asset::AssetEventSystems),
            )
            .add_systems(Startup, cluster_capacity)
            .add_systems(PostUpdate, shadows::restrict_contact_shadows)
            .add_systems(
                PostUpdate,
                viewport::fit.before(bevy::camera::CameraUpdateSystems),
            )
            .add_systems(
                PostUpdate,
                shadows::select_local_lights
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::light::SimulationLightSystems::AssignLightsToClusters)
                    .before(bevy::camera::visibility::VisibilitySystems::VisibilityPropagate),
            )
            .add_systems(
                Update,
                (
                    camera_follow.run_if(crate::flythrough::normal_play),
                    (
                        pbr::refresh,
                        pbr::prepare,
                        animated_surfaces::prepare,
                        surfaces,
                        fixture_lights,
                        opaque::convert,
                        animated_surfaces::pending,
                        eyes::prepare.run_if(opaque::material_scene_active),
                    )
                        .chain()
                        .in_set(SurfacePreparation),
                ),
            )
            .add_systems(
                PostUpdate,
                opaque::animate_flags
                    .after(MaterialAnimation)
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility)
                    .run_if(opaque::material_scene_active),
            )
            .add_systems(
                PostUpdate,
                (
                    opaque::advance_time.run_if(in_state(GameState::Playing)),
                    opaque::refresh_owners,
                    opaque::animate.run_if(in_state(GameState::Playing)),
                    opaque::animate_scalars,
                    opaque::animate_uvs,
                    eyes::animate,
                )
                    .chain()
                    .in_set(MaterialAnimation)
                    .run_if(opaque::material_scene_active),
            );
    }

    fn finish(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render.add_observer(shadows::retire_hidden_light_views);
        }
    }
}

fn cluster_capacity(mut settings: ResMut<bevy::light::cluster::GlobalClusterSettings>) {
    if let Some(gpu) = settings.gpu_clustering.as_mut() {
        gpu.initial_z_slice_list_capacity = gpu.initial_z_slice_list_capacity.max(8192);
        gpu.initial_index_list_capacity = gpu.initial_index_list_capacity.max(262144);
    }
}

pub(crate) fn setup(mut commands: Commands, profile: Res<profile::VisualProfile>) {
    commands.insert_resource(bevy::light::PointLightShadowMap {
        size: profile.lantern_shadow_map_size,
    });
    commands.insert_resource(bevy::light::DirectionalLightShadowMap {
        size: profile.sun_shadow_map_size,
    });
    commands.insert_resource(GlobalAmbientLight {
        color: profile::color(profile.ambient_color),
        brightness: profile.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
    let mut camera = commands.spawn((
        Camera3d::default(),
        bevy::pbr::ScreenSpaceTransmission::default(),
        GameCamera,
        bevy::camera::ShadowLodOrigin,
        DepthPrepass,
        Transform::from_xyz(0., 2., 0.),
        Hdr,
        (
            Tonemapping::TonyMcMapface,
            bevy::render::view::ColorGrading::default(),
            postprocess::bloom(&profile),
            bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
            Projection::Perspective(PerspectiveProjection {
                fov: BASE_FOV_DEGREES.to_radians(),
                near: 0.01,
                ..default()
            }),
            Name::new("Origins camera"),
        ),
        bevy::core_pipeline::tonemapping::DebandDither::Disabled,
        Msaa::Off,
        bevy::solari::prelude::SolariLighting::default(),
        bevy::camera::CameraMainTextureUsages::default().with(bevy::render::render_resource::TextureUsages::STORAGE_BINDING),
        bevy::anti_alias::dlss::Dlss::<bevy::anti_alias::dlss::DlssRayReconstructionFeature>::default(),
    ));
    camera.insert(bevy::audio::SpatialListener::new(0.2));
}

fn configure(
    mut commands: Commands,
    profile: Res<profile::VisualProfile>,
    source: Res<atmosphere::SourceAtmosphere>,
    cameras: Query<Entity, With<GameCamera>>,
    suns: Query<Entity, With<atmosphere::Sun>>,
) {
    commands.insert_resource(ClearColor(source.fog.native(source.exposure).color));
    for sun in &suns {
        commands.entity(sun).despawn();
    }
    for camera in &cameras {
        commands
            .entity(camera)
            .insert(source.camera_exposure(&profile));
    }
    commands.spawn((
        DirectionalLight {
            illuminance: profile.sun_illuminance,
            color: Color::linear_rgb(
                source.sun_color[0],
                source.sun_color[1],
                source.sun_color[2],
            ),
            affects_lightmapped_mesh_diffuse: true,
            contact_shadows_enabled: false,
            shadow_maps_enabled: false,
            shadow_depth_bias: 0.002,
            shadow_normal_bias: 0.25,
            ..default()
        },
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: profile.sun_shadow_cascades,
            maximum_distance: profile.sun_shadow_distance,
            first_cascade_far_bound: profile.sun_shadow_first_cascade,
            ..default()
        }
        .build(),
        Transform::IDENTITY.looking_to(
            -atmosphere::source_direction(source.sun_angles[0], source.sun_angles[1]),
            Vec3::Y,
        ),
        Name::new("Origins sunlight"),
        atmosphere::Sun,
    ));
}

fn camera_follow(
    #[cfg(feature = "dev-tools")] validation_view: Option<Res<ValidationView>>,
    players: Query<
        (
            &Player,
            &Transform,
            &crate::player::PreviousTranslation,
            &crate::weapons::WeaponInventory,
            &crate::movement::Mobility,
            &crate::movement::MotionFeedback,
        ),
        Without<GameCamera>,
    >,
    time: Res<Time<Fixed>>,
    mut camera: Query<(&mut Transform, &mut Projection), With<GameCamera>>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
) {
    #[cfg(feature = "dev-tools")]
    if let Some(view) = validation_view {
        for (mut transform, _) in &mut camera {
            *transform = view.0;
        }
        return;
    }
    if !matches!(
        **state,
        GameState::PreparingShaders | GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let (
        Ok((player, body, previous, inventory, mobility, feedback)),
        Some(definition),
        Ok((mut camera, mut projection)),
    ) = (players.single(), definition, camera.single_mut())
    else {
        return;
    };
    let eye = if **state == GameState::Playing {
        mobility
            .previous_eye
            .lerp(player.eye_height, time.overstep_fraction())
    } else {
        player.eye_height
    };
    let bob = feedback
        .previous_bob
        .lerp(feedback.bob, time.overstep_fraction());
    let target = Transform {
        translation: (if **state == GameState::Playing {
            previous.0.lerp(body.translation, time.overstep_fraction())
        } else {
            body.translation
        }) + Vec3::Y * (eye - player.height * 0.5)
            + Quat::from_rotation_y(player.yaw) * Vec3::new(bob.x, bob.y, 0.),
        rotation: Quat::from_rotation_y(player.yaw + inventory.recoil.x + feedback.rumble.x)
            * Quat::from_rotation_x(
                (player.pitch + inventory.recoil.y + feedback.rumble.y).clamp(-1.5, 1.5),
            ),
        ..*camera
    };
    camera.set_if_neq(target);
    if **state == GameState::Playing
        && let Projection::Perspective(perspective) = &mut *projection
    {
        perspective.fov = inventory
            .definition(&definition.content.weapons)
            .map_or(BASE_FOV_DEGREES.to_radians(), |w| {
                inventory.ads_fov(BASE_FOV_DEGREES, w)
            });
    }
}

fn surfaces(
    mut commands: Commands,
    surfaces: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialName,
            Has<bevy::mesh::skinning::SkinnedMesh>,
        ),
        (
            Without<SurfaceReady>,
            Without<WorldCollider>,
            Without<DynamicSurface>,
            With<PbrReady>,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    library: Option<Res<opaque::SurfaceLibrary>>,
    profile: Res<profile::VisualProfile>,
    parents: Query<&ChildOf>,
    zombies: Query<(), With<crate::enemies::Zombie>>,
    boards: Query<
        (),
        Or<(
            With<crate::barriers::BoardVisual>,
            With<crate::generators::GeneratorVisual>,
            With<crate::enemies::Zombie>,
            With<crate::equipment::Grenade>,
            With<crate::mines::PlantedMine>,
            With<crate::presentation::ViewModelGroup>,
        )>,
    >,
) {
    let Some(library) = library else { return };
    for (entity, handle, name, skinned) in &surfaces {
        let mut ancestor = Some(entity);
        let mut dynamic = skinned;
        let mut actor = false;
        while let Some(e) = ancestor {
            actor |= zombies.contains(e);
            if boards.contains(e) {
                dynamic = true;
                break;
            }
            ancestor = parents.get(e).ok().map(ChildOf::parent);
        }
        if dynamic {
            commands.entity(entity).insert(DynamicSurface);
            if skinned {
                commands
                    .entity(entity)
                    .insert(bevy::render::batching::NoAutomaticBatching);
            }
            if actor {
                commands.entity(entity).insert(ActorSurface);
            }
            commands.entity(entity).insert(SurfaceReady);
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let Some(definition) = library.definitions.get(&name.0) else {
            commands.entity(entity).insert(SurfaceReady);
            continue;
        };
        let state = &definition.presentation;
        if state["visible"].as_bool() == Some(false) {
            commands.entity(entity).insert(Visibility::Hidden);
        }
        material.fog_enabled = state["fog"].as_bool().unwrap_or(true);
        material.double_sided = state["cull"].as_str() == Some("none");
        material.cull_mode = match state["cull"].as_str() {
            Some("none") => None,
            Some("front") => Some(bevy::render::render_resource::Face::Front),
            _ => Some(bevy::render::render_resource::Face::Back),
        };
        if state["emissive"].as_bool() == Some(true) {
            material.emissive_exposure_weight = 1.;
            material.emissive = if state["lamp"].as_bool() == Some(true) {
                profile::color(profile.lantern_color).to_linear() * profile.lantern_emissive_nits
            } else {
                LinearRgba::WHITE * profile.emissive_nits
            };
        }
        commands.entity(entity).insert(SurfaceReady);
    }
}

fn fixture_lights(
    mut commands: Commands,
    fixtures: Query<
        (Entity, &Aabb, &FixtureEmitter),
        (
            With<FixtureEmitter>,
            With<PbrReady>,
            Without<FixtureLightReady>,
        ),
    >,
    profile: Res<profile::VisualProfile>,
) {
    for (entity, bounds, emitter) in &fixtures {
        commands
            .entity(entity)
            .insert(FixtureLightReady)
            .with_children(|parent| {
                parent.spawn((
                    PointLight {
                        color: profile::color(profile.lantern_color),
                        intensity: profile.lantern_lumens,
                        range: profile.lantern_range,
                        radius: emitter.radius(),
                        shadow_depth_bias: 0.005,
                        shadow_normal_bias: 0.3,
                        shadow_map_near_z: 0.01,
                        shadow_maps_enabled: false,
                        contact_shadows_enabled: false,
                        ..default()
                    },
                    Transform::from_translation(Vec3::from(bounds.center)),
                    LanternLight,
                    Visibility::Hidden,
                    bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
                    Name::new("Origins warm lantern"),
                ));
            });
    }
}
