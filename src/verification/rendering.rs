use crate::{
    presentation::ViewModelCamera,
    rendering::{
        GameCamera,
        profile::{Antialiasing, VisualProfile},
    },
};
use bevy::{
    anti_alias::{smaa::Smaa, taa::TemporalAntiAliasing},
    light::ShadowFilteringMethod,
    prelude::*,
    remote::{BrpError, BrpResult, RemoteMethods},
    render::camera::TemporalJitter,
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
    antialiasing: Option<Antialiasing>,
    viewmodel_contact_shadows: Option<bool>,
    view: Option<[f32; 6]>,
    restore_player_view: bool,
}

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
    if request.restore_player_view {
        world.remove_resource::<crate::rendering::ValidationView>();
    }
    if let Some(view) = request.view {
        let pose = Transform::from_translation(Vec3::from_slice(&view[..3]))
            .looking_at(Vec3::from_slice(&view[3..]), Vec3::Y);
        world.insert_resource(crate::rendering::ValidationView(pose));
    }
    let cameras: Vec<_> = world.query_filtered::<(Entity, Has<ViewModelCamera>), Or<(With<GameCamera>,With<ViewModelCamera>)>>()
        .iter(world).collect();
    if let Some(aa) = request.antialiasing {
        world.resource_mut::<VisualProfile>().antialiasing = aa;
        for &(entity, first) in &cameras {
            let mut camera = world.entity_mut(entity);
            camera.remove::<(TemporalAntiAliasing, TemporalJitter, Smaa)>();
            camera.insert((
                if matches!(aa, Antialiasing::Msaa4) {
                    Msaa::Sample4
                } else {
                    Msaa::Off
                },
                if matches!(aa, Antialiasing::Taa) {
                    ShadowFilteringMethod::Temporal
                } else {
                    ShadowFilteringMethod::Gaussian
                },
            ));
            match aa {
                Antialiasing::Taa => {
                    camera.insert(TemporalAntiAliasing::default());
                }
                Antialiasing::Smaa if first => {
                    camera.insert(crate::rendering::postprocess::smaa());
                }
                _ => {}
            }
        }
    }
    if let Some(enabled) = request.viewmodel_contact_shadows {
        for &(entity, first) in &cameras {
            if first {
                if enabled {
                    world
                        .entity_mut(entity)
                        .insert(bevy::pbr::ContactShadows::default());
                } else {
                    world
                        .entity_mut(entity)
                        .remove::<bevy::pbr::ContactShadows>();
                }
            }
        }
    }
    Ok(json!({"antialiasing":world.resource::<VisualProfile>().antialiasing,"scenario_setup":true}))
}
