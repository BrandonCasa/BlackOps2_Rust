use crate::rendering::{GameCamera, profile::VisualProfile};
use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethods},
};
use serde::Deserialize;
use serde_json::{Value, json};

#[cfg(windows)]
mod indirect;

pub(super) fn install(app: &mut App) {
    #[cfg(windows)]
    {
        let system = app.world_mut().register_system(indirect::probe);
        app.world_mut().resource_mut::<RemoteMethods>().insert(
            "origins/rendering/indirect",
            bevy::remote::RemoteMethodSystemId::Instant(system),
        );
    }
    let system = app.world_mut().register_system(configure);
    app.world_mut().resource_mut::<RemoteMethods>().insert(
        "origins/rendering",
        bevy::remote::RemoteMethodSystemId::Instant(system),
    );
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Setup {
    view: Option<[f32; 6]>,
    restore_player_view: bool,
    color_patches: Option<Vec<[f32; 3]>>,
    bloom: Option<bool>,
}

#[derive(Component)]
struct ColorPatch;

fn configure(In(params): In<Option<Value>>, world: &mut World) -> BrpResult {
    let request: Setup =
        serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|error| BrpError {
            code: -32602,
            message: error.to_string(),
            data: None,
        })?;
    if request
        .view
        .is_some_and(|v| v.iter().any(|x| !x.is_finite()))
    {
        return Err(BrpError {
            code: -32602,
            message: "View must be finite [x,y,z,target_x,target_y,target_z]".into(),
            data: None,
        });
    }
    if request.color_patches.as_ref().is_some_and(|colors| {
        colors.len() > 64
            || colors
                .iter()
                .flatten()
                .any(|v| !v.is_finite() || *v < 0. || *v > 16.)
    }) {
        return Err(BrpError {
            code: -32602,
            message: "Color patches require at most 64 finite RGB values in [0,16]".into(),
            data: None,
        });
    }
    if request.restore_player_view {
        world.remove_resource::<crate::rendering::ValidationView>();
    }
    if let Some(view) = request.view {
        let pose = Transform::from_translation(Vec3::from_slice(&view[..3]))
            .looking_at(Vec3::from_slice(&view[3..]), Vec3::Y);
        world.insert_resource(crate::rendering::ValidationView(pose));
    }
    let cameras: Vec<_> = world
        .query_filtered::<Entity, With<GameCamera>>()
        .iter(world)
        .collect();
    if let Some(enabled) = request.bloom {
        for &entity in &cameras {
            if enabled {
                let bloom = crate::rendering::postprocess::bloom(world.resource::<VisualProfile>());
                world.entity_mut(entity).insert(bloom);
            } else {
                world
                    .entity_mut(entity)
                    .remove::<bevy::post_process::bloom::Bloom>();
            }
        }
    }
    let mut patches = Vec::new();
    if let Some(colors) = request.color_patches {
        let old: Vec<_> = world
            .query_filtered::<Entity, With<ColorPatch>>()
            .iter(world)
            .collect();
        for entity in old {
            world.despawn(entity);
        }
        if let Some(&camera) = cameras.first() {
            let projection = world.get::<Projection>(camera);
            let aspect = match projection {
                Some(Projection::Perspective(p)) => p.aspect_ratio,
                _ => 16. / 9.,
            };
            let fov = match projection {
                Some(Projection::Perspective(p)) => p.fov,
                _ => crate::rendering::BASE_FOV_DEGREES.to_radians(),
            };
            let half_height = 0.1 * (fov * 0.5).tan();
            let half_width = half_height * aspect;
            let columns = 8;
            let rows = colors.len().div_ceil(columns).max(1);
            let width = 1.8 * half_width / columns as f32;
            let height = 1.6 * half_height / rows as f32;
            let mesh = world
                .resource_mut::<Assets<Mesh>>()
                .add(Rectangle::new(width * 0.9, height * 0.9));
            for (index, rgb) in colors.into_iter().enumerate() {
                let x = (index % columns) as f32;
                let y = (index / columns) as f32;
                let position = Vec3::new(
                    -0.9 * half_width + (x + 0.5) * width,
                    0.8 * half_height - (y + 0.5) * height,
                    -0.1,
                );
                let material =
                    world
                        .resource_mut::<Assets<StandardMaterial>>()
                        .add(StandardMaterial {
                            base_color: Color::linear_rgb(rgb[0], rgb[1], rgb[2]),
                            unlit: true,
                            fog_enabled: false,
                            ..default()
                        });
                world.spawn((
                    Mesh3d(mesh.clone()),
                    MeshMaterial3d(material),
                    Transform::from_translation(position),
                    bevy::camera::visibility::RenderLayers::layer(1),
                    ChildOf(camera),
                    ColorPatch,
                    crate::gameplay::SessionEntity,
                    bevy::light::NotShadowCaster,
                ));
                patches.push(
                    json!({"rgb": rgb, "uv": [0.05 + (x + 0.5) * 0.9 / columns as f32,
                    0.1 + (y + 0.5) * 0.8 / rows as f32]}),
                );
            }
        }
    }
    Ok(
        json!({"antialiasing":"dlss_ray_reconstruction","scenario_setup":true, "color_patches":patches}),
    )
}
