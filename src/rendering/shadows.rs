use super::{GameCamera, LanternLight, profile::VisualProfile};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub(super) fn restrict_contact_shadows(
    mut commands: Commands,
    cameras: Query<
        Entity,
        (
            With<bevy::pbr::ContactShadows>,
            Without<crate::presentation::ViewModelCamera>,
        ),
    >,
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
    camera: Query<&GlobalTransform, With<GameCamera>>,
    mut lights: Query<
        (
            Entity,
            &GlobalTransform,
            &ChildOf,
            &mut PointLight,
            &mut Visibility,
        ),
        With<LanternLight>,
    >,
    profile: Res<VisualProfile>,
    emitters: Query<&super::FixtureEmitter>,
    time: Res<Time<Real>>,
    mut weights: Local<HashMap<Entity, f32>>,
    mut shadowed: Local<HashSet<Entity>>,
) {
    let Ok(camera) = camera.single() else { return };
    let eye = camera.translation();
    let mut ranked: Vec<_> = lights
        .iter()
        .filter_map(|(entity, pose, _, _, _)| {
            let distance = pose.translation().distance(eye);
            let retained = weights.get(&entity).is_some_and(|weight| *weight > 0.);
            (distance < profile.local_light_distance)
                .then_some((entity, distance * if retained { 0.8 } else { 1. }))
        })
        .collect();
    ranked.sort_by(|a, b| {
        a.1.total_cmp(&b.1)
            .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
    });
    let selected: HashSet<_> = ranked
        .iter()
        .take(profile.max_local_lights)
        .map(|(entity, _)| *entity)
        .collect();
    weights.retain(|entity, _| lights.contains(*entity));
    let step = time.delta_secs() / profile.local_light_fade_seconds.max(0.01);
    for (entity, weight) in weights.iter_mut() {
        if !selected.contains(entity) {
            *weight = (*weight - step).max(0.);
        }
    }
    weights.retain(|_, weight| *weight > 0.);
    let mut slots = profile.max_local_lights.saturating_sub(weights.len());
    for (entity, _) in ranked.iter().take(profile.max_local_lights) {
        if let Some(weight) = weights.get_mut(entity) {
            *weight = (*weight + step).min(1.);
        } else if slots > 0 {
            weights.insert(*entity, step.min(1.));
            slots -= 1;
        }
    }
    shadowed.retain(|entity| {
        lights.get(*entity).is_ok_and(|(_, pose, _, _, _)| {
            weights.get(entity).is_some_and(|weight| *weight > 0.)
                && pose.translation().distance(eye) < profile.local_shadow_retention_distance
        })
    });
    let mut shadow_ranked: Vec<_> = lights
        .iter()
        .filter_map(|(entity, pose, _, _, _)| {
            let distance = pose.translation().distance(eye);
            (weights.get(&entity).is_some_and(|weight| *weight > 0.)
                && (shadowed.contains(&entity)
                    || distance < profile.local_shadow_admission_distance))
                .then_some((entity, distance, shadowed.contains(&entity)))
        })
        .collect();
    shadow_ranked.sort_by(|a, b| {
        (a.1 * if a.2 { 0.8 } else { 1.0 })
            .total_cmp(&(b.1 * if b.2 { 0.8 } else { 1.0 }))
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| a.0.to_bits().cmp(&b.0.to_bits()))
    });
    shadowed.clear();
    shadowed.extend(
        shadow_ranked
            .iter()
            .take(profile.max_local_shadows)
            .map(|entry| entry.0),
    );
    for (entity, pose, parent, mut light, mut visibility) in &mut lights {
        if let Ok(emitter) = emitters.get(parent.parent()) {
            let scale = pose.to_scale_rotation_translation().0.abs().max_element();
            let radius = emitter.radius() * scale;
            let near_z = 0.01 * scale;
            if light.radius != radius {
                light.radius = radius;
            }
            if light.shadow_map_near_z != near_z {
                light.shadow_map_near_z = near_z;
            }
        }
        let weight = weights.get(&entity).copied().unwrap_or(0.);
        let distance = pose.translation().distance(eye);
        let fade = ((profile.local_light_distance - distance) / 6.).clamp(0., 1.);
        let intensity = profile.lantern_lumens * weight * fade * fade * (3. - 2. * fade);
        if light.intensity != intensity {
            light.intensity = intensity;
        }
        let enabled = intensity > 0.;
        let cast_shadows = enabled && shadowed.contains(&entity);
        if light.shadow_maps_enabled != cast_shadows {
            light.shadow_maps_enabled = cast_shadows;
        }
        visibility.set_if_neq(if enabled {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
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
    let cameras: Vec<_> = world.query::<(Entity, &Camera, Has<super::GameCamera>, Has<crate::presentation::ViewModelCamera>, Option<&bevy::pbr::ContactShadows>, &Msaa, Option<&bevy::light::ShadowFilteringMethod>)>()
        .iter(world).map(|(entity, camera, game, first, contact, msaa, filter)| json!({
            "entity":entity.to_bits(), "active":camera.is_active, "world":game, "viewmodel":first,
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
            "shadow_maps":light.shadow_maps_enabled,"contact_shadows":light.contact_shadows_enabled,"soft_shadows":light.soft_shadows_enabled,"intensity":light.intensity
        })).collect();
    let surfaces: Vec<_> = world.query::<(Entity,&GltfMaterialName,&GltfMaterialExtras,Has<NotShadowCaster>,Has<super::FirstPersonSurface>,&GlobalTransform)>()
        .iter(world).map(|(entity,name,extras,excluded,first,pose)| {
            let value: serde_json::Value = serde_json::from_str(&extras.value).unwrap_or_default();
            json!({"entity":entity.to_bits(),"material":name.0,"excluded":excluded,"expected_excluded":excludes_caster(&value["surface"]),
                "viewmodel":first,"alpha":value["surface"]["presentation"]["alpha"],"position":pose.translation().to_array()})
        }).collect();
    json!({"cameras":cameras,"lights":lights,"surfaces":surfaces})
}
