use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::{
    asset::RenderAssetUsages,
    camera::{Exposure, Hdr, primitives::Aabb},
    core_pipeline::tonemapping::Tonemapping,
    gltf::{GltfExtras, GltfMaterialName},
    light::NotShadowCaster,
    pbr::Lightmap,
    post_process::bloom::Bloom,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::collections::HashMap;

#[derive(Component)]
pub struct GameCamera;

#[derive(Component)]
struct PendingLightmap(Handle<Image>);
#[derive(Component)]
struct SurfaceReady;
#[derive(Component)]
struct LanternLight;

pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup).add_systems(
            Update,
            (
                camera_follow,
                lightmaps,
                decode_lightmaps,
                surfaces,
                lantern_shadows,
                exposure.after(camera_follow),
            ),
        );
    }
}

pub(crate) fn setup(mut commands: Commands) {
    commands.insert_resource(ClearColor(Color::srgb(0.08, 0.11, 0.13)));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.48, 0.61, 0.68),
        brightness: 180.,
        affects_lightmapped_meshes: true,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 2600.,
            color: Color::srgb(0.65, 0.74, 0.8),
            affects_lightmapped_mesh_diffuse: false,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.8, -0.5, 0.)),
        Name::new("Origins sunlight"),
    ));
    commands.spawn((
        Camera3d::default(),
        GameCamera,
        Transform::from_xyz(0., 2., 0.),
        Hdr,
        Exposure::INDOOR,
        Tonemapping::AgX,
        Bloom {
            intensity: 0.18,
            ..default()
        },
        Msaa::Sample4,
        DistanceFog {
            color: Color::srgb(0.13, 0.19, 0.2),
            falloff: FogFalloff::Exponential { density: 0.009 },
            ..default()
        },
    ));
}

fn camera_follow(
    players: Query<(&Player, &Transform), Without<GameCamera>>,
    mut camera: Query<&mut Transform, With<GameCamera>>,
    definition: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
) {
    if !matches!(
        **state,
        GameState::Playing | GameState::Paused | GameState::GameOver
    ) {
        return;
    }
    let (Ok((player, body)), Some(definition), Ok(mut camera)) =
        (players.single(), definition, camera.single_mut())
    else {
        return;
    };
    let eye = if player.crouched {
        definition.tuning.crouch_height * 0.9
    } else {
        definition.tuning.eye_height
    };
    camera.translation = body.translation + Vec3::Y * (eye - player.height * 0.5);
    camera.rotation = Quat::from_rotation_y(player.yaw) * Quat::from_rotation_x(player.pitch);
}

fn lightmaps(
    mut commands: Commands,
    extras: Query<(Entity, &GltfExtras), Added<GltfExtras>>,
    children: Query<&Children>,
    meshes: Query<(), With<Mesh3d>>,
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
        Without<SurfaceReady>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut normals: Local<HashMap<AssetId<Image>, Handle<Image>>>,
) {
    for (entity, handle, name, bounds) in &surfaces {
        let Some(mut material) = materials.get_mut(&handle.0) else {
            continue;
        };
        if let Some(normal) = material.normal_map_texture.as_ref() {
            if let Some(softened) = normals.get(&normal.id()) {
                material.normal_map_texture = Some(softened.clone());
            } else {
                let Some(source) = images.get(normal) else {
                    continue;
                };
                let mut image = source.clone();
                if let Some(raw) = image.data.as_mut() {
                    for pixel in raw.chunks_exact_mut(4) {
                        let x = (pixel[0] as f32 / 127.5 - 1.) * 0.35;
                        let y = (pixel[1] as f32 / 127.5 - 1.) * 0.35;
                        pixel[0] = (x * 127.5 + 127.5) as u8;
                        pixel[1] = (y * 127.5 + 127.5) as u8;
                        pixel[2] = ((1. - x * x - y * y).max(0.).sqrt() * 127.5 + 127.5) as u8;
                    }
                }
                let id = normal.id();
                let softened = images.add(image);
                normals.insert(id, softened.clone());
                // Shared materials must not soften an already converted image again.
                normals.insert(softened.id(), softened.clone());
                material.normal_map_texture = Some(softened);
            }
        }
        material.lightmap_exposure = 4.;
        material.reflectance = 0.18;
        if name.0.contains("concrete") {
            material.perceptual_roughness = 0.9;
            material.metallic_roughness_texture = None;
        }
        if name.0.contains("glass_on") {
            material.emissive = LinearRgba::rgb(4., 1.9, 0.6);
            commands.entity(entity).insert(NotShadowCaster);
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    PointLight {
                        color: Color::srgb(1., 0.69, 0.35),
                        intensity: 6000.,
                        range: 9.,
                        radius: 0.12,
                        ..default()
                    },
                    Transform::from_translation(Vec3::from(bounds.center)),
                    LanternLight,
                    Name::new("Origins warm lantern"),
                ));
            });
        }
        commands.entity(entity).insert(SurfaceReady);
    }
}

fn lantern_shadows(
    camera: Query<&GlobalTransform, With<GameCamera>>,
    mut lights: Query<(&GlobalTransform, &mut PointLight), With<LanternLight>>,
) {
    let Ok(camera) = camera.single() else { return };
    for (transform, mut light) in &mut lights {
        light.shadow_maps_enabled = transform
            .translation()
            .distance_squared(camera.translation())
            < 100.;
    }
}

fn exposure(
    mut cameras: Query<(&Transform, &mut Exposure), With<GameCamera>>,
    players: Query<Entity, With<Player>>,
    query: avian3d::prelude::SpatialQuery,
    time: Res<Time>,
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
            &avian3d::prelude::SpatialQueryFilter::from_excluded_entities([player]),
        )
        .is_some();
    let target = if covered {
        Exposure::EV100_INDOOR
    } else {
        Exposure::EV100_BLENDER
    };
    exposure.ev100 += (target - exposure.ev100) * (1. - (-time.delta_secs() * 2.).exp());
}
