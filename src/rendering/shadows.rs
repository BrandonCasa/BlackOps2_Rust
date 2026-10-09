use super::{GameCamera, LanternLight, profile::VisualProfile};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub(super) fn select_local_lights(
    camera: Query<&GlobalTransform, With<GameCamera>>,
    mut lights: Query<
        (Entity, &GlobalTransform, &mut PointLight, &mut Visibility),
        With<LanternLight>,
    >,
    profile: Res<VisualProfile>,
    time: Res<Time<Real>>,
    mut weights: Local<HashMap<Entity, f32>>,
) {
    let Ok(camera) = camera.single() else { return };
    let eye = camera.translation();
    let mut ranked: Vec<_> = lights
        .iter()
        .filter_map(|(entity, pose, _, _)| {
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
    for (entity, pose, mut light, mut visibility) in &mut lights {
        let weight = weights.get(&entity).copied().unwrap_or(0.);
        let distance = pose.translation().distance(eye);
        let fade = ((profile.local_light_distance - distance) / 6.).clamp(0., 1.);
        let intensity = profile.lantern_lumens * weight * fade * fade * (3. - 2. * fade);
        if light.intensity != intensity {
            light.intensity = intensity;
        }
        let enabled = intensity > 0.;
        if light.shadow_maps_enabled != enabled {
            light.shadow_maps_enabled = enabled;
        }
        visibility.set_if_neq(if enabled {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}
