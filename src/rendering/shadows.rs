use super::{LanternLight, profile::VisualProfile};
use bevy::prelude::*;
use std::collections::HashMap;

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

#[cfg(feature = "dev-dynamic")]
pub(crate) fn snapshot(world: &mut World) -> serde_json::Value {
    use bevy::{
        camera::primitives::Aabb,
        gltf::{GltfMaterialExtras, GltfMaterialName},
        light::NotShadowCaster,
    };
    use serde_json::json;
    let cameras: Vec<_> = world.query::<(Entity, &Camera, Has<super::GameCamera>, Option<&bevy::pbr::ContactShadows>, &Msaa, Option<&bevy::light::ShadowFilteringMethod>)>()
        .iter(world).map(|(entity, camera, game, contact, msaa, filter)| json!({
            "entity":entity.to_bits(), "active":camera.is_active, "world":game,
            "contact_shadows":contact.is_some(), "contact_settings":contact.map(|c|json!({"steps":c.linear_steps,"length":c.length,"thickness":c.thickness})),
            "samples":msaa.samples(), "filter":format!("{filter:?}")
        })).collect();
    let fixtures: HashMap<_, _> = world.query::<(Entity, &super::FixtureEmitter, &GlobalTransform, &Aabb)>()
        .iter(world).map(|(entity, emitter, pose, bounds)| (entity, json!({
            "model":emitter.0, "base_radius":emitter.radius(),
            "scale":pose.to_scale_rotation_translation().0.abs().max_element(),
            "position":pose.transform_point(Vec3::from(bounds.center)).to_array(),
            "bounds_min":(Vec3::from(bounds.center)-Vec3::from(bounds.half_extents)).to_array(),
            "bounds_max":(Vec3::from(bounds.center)+Vec3::from(bounds.half_extents)).to_array()
        }))).collect();
    let lights: Vec<_> = world.query::<(Entity, &PointLight, &GlobalTransform, &ChildOf, Has<LanternLight>)>()
        .iter(world).filter(|(_,_,_,_,lantern)| *lantern).map(|(entity,light,pose,parent,_)| json!({
            "entity":entity.to_bits(), "fixture":fixtures.get(&parent.parent()), "position":pose.translation().to_array(),
            "radius":light.radius,"near_z":light.shadow_map_near_z,"depth_bias":light.shadow_depth_bias,"normal_bias":light.shadow_normal_bias,
            "shadow_maps":light.shadow_maps_enabled,"contact_shadows":light.contact_shadows_enabled,"intensity":light.intensity
        })).collect();
    let surfaces: Vec<_> = world.query::<(Entity,&GltfMaterialName,&GltfMaterialExtras,Has<NotShadowCaster>,Has<super::FirstPersonSurface>,&GlobalTransform)>()
        .iter(world).map(|(entity,name,extras,excluded,first,pose)| {
            let value: serde_json::Value = serde_json::from_str(&extras.value).unwrap_or_default();
            json!({"entity":entity.to_bits(),"material":name.0,"excluded":excluded,"expected_excluded":excludes_caster(&value["surface"]),
                "viewmodel":first,"alpha":value["surface"]["presentation"]["alpha"],"position":pose.translation().to_array()})
        }).collect();
    json!({"cameras":cameras,"lights":lights,"surfaces":surfaces})
}
