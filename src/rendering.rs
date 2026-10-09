use crate::{
    GameState,
    assets::{GameDefinition, WorldCollider},
    player::Player,
};
use bevy::{
    camera::{Exposure, Hdr, primitives::Aabb},
    core_pipeline::{prepass::DepthPrepass, tonemapping::Tonemapping},
    gltf::GltfMaterialName,
    light::NotShadowCaster,
    prelude::*,
};
mod first_person;
pub(crate) mod grading;
mod lighting_inspection;
mod opaque;
mod pbr;
pub(crate) mod postprocess;
mod preprocessing;
pub(crate) mod profile;
mod readback_shutdown;
mod residency;
mod shadows;
mod sky;
mod streaming;
pub(crate) use first_person::FirstPersonDepthStats;
pub(crate) use opaque::OpaqueSurfaceStatus;
pub(crate) use opaque::{LAYER_UV, LAYER3_UV, ROTATION1, ROTATION2};
pub(crate) use pbr::PbrReady;
pub(crate) use preprocessing::PreprocessingStatus;
pub(crate) use streaming::{ForegroundTexture, TextureUse};
pub(crate) use streaming::{TextureStreamingStats, TextureStreams};

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
struct SurfaceReady;
#[derive(Component)]
pub(crate) struct DynamicSurface;

#[derive(Component)]
pub(crate) struct ActorSurface;

#[derive(Component)]
struct LanternLight;

#[derive(Component)]
pub(crate) struct FixtureEmitter;

#[derive(Component)]
struct FixtureLightReady;

pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<opaque::OpaqueMaterial>::default())
            .add_plugins(first_person::FirstPersonRenderPlugin)
            .add_plugins(bevy::post_process::auto_exposure::AutoExposurePlugin)
            .add_plugins(MaterialPlugin::<pbr::DiagnosticMaterial>::default())
            .add_plugins(preprocessing::PreprocessingPlugin)
            .add_plugins(streaming::TextureStreamingPlugin)
            .add_plugins(residency::ResidencyPlugin)
            .init_resource::<profile::VisualProfile>()
            .add_plugins(lighting_inspection::LightingInspectionPlugin)
            .add_plugins(readback_shutdown::ReadbackShutdownPlugin)
            .init_resource::<OpaqueSurfaceStatus>()
            .init_resource::<pbr::MaterialDiagnostic>()
            .add_systems(Startup, (setup, opaque::initialize))
            .add_systems(PostStartup, sky::setup)
            .add_systems(Startup, cluster_capacity)
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
                    camera_follow,
                    sky::viewmodel_environment,
                    opaque::prepare_shaders,
                    (pbr::prepare, surfaces, fixture_lights, opaque::convert).chain(),
                ),
            )
            .add_systems(
                PostUpdate,
                (pbr::diagnostics, opaque::animate_water).chain(),
            );
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
    commands.insert_resource(ClearColor(profile.fog_color()));
    commands.insert_resource(GlobalAmbientLight {
        color: profile::color(profile.ambient_color),
        brightness: profile.ambient_brightness,
        affects_lightmapped_meshes: false,
    });
    commands.spawn((
        DirectionalLight {
            illuminance: profile.sun_illuminance,
            color: profile::color(profile.sun_color),
            affects_lightmapped_mesh_diffuse: true,
            contact_shadows_enabled: true,
            shadow_maps_enabled: true,
            shadow_depth_bias: 0.002,
            shadow_normal_bias: 0.25,
            ..default()
        },
        bevy::camera::visibility::RenderLayers::from_layers(&[0, 1]),
        bevy::light::CascadeShadowConfigBuilder {
            num_cascades: 3,
            maximum_distance: profile.sun_shadow_distance,
            first_cascade_far_bound: 12.,
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            profile.sun_rotation[0],
            profile.sun_rotation[1],
            profile.sun_rotation[2],
        )),
        Name::new("Origins sunlight"),
    ));
    let mut camera = commands.spawn((
        Camera3d::default(),
        bevy::pbr::ScreenSpaceTransmission::default(),
        GameCamera,
        bevy::light::ShadowFilteringMethod::Gaussian,
        DepthPrepass,
        Transform::from_xyz(0., 2., 0.),
        Hdr,
        Exposure {
            ev100: profile.outdoor_ev100,
        },
        Tonemapping::None,
        match profile.antialiasing {
            profile::Antialiasing::Msaa4 => Msaa::Sample4,
            profile::Antialiasing::Smaa => Msaa::Off,
        },
    ));
    camera.insert(bevy::audio::SpatialListener::new(0.2));
}

fn camera_follow(
    players: Query<
        (
            &Player,
            &Transform,
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
    if !matches!(
        **state,
        GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let (
        Ok((player, body, inventory, mobility, feedback)),
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
        translation: body.translation
            + Vec3::Y * (eye - player.height * 0.5)
            + Quat::from_rotation_y(player.yaw) * Vec3::new(bob.x, bob.y, 0.),
        rotation: Quat::from_rotation_y(player.yaw + inventory.recoil.x)
            * Quat::from_rotation_x((player.pitch + inventory.recoil.y).clamp(-1.5, 1.5)),
        ..*camera
    };
    camera.set_if_neq(target);
    if **state == GameState::Playing
        && let Projection::Perspective(perspective) = &mut *projection
    {
        perspective.fov = inventory
            .definition(&definition.content.weapons)
            .map_or(55_f32.to_radians(), |w| {
                inventory.ads_fov(55., &w.mechanics)
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
            if actor {
                commands.entity(entity).insert(ActorSurface);
            }
            if actor || opaque::requires_extension(&library, &name.0) {
                commands.entity(entity).insert(SurfaceReady);
            }
            continue;
        }
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        let Some(definition) = library.definitions.get(&name.0) else {
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
        if state["lamp"].as_bool() == Some(true) {
            commands.entity(entity).insert(NotShadowCaster);
        }
        commands.entity(entity).insert(SurfaceReady);
    }
}

fn fixture_lights(
    mut commands: Commands,
    fixtures: Query<
        (Entity, &Aabb),
        (
            With<FixtureEmitter>,
            With<PbrReady>,
            Without<FixtureLightReady>,
        ),
    >,
    profile: Res<profile::VisualProfile>,
) {
    for (entity, bounds) in &fixtures {
        commands
            .entity(entity)
            .insert(FixtureLightReady)
            .with_children(|parent| {
                parent.spawn((
                    PointLight {
                        color: profile::color(profile.lantern_color),
                        intensity: profile.lantern_lumens,
                        range: profile.lantern_range,
                        radius: 0.12,
                        shadow_depth_bias: 0.005,
                        shadow_normal_bias: 0.3,
                        shadow_maps_enabled: false,
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
