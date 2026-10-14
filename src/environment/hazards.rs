use super::triggers::{MapTrigger, TRIGGER_LAYER};
use crate::{
    access::ZoneAccess,
    damage::{DamageQueue, Impact, ImpactSource},
    life::{LifeState, Vitality},
    origins::{DamageCategory, ZoneId},
    player::Player,
    weapons::HitRegion,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Component)]
pub struct DamageVolume {
    pub amount: u32,
    pub interval: f32,
    pub category: DamageCategory,
}

#[derive(Component)]
pub enum BoundaryVolume {
    Playable(ZoneId),
    Safe,
    Lethal,
}

#[derive(Component)]
pub(super) struct NativeKillBrush;

#[derive(Component)]
pub struct BoundaryMonitor {
    remaining: f32,
    condemned: bool,
}
impl Default for BoundaryMonitor {
    fn default() -> Self {
        Self {
            remaining: 3.,
            condemned: false,
        }
    }
}

pub(super) fn boundary(trigger: &super::triggers::TriggerDefinition) -> Option<BoundaryVolume> {
    match trigger.source.get("script_noteworthy").map(String::as_str) {
        Some("player_volume") => trigger
            .source
            .get("targetname")
            .and_then(|name| name.strip_prefix("zone_"))
            .map(|name| BoundaryVolume::Playable(ZoneId(format!("zone.{name}")))),
        Some("life_brush") => Some(BoundaryVolume::Safe),
        Some("kill_brush") => Some(BoundaryVolume::Lethal),
        _ => None,
    }
}

pub(super) fn spawn(mut commands: Commands, existing: Query<(), With<NativeKillBrush>>) {
    if !existing.is_empty() {
        return;
    }
    for (position, size) in [
        ([1643., 2168., 96.], [256., 1200., 512.]),
        ([-1277., 892., 184.], [148., 88., 128.]),
    ] {
        commands.spawn((
            NativeKillBrush,
            crate::gameplay::SessionEntity,
            BoundaryVolume::Lethal,
            Collider::cuboid(size[0] * 0.0254, size[2] * 0.0254, size[1] * 0.0254),
            Sensor,
            CollisionLayers::new(TRIGGER_LAYER, LayerMask::NONE),
            Transform::from_translation(Vec3::new(position[0], position[2], -position[1]) * 0.0254),
        ));
    }
}

pub(super) fn boundaries(
    time: Res<Time<Fixed>>,
    spatial: SpatialQuery,
    access: Res<ZoneAccess>,
    volumes: Query<(&BoundaryVolume, Option<&super::triggers::TriggerState>)>,
    mut players: Query<
        (
            Entity,
            &Transform,
            &Collider,
            &Vitality,
            &mut BoundaryMonitor,
        ),
        With<Player>,
    >,
    mut damage: ResMut<DamageQueue>,
) {
    for (owner, pose, shape, health, mut monitor) in &mut players {
        if !matches!(health.state(), LifeState::Standing | LifeState::Downed) {
            continue;
        }
        monitor.remaining -= time.delta_secs();
        if monitor.remaining > 0. {
            continue;
        }
        if monitor.condemned {
            damage.0.push(Impact {
                attacker: owner,
                target: owner,
                source: ImpactSource::OutOfBounds,
                category: DamageCategory::Environmental,
                region: HitRegion::Torso,
                amount: health.maximum(),
            });
            monitor.condemned = false;
            monitor.remaining = 3.;
            continue;
        }
        let mut safe = false;
        let mut playable = false;
        let mut lethal = false;
        for entity in spatial.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::from_mask(TRIGGER_LAYER),
        ) {
            match volumes.get(entity) {
                Ok((_, Some(state))) if !state.enabled => (),
                Ok((BoundaryVolume::Safe, _)) => safe = true,
                Ok((BoundaryVolume::Lethal, _)) => lethal = true,
                Ok((BoundaryVolume::Playable(zone), _)) => {
                    playable |= access.reachable.contains(zone)
                }
                Err(_) => (),
            }
        }
        monitor.condemned = !safe && (lethal || !playable);
        monitor.remaining = if monitor.condemned { 0.5 } else { 3. };
    }
}

pub(super) fn damage(
    time: Res<Time<Fixed>>,
    spatial: SpatialQuery,
    volumes: Query<(
        &DamageVolume,
        Option<&MapTrigger>,
        Option<&super::triggers::TriggerState>,
    )>,
    players: Query<(Entity, &Transform, &Collider, &Vitality), With<Player>>,
    mut clocks: Local<HashMap<(Entity, Entity), f32>>,
    mut queue: ResMut<DamageQueue>,
) {
    if volumes.is_empty() {
        clocks.clear();
        return;
    }
    let mut active = HashMap::new();
    for (owner, pose, shape, health) in &players {
        if health.state() != LifeState::Standing {
            continue;
        }
        for entity in spatial.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::from_mask(TRIGGER_LAYER),
        ) {
            let Ok((volume, id, state)) = volumes.get(entity) else {
                continue;
            };
            if state.is_some_and(|state| !state.enabled)
                || volume.amount == 0
                || volume.amount > i32::MAX as u32
                || !volume.interval.is_finite()
                || volume.interval <= 0.
            {
                continue;
            }
            let clock = clocks.get(&(owner, entity)).copied().unwrap_or(0.) - time.delta_secs();
            if clock <= 0. {
                queue.0.push(Impact {
                    attacker: owner,
                    target: owner,
                    source: ImpactSource::Environment(
                        id.map_or_else(|| "hazard.environment".into(), |id| id.0.clone()),
                    ),
                    category: volume.category,
                    region: HitRegion::Torso,
                    amount: volume.amount,
                });
            }
            active.insert(
                (owner, entity),
                if clock <= 0. {
                    volume.interval - (-clock).rem_euclid(volume.interval)
                } else {
                    clock
                },
            );
        }
    }
    *clocks = active;
}
