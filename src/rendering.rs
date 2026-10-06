use crate::{GameState, assets::GameDefinition, player::Player};
use bevy::{
    camera::Hdr, core_pipeline::tonemapping::Tonemapping, gltf::GltfExtras, pbr::Lightmap,
    post_process::bloom::Bloom, prelude::*,
};

#[derive(Component)]
pub struct GameCamera;

pub struct GameRenderingPlugin;
impl Plugin for GameRenderingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, (camera_follow, lightmaps));
    }
}

pub(crate) fn setup(mut commands: Commands) {
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.65, 0.72, 0.85),
        brightness: 650.,
        ..default()
    });
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.,
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
        Tonemapping::AgX,
        Bloom {
            intensity: 0.12,
            ..default()
        },
        Msaa::Sample4,
        DistanceFog {
            color: Color::srgb(0.35, 0.39, 0.42),
            falloff: FogFalloff::Exponential { density: 0.004 },
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
                    commands.entity(child).insert(Lightmap {
                        image: handle.clone(),
                        uv_rect: Rect::new(0., 0., 1., 1.),
                        bicubic_sampling: false,
                    });
                }
            }
        }
    }
}
