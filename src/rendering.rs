use crate::{
    GameState,
    assets::{GameDefinition, WorldCollider},
    player::Player,
};
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::NoFrustumCulling,
    camera::{Exposure, Hdr, primitives::Aabb},
    core_pipeline::tonemapping::Tonemapping,
    gltf::{GltfExtras, GltfMaterialName},
    light::NotShadowCaster,
    pbr::Lightmap,
    post_process::bloom::Bloom,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    render::view::{ColorGrading, ColorGradingGlobal},
};
use std::collections::HashMap;
mod mipmaps;
mod opaque;
pub(crate) mod profile;
mod residency;
mod shadow_cache;
mod sky;
mod static_probes;
pub(crate) use opaque::OpaqueSurfaceStatus;
pub(crate) use opaque::{LAYER_UV, LAYER3_UV, ROTATION1, ROTATION2};
pub(crate) use shadow_cache::ShadowCacheStats;

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
struct PendingLightmap(Handle<Image>);
#[derive(Component)]
struct SurfaceReady;
#[derive(Component)]
pub(crate) struct DynamicSurface;
#[derive(Component)]
struct LanternLight;

pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<opaque::OpaqueMaterial>::default())
            .add_plugins(static_probes::StaticProbePlugin)
            .add_plugins((shadow_cache::ShadowCachePlugin, residency::ResidencyPlugin))
            .init_resource::<profile::VisualProfile>()
            .init_resource::<OpaqueSurfaceStatus>()
            .add_systems(Startup, (setup, opaque::initialize))
            .add_systems(PostStartup, sky::setup)
            .add_systems(Startup, cpu_clusters)
            .add_systems(
                Update,
                (
                    camera_follow,
                    opaque::probes,
                    opaque::baked_lighting,
                    mipmaps::prepare.before(opaque::convert),
                    (
                        lightmaps,
                        surfaces,
                        opaque::convert.run_if(resource_exists::<opaque::BakedLightingReady>),
                        decode_lightmaps,
                    )
                        .chain(),
                    opaque::animate_water.after(opaque::convert),
                    exposure.after(camera_follow),
                ),
            );
    }
}

fn cpu_clusters(mut settings: ResMut<bevy::light::cluster::GlobalClusterSettings>) {
    settings.gpu_clustering = None;
}

pub(crate) fn setup(mut commands: Commands, profile: Res<profile::VisualProfile>) {
    commands.insert_resource(bevy::light::PointLightShadowMap {
        size: profile.lantern_shadow_map_size,
    });
    commands.insert_resource(ClearColor(profile::color(profile.fog_color)));
    commands.insert_resource(GlobalAmbientLight {
        color: profile::color(profile.ambient_color),
        brightness: profile.ambient_brightness,
        affects_lightmapped_meshes: false,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: profile.sun_illuminance,
            color: profile::color(profile.sun_color),
            affects_lightmapped_mesh_diffuse: false,
            shadow_maps_enabled: true,
            ..default()
        },
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
        GameCamera,
        Transform::from_xyz(0., 2., 0.),
        Hdr,
        Exposure {
            ev100: profile.indoor_ev100,
        },
        Tonemapping::AgX,
        ColorGrading {
            global: ColorGradingGlobal {
                post_saturation: profile.post_saturation,
                ..default()
            },
            ..default()
        },
        Bloom {
            intensity: profile.bloom_intensity,
            ..default()
        },
        match profile.antialiasing {
            profile::Antialiasing::Msaa4 => Msaa::Sample4,
            profile::Antialiasing::Smaa => Msaa::Off,
        },
        DistanceFog {
            color: profile::color(profile.fog_color),
            falloff: FogFalloff::Exponential {
                density: profile.fog_density,
            },
            ..default()
        },
    ));
    camera.insert(bevy::audio::SpatialListener::new(0.2));
    if matches!(profile.antialiasing, profile::Antialiasing::Smaa) {
        camera.insert(bevy::anti_alias::smaa::Smaa::default());
    }
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
        let ads_fov = inventory
            .definition(&definition.content.weapons)
            .map_or(55., |w| w.mechanics.ads_fov);
        perspective.fov = (55. + (ads_fov - 55.) * inventory.ads).to_radians();
    }
}

fn lightmaps(
    mut commands: Commands,
    extras: Query<(Entity, &GltfExtras), Added<GltfExtras>>,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<WorldCollider>)>,
    server: Res<AssetServer>,
) {
    for (entity, extras) in &extras {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&extras.value)
            && let Some(path) = value.get("lightmap").and_then(|v| v.as_str())
        {
            let handle = server.load(path.to_string());
            for child in std::iter::once(entity).chain(children.iter_descendants(entity)) {
                if meshes.contains(child) {
                    commands
                        .entity(child)
                        .insert(PendingLightmap(handle.clone()));
                }
            }
        }
    }
}

fn decode_lightmaps(
    mut commands: Commands,
    pending: Query<(Entity, &PendingLightmap)>,
    mut images: ResMut<Assets<Image>>,
    mut cache: Local<HashMap<AssetId<Image>, Handle<Image>>>,
) {
    for (entity, pending) in &pending {
        let id = pending.0.id();
        let handle = if let Some(handle) = cache.get(&id) {
            handle.clone()
        } else {
            let Some(source) = images.get(&pending.0) else {
                continue;
            };
            let size = source.texture_descriptor.size;
            let Some(raw) = source.data.as_ref() else {
                continue;
            };
            if size.height % 3 != 0 || raw.len() != size.width as usize * size.height as usize * 4 {
                warn!("Unsupported Origins packed lightmap layout");
                commands.entity(entity).remove::<PendingLightmap>();
                continue;
            }
            let height = size.height / 3;
            let band = size.width as usize * height as usize * 4;
            let mut pixels = Vec::with_capacity(band);
            for (ambient, directional) in raw[..band]
                .chunks_exact(4)
                .zip(raw[band..band * 2].chunks_exact(4))
            {
                // Original atlas: RGB/alpha ambient, RGB/alpha directional, then direction.
                // A diffuse average replaces the native per-fragment directional response.
                for channel in 0..3 {
                    let irradiance = ambient[channel] as f32 / ambient[3].max(1) as f32
                        + 0.35 * directional[channel] as f32 / directional[3].max(1) as f32;
                    pixels.push((irradiance / 4. * 255.).clamp(0., 255.) as u8);
                }
                pixels.push(255);
            }
            let image = Image::new(
                Extent3d {
                    width: size.width,
                    height,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                pixels,
                TextureFormat::Rgba8Unorm,
                RenderAssetUsages::default(),
            );
            let handle = images.add(image);
            cache.insert(id, handle.clone());
            handle
        };
        commands
            .entity(entity)
            .remove::<PendingLightmap>()
            .insert(Lightmap {
                image: handle,
                uv_rect: Rect::new(0., 0., 1., 1.),
                bicubic_sampling: true,
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
            &Aabb,
        ),
        (
            Without<SurfaceReady>,
            Without<WorldCollider>,
            Without<DynamicSurface>,
        ),
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    library: Option<Res<opaque::SurfaceLibrary>>,
    profile: Res<profile::VisualProfile>,
    parents: Query<&ChildOf>,
    boards: Query<
        (),
        Or<(
            With<crate::barriers::BoardVisual>,
            With<crate::generators::GeneratorVisual>,
        )>,
    >,
) {
    let Some(library) = library else { return };
    for (entity, handle, name, bounds) in &surfaces {
        let mut ancestor = Some(entity);
        let mut dynamic = false;
        while let Some(e) = ancestor {
            if boards.contains(e) {
                dynamic = true;
                break;
            }
            ancestor = parents.get(e).ok().map(ChildOf::parent);
        }
        if dynamic {
            commands.entity(entity).insert(DynamicSurface);
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
        material.alpha_mode = match state["alpha"].as_str() {
            Some("mask") => AlphaMode::Mask(state["cutoff"].as_f64().unwrap_or(0.5) as f32),
            Some("blend") => AlphaMode::Blend,
            Some("premultiplied") => AlphaMode::Premultiplied,
            Some("add") => AlphaMode::Add,
            Some("multiply") => AlphaMode::Multiply,
            _ => AlphaMode::Opaque,
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
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    PointLight {
                        color: profile::color(profile.lantern_color),
                        intensity: profile.lantern_lumens,
                        range: profile.lantern_range,
                        radius: 0.12,
                        shadow_maps_enabled: true,
                        ..default()
                    },
                    Transform::from_translation(Vec3::from(bounds.center)),
                    LanternLight,
                    // Keep cube-array ownership stable across camera rotation.
                    // Bevy still clusters the finite-range contribution per fragment.
                    NoFrustumCulling,
                    Name::new("Origins warm lantern"),
                ));
            });
        }
        commands.entity(entity).insert(SurfaceReady);
    }
}

fn exposure(
    mut cameras: Query<(&Transform, &mut Exposure), With<GameCamera>>,
    players: Query<Entity, With<Player>>,
    query: avian3d::prelude::SpatialQuery,
    time: Res<Time>,
    profile: Res<profile::VisualProfile>,
) {
    let (Ok((camera, mut exposure)), Ok(player)) = (cameras.single_mut(), players.single()) else {
        return;
    };
    let covered = query
        .cast_ray(
            camera.translation + Vec3::Y * 0.1,
            Dir3::Y,
            20.,
            true,
            &crate::environment::solids([player]),
        )
        .is_some();
    let target = if covered {
        profile.indoor_ev100
    } else {
        profile.outdoor_ev100
    };
    exposure.ev100 +=
        (target - exposure.ev100) * (1. - (-time.delta_secs() * profile.adaptation_speed).exp());
}

/// Reconstruct native dynamic-object probe fill while retaining direct lights and normals.
pub(crate) fn with_actor_fill(source: &StandardMaterial) -> StandardMaterial {
    let mut material = source.clone();
    material.emissive_texture = material.base_color_texture.clone();
    material.emissive = LinearRgba::rgb(90., 90., 90.);
    material.emissive_exposure_weight = 1.;
    material
}
