use super::{LanternLight, profile::VisualProfile};
use bevy::prelude::*;

pub(super) fn restrict_contact_shadows(
    mut commands: Commands,
    cameras: Query<Entity, With<bevy::pbr::ContactShadows>>,
) {
    for camera in &cameras {
        commands
            .entity(camera)
            .remove::<bevy::pbr::ContactShadows>();
    }
}

pub(super) fn excludes_caster(surface: &serde_json::Value) -> bool {
    let identity = surface["identity"].as_str().unwrap_or_default();
    identity
        .split(['/', '_'])
        .any(|part| matches!(part, "glass" | "glassdirt" | "lens"))
        || matches!(
            surface["native_weapon"]["kind"].as_str(),
            Some("lens" | "reticle")
        )
        || (surface["presentation"]["lamp"].as_bool() == Some(true)
            && identity != "mc/mtl_p6_zm_tm_portable_lantern_on")
}

pub(super) fn retire_hidden_light_views(
    removed: On<Remove<bevy::pbr::ExtractedPointLight>>,
    views: Query<&bevy::pbr::PointAndSpotLightViewEntities>,
    mut commands: Commands,
) {
    if let Ok(views) = views.get(removed.entity) {
        if !views.is_empty() {
            info!(views = views.len(), "Retiring hidden-light shadow views");
        }
        commands
            .entity(removed.entity)
            .try_remove::<bevy::pbr::PointAndSpotLightViewEntities>();
    }
}

pub(super) fn select_local_lights(
    mut lights: Query<
        (&GlobalTransform, &ChildOf, &mut PointLight, &mut Visibility),
        With<LanternLight>,
    >,
    profile: Res<VisualProfile>,
    emitters: Query<&super::FixtureEmitter>,
) {
    for (pose, parent, mut light, mut visibility) in &mut lights {
        if let Ok(emitter) = emitters.get(parent.parent()) {
            let scale = pose.to_scale_rotation_translation().0.abs().max_element();
            let radius = emitter.radius() * scale;
            if light.radius != radius {
                light.radius = radius;
            }
        }
        if light.intensity != profile.lantern_lumens {
            light.intensity = profile.lantern_lumens;
        }
        light.shadow_maps_enabled = false;
        light.contact_shadows_enabled = false;
        visibility.set_if_neq(Visibility::Inherited);
    }
}
