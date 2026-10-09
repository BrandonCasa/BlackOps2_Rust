use crate::{
    GameState,
    assets::GameDefinition,
    environment::VolumeDefinition,
    gameplay::{GameStep, SessionEntity},
    origins::content::{ContentCatalog, ContentId, ZoneId},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone, Default, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AccessDefinition {
    pub doors: Vec<DoorDefinition>,
    pub volumes: Vec<ZoneVolumeDefinition>,
    pub spawns: Vec<SpawnDefinition>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneVolumeDefinition {
    pub zone: ZoneId,
    pub volume: VolumeDefinition,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpawnDefinition {
    pub id: String,
    pub zone: ZoneId,
    pub position: [f32; 3],
    pub rising: bool,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoorDefinition {
    pub connection: ContentId,
    pub connections: Vec<ContentId>,
    pub debris: bool,
    pub seconds: f32,
    pub props: Vec<DoorVisual>,
    pub blocks: Vec<VolumeDefinition>,
    pub sound: ContentId,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DoorVisual {
    pub presentation: ContentId,
    pub position: [f32; 3],
    pub yaw: f32,
    pub offset: [f32; 3],
}
impl AccessDefinition {
    pub fn valid(&self, content: &ContentCatalog) -> bool {
        self.volumes.len() <= 4096
            && self.volumes.iter().all(|v| {
                content.zones.iter().any(|z| z.id == v.zone)
                    && v.volume.position.iter().all(|x| x.is_finite())
                    && (4..=128).contains(&v.volume.vertices.len())
                    && v.volume.vertices.iter().flatten().all(|x| x.is_finite())
                    && Collider::convex_hull(
                        &v.volume
                            .vertices
                            .iter()
                            .copied()
                            .map(Vec3::from_array)
                            .collect::<Vec<_>>(),
                    )
                    .is_some()
            })
            && self.spawns.len() <= 4096
            && self.spawns.iter().all(|s| {
                !s.id.is_empty()
                    && s.id.len() < 128
                    && s.position.iter().all(|x| x.is_finite())
                    && content.zones.iter().any(|z| z.id == s.zone)
            })
            && self.doors.len() <= 256
            && self
                .doors
                .iter()
                .map(|d| &d.connection)
                .collect::<BTreeSet<_>>()
                .len()
                == self.doors.len()
            && self.doors.iter().all(|d| {
                d.seconds.is_finite()
                    && d.seconds > 0.
                    && d.seconds <= 10.
                    && !d.connections.is_empty()
                    && d.connections.contains(&d.connection)
                    && d.connections
                        .iter()
                        .all(|id| content.zone_connections.iter().any(|c| c.id == *id))
                    && content.sounds.iter().any(|s| s.id == d.sound && !s.looping)
                    && !d.props.is_empty()
                    && d.props.len() <= 64
                    && d.props.iter().all(|p| {
                        p.yaw.is_finite()
                            && p.position.iter().chain(&p.offset).all(|v| v.is_finite())
                            && content
                                .presentations
                                .iter()
                                .any(|s| s.id == p.presentation && s.scene.is_some())
                    })
                    && !d.blocks.is_empty()
                    && d.blocks.len() <= 256
                    && d.blocks.iter().all(|v| {
                        v.position.iter().all(|x| x.is_finite())
                            && (4..=128).contains(&v.vertices.len())
                            && v.vertices.iter().flatten().all(|x| x.is_finite())
                            && Collider::convex_hull(
                                &v.vertices
                                    .iter()
                                    .copied()
                                    .map(Vec3::from_array)
                                    .collect::<Vec<_>>(),
                            )
                            .is_some()
                    })
            })
    }
}
#[derive(Resource, Default)]
pub struct ZoneAccess {
    pub open: BTreeSet<ContentId>,
    pub reachable: BTreeSet<ZoneId>,
    pub purchases: u32,
}
impl ZoneAccess {
    pub fn opened(&self, id: &ContentId) -> bool {
        self.open.contains(id)
    }
    pub fn refresh(&mut self, content: &ContentCatalog) {
        self.reachable = content
            .zones
            .iter()
            .filter(|z| z.initially_open)
            .map(|z| z.id.clone())
            .collect();
        loop {
            let before = self.reachable.len();
            for c in &content.zone_connections {
                if !self.open.contains(&c.id) {
                    continue;
                }
                if self.reachable.contains(&c.from) {
                    self.reachable.insert(c.to.clone());
                }
                if self.reachable.contains(&c.to) {
                    self.reachable.insert(c.from.clone());
                }
            }
            if self.reachable.len() == before {
                break;
            }
        }
    }
    pub fn unlock(&mut self, definition: &GameDefinition, id: &ContentId) -> Result<(), ()> {
        if self.opened(id) {
            return Err(());
        }
        let Some(door) = definition.access.doors.iter().find(|d| d.connection == *id) else {
            return Err(());
        };
        self.open.extend(door.connections.iter().cloned());
        self.purchases += 1;
        self.refresh(&definition.content);
        Ok(())
    }
}
#[derive(Component)]
pub struct DoorBlocker(pub ContentId);
#[derive(Component)]
pub(crate) struct DoorProp {
    pub(crate) connection: ContentId,
    start: Vec3,
    offset: Vec3,
    age: f32,
}
#[derive(Resource, Default)]
pub struct DoorEvidence {
    pub removed: u32,
    pub animated: u32,
    pub sounds: u32,
    pub playing_sounds: u32,
}
#[derive(Component)]
struct DoorAudio;
fn audio_evidence(
    sinks: Query<(), (With<DoorAudio>, Added<bevy::audio::SpatialAudioSink>)>,
    mut evidence: ResMut<DoorEvidence>,
) {
    evidence.playing_sounds += sinks.iter().count() as u32;
}
#[derive(Resource, Default)]
pub struct CurrentZone(pub Option<ZoneId>);
const ZONE_SENSOR_LAYER: u32 = 8;
#[derive(Component)]
struct ZoneSensor {
    zone: ZoneId,
}
fn locate(
    query: SpatialQuery,
    players: Query<&Transform, With<Player>>,
    volumes: Query<(&ZoneSensor, &Transform), Without<Player>>,
    mut zone: ResMut<CurrentZone>,
) {
    let Ok(pose) = players.single() else {
        zone.0 = None;
        return;
    };
    let filter = SpatialQueryFilter::from_mask(ZONE_SENSOR_LAYER);
    zone.0 = query
        .shape_intersections(
            &Collider::sphere(0.03),
            pose.translation,
            Quat::IDENTITY,
            &filter,
        )
        .into_iter()
        .filter_map(|e| volumes.get(e).ok())
        .min_by(|(_, a), (_, b)| {
            a.translation
                .distance_squared(pose.translation)
                .total_cmp(&b.translation.distance_squared(pose.translation))
        })
        .map(|(z, _)| z.zone.clone());
}
pub struct AccessPlugin;
impl Plugin for AccessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneAccess>()
            .init_resource::<CurrentZone>()
            .init_resource::<DoorEvidence>()
            .add_systems(
                OnEnter(GameState::Playing),
                assemble.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(Update, audio_evidence)
            .add_systems(
                FixedUpdate,
                locate
                    .in_set(GameStep::Movement)
                    .after(crate::player::move_player)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                opening
                    .in_set(GameStep::Interactions)
                    .after(crate::interactions::interact)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn assemble(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    server: Res<AssetServer>,
    players: Query<(), With<Player>>,
    mut access: ResMut<ZoneAccess>,
    mut evidence: ResMut<DoorEvidence>,
    volumes: Query<(), With<ZoneSensor>>,
    mut zone: ResMut<CurrentZone>,
) {
    if !players.is_empty() {
        return;
    }
    zone.0 = None;
    if volumes.is_empty() {
        for v in &definition.access.volumes {
            if let Some(shape) = Collider::convex_hull(
                &v.volume
                    .vertices
                    .iter()
                    .copied()
                    .map(Vec3::from_array)
                    .collect::<Vec<_>>(),
            ) {
                commands.spawn((
                    ZoneSensor {
                        zone: v.zone.clone(),
                    },
                    RigidBody::Static,
                    shape,
                    Sensor,
                    CollisionLayers::new(ZONE_SENSOR_LAYER, LayerMask::NONE),
                    Transform::from_translation(Vec3::from_array(v.volume.position)),
                ));
            }
        }
    }
    *evidence = default();
    *access = default();
    access.open.extend(
        definition
            .content
            .zone_connections
            .iter()
            .filter(|c| c.initially_open)
            .map(|c| c.id.clone()),
    );
    access.refresh(&definition.content);
    for door in &definition.access.doors {
        for v in &door.blocks {
            if let Some(shape) = Collider::convex_hull(
                &v.vertices
                    .iter()
                    .copied()
                    .map(Vec3::from_array)
                    .collect::<Vec<_>>(),
            ) {
                commands.spawn((
                    SessionEntity,
                    DoorBlocker(door.connection.clone()),
                    RigidBody::Static,
                    shape,
                    Transform::from_translation(Vec3::from_array(v.position)),
                ));
            }
        }
        for p in &door.props {
            let scene = definition
                .content
                .presentations
                .iter()
                .find(|s| s.id == p.presentation)
                .and_then(|s| s.scene.as_ref());
            if let Some(scene) = scene {
                commands.spawn((
                    SessionEntity,
                    DoorProp {
                        connection: door.connection.clone(),
                        start: Vec3::from_array(p.position),
                        offset: Vec3::from_array(p.offset),
                        age: 0.,
                    },
                    WorldAssetRoot(server.load(scene.clone())),
                    Transform::from_translation(Vec3::from_array(p.position))
                        .with_rotation(Quat::from_rotation_y(p.yaw)),
                    Visibility::Inherited,
                ));
            }
        }
    }
}
fn opening(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    access: Res<ZoneAccess>,
    definition: Res<GameDefinition>,
    blockers: Query<(Entity, &DoorBlocker)>,
    mut props: Query<(Entity, &mut DoorProp, &mut Transform)>,
    mut evidence: ResMut<DoorEvidence>,
) {
    for (entity, block) in &blockers {
        if access.opened(&block.0) {
            commands.entity(entity).despawn();
            evidence.removed += 1;
        }
    }
    for (entity, mut prop, mut transform) in &mut props {
        if !access.opened(&prop.connection) {
            continue;
        }
        let Some(door) = definition
            .access
            .doors
            .iter()
            .find(|d| d.connection == prop.connection)
        else {
            continue;
        };
        if prop.age == 0. {
            if let Some(alias) = definition.audio.bindings.get(&door.sound.0) {
                let source = if door.debris {
                    crate::audio::SoundSource::World(prop.start)
                } else {
                    crate::audio::SoundSource::attached(entity)
                };
                let entity = crate::audio::emit(&mut commands, alias.clone(), source);
                commands.entity(entity).insert(DoorAudio);
                evidence.sounds += 1;
            }
        }
        prop.age += time.delta_secs();
        let t = (prop.age / door.seconds).clamp(0., 1.);
        let ease = if t < 0.25 {
            t * t * (8. / 3.)
        } else if t > 0.75 {
            1. - (1. - t).powi(2) * (8. / 3.)
        } else {
            (t - 0.125) * (4. / 3.)
        };
        transform.translation = prop.start + prop.offset * ease;
        if door.debris || t >= 1. {
            commands.entity(entity).despawn();
            evidence.animated += 1;
        }
    }
}
