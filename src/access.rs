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
    pub flag_links: Vec<ZoneFlagLinkDefinition>,
    pub trigger_activations: Vec<TriggerActivationDefinition>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZoneFlagLinkDefinition {
    pub flag: String,
    pub connections: Vec<ContentId>,
}
#[derive(Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TriggerActivationDefinition {
    pub trigger: String,
    pub flags: Vec<String>,
    pub contents: u32,
}
#[derive(Component, Default)]
pub struct TriggerClientMode(pub i32);

pub(crate) fn synchronize_client_modes(
    mut players: Query<(&crate::life::Vitality, &mut TriggerClientMode), With<Player>>,
) {
    for (life, mut mode) in &mut players {
        mode.0 = match life.state() {
            crate::life::LifeState::Standing => 0,
            crate::life::LifeState::Downed => 7,
            _ => 10,
        };
    }
}
#[derive(Message)]
pub struct ZoneFlagSet(pub String);
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
    pub fn valid(
        &self,
        content: &ContentCatalog,
        environment: &crate::environment::EnvironmentDefinition,
    ) -> bool {
        self.flag_links.iter().all(|link| {
            !link.flag.is_empty()
                && !link.connections.is_empty()
                && link.connections.iter().all(|id| {
                    content
                        .zone_connections
                        .iter()
                        .any(|connection| connection.id == *id)
                })
        }) && self
            .flag_links
            .iter()
            .map(|link| &link.flag)
            .collect::<BTreeSet<_>>()
            .len()
            == self.flag_links.len()
            && self.trigger_activations.iter().all(|activation| {
                !activation.trigger.is_empty()
                    && !activation.flags.is_empty()
                    && activation.contents & 0x40000000 != 0
                    && activation
                        .flags
                        .iter()
                        .all(|flag| self.flag_links.iter().any(|link| link.flag == *flag))
                    && environment.triggers.iter().any(|trigger| {
                        trigger.id == activation.trigger
                            && trigger.source.get("classname").is_some_and(|class| {
                                matches!(
                                    class.as_str(),
                                    "trigger_box"
                                        | "trigger_radius"
                                        | "trigger_multiple"
                                        | "trigger_once"
                                )
                            })
                            && crate::environment::triggers::TriggerState::from_definition(trigger)
                                .contents
                                == activation.contents
                            && trigger.collider().is_some()
                    })
            })
            && self
                .trigger_activations
                .iter()
                .map(|activation| &activation.trigger)
                .collect::<BTreeSet<_>>()
                .len()
                == self.trigger_activations.len()
            && self.volumes.len() <= 4096
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
    pub activated: BTreeSet<ZoneId>,
    pub purchases: u32,
    pub flags: BTreeSet<String>,
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
            .chain(self.activated.iter().cloned())
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
#[derive(Resource, Default)]
pub struct PlayerZoneContacts(pub BTreeSet<ZoneId>);

fn clear_removed_player_contacts(
    players: Query<(), With<Player>>,
    mut zone: ResMut<CurrentZone>,
    mut contacts: ResMut<PlayerZoneContacts>,
) {
    if players.is_empty() {
        zone.0 = None;
        contacts.0.clear();
    }
}
#[derive(Component, Default)]
struct ZoneCheckpoint(Option<Vec3>);
const ZONE_SENSOR_LAYER: u32 = 8;
#[derive(Component)]
struct ZoneSensor {
    zone: ZoneId,
}
fn locate(
    query: SpatialQuery,
    mut players: Query<(&mut Transform, &mut Player, &mut ZoneCheckpoint, &Collider)>,
    volumes: Query<&ZoneSensor, Without<Player>>,
    mut zone: ResMut<CurrentZone>,
    mut contacts: ResMut<PlayerZoneContacts>,
    access: Res<ZoneAccess>,
    game: Res<GameDefinition>,
) {
    let Ok((mut pose, mut player, mut checkpoint, shape)) = players.single_mut() else {
        zone.0 = None;
        contacts.0.clear();
        return;
    };
    let filter = SpatialQueryFilter::from_mask(ZONE_SENSOR_LAYER);
    let touching: BTreeSet<_> = query
        .shape_intersections(shape, pose.translation, pose.rotation, &filter)
        .into_iter()
        .filter_map(|e| volumes.get(e).ok())
        .map(|sensor| sensor.zone.clone())
        .collect();
    let candidate = game
        .content
        .zones
        .iter()
        .find(|zone| touching.contains(&zone.id) && access.reachable.contains(&zone.id))
        .map(|zone| zone.id.clone());
    if candidate.is_none() && !touching.is_empty() {
        pose.translation = checkpoint
            .0
            .unwrap_or(Vec3::from_array(game.spawn) + Vec3::Y * (player.height * 0.5));
        player.velocity = Vec3::ZERO;
        return;
    }
    checkpoint.0 = Some(pose.translation);
    contacts.0 = touching;
    zone.0 = candidate;
}
pub struct AccessPlugin;
impl Plugin for AccessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneAccess>()
            .register_required_components::<Player, ZoneCheckpoint>()
            .register_required_components::<Player, TriggerClientMode>()
            .add_message::<ZoneFlagSet>()
            .add_observer(activate_triggers)
            .init_resource::<CurrentZone>()
            .init_resource::<PlayerZoneContacts>()
            .init_resource::<DoorEvidence>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                assemble.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                Update,
                (
                    audio_evidence,
                    synchronize_client_modes,
                    clear_removed_player_contacts,
                ),
            )
            .add_systems(
                FixedUpdate,
                synchronize_client_modes
                    .in_set(GameStep::Movement)
                    .after(crate::player::move_player)
                    .before(crate::environment::triggers::contacts)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (apply_flags, locate)
                    .chain()
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
fn activate_triggers(
    event: On<crate::environment::triggers::TriggerActivated>,
    mut commands: Commands,
    game: Res<GameDefinition>,
    triggers: Query<&crate::environment::triggers::MapTrigger>,
    mut flags: MessageWriter<ZoneFlagSet>,
) {
    let Ok(trigger) = triggers.get(event.trigger) else {
        return;
    };
    let Some(activation) = game
        .access
        .trigger_activations
        .iter()
        .find(|activation| activation.trigger == trigger.0)
    else {
        return;
    };
    for flag in &activation.flags {
        flags.write(ZoneFlagSet(flag.clone()));
    }
    commands.entity(event.trigger).despawn();
}
pub(crate) fn apply_flags(
    mut flags: MessageReader<ZoneFlagSet>,
    game: Res<GameDefinition>,
    mut access: ResMut<ZoneAccess>,
) {
    let mut changed = false;
    for ZoneFlagSet(flag) in flags.read() {
        let Some(link) = game
            .access
            .flag_links
            .iter()
            .find(|link| link.flag == *flag)
        else {
            continue;
        };
        access.flags.insert(flag.clone());
        for connection in &link.connections {
            changed |= access.open.insert(connection.clone());
        }
    }
    if changed {
        access.refresh(&game.content);
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
    mut contacts: ResMut<PlayerZoneContacts>,
) {
    if !players.is_empty() {
        return;
    }
    zone.0 = None;
    contacts.0.clear();
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
                    SessionEntity,
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
    if definition
        .access
        .flag_links
        .iter()
        .any(|link| link.flag == "always_on")
    {
        access.flags.insert("always_on".into());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::life::{PlayerLethalPolicy, ResolvedDamage, ReviveRestoration, Vitality};

    fn player() -> Player {
        Player {
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            height: 1.8,
            grounded: true,
            crouched: false,
            prone: false,
            eye_height: 1.62,
        }
    }

    #[test]
    fn client_contact_modes_follow_damage_revive_and_respawn() {
        let mut app = App::new();
        app.register_required_components::<Player, TriggerClientMode>()
            .init_resource::<CurrentZone>()
            .init_resource::<PlayerZoneContacts>()
            .add_systems(
                Update,
                (synchronize_client_modes, clear_removed_player_contacts),
            );
        let owner = app
            .world_mut()
            .spawn((
                player(),
                Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloSelfRevive).unwrap(),
            ))
            .id();
        let trigger = crate::environment::triggers::TriggerState::from_definition(
            &crate::environment::triggers::TriggerDefinition {
                id: "zone.sensor".into(),
                position: [0.; 3],
                target_position: None,
                source: std::collections::BTreeMap::from([
                    ("classname".into(), "trigger_box".into()),
                    ("spawnflags".into(), "1024".into()),
                ]),
                shape: crate::environment::triggers::TriggerShape::Convex { volumes: vec![] },
            },
        );
        app.update();
        assert_eq!(app.world().get::<TriggerClientMode>(owner).unwrap().0, 0);
        app.world_mut()
            .get_mut::<Vitality>(owner)
            .unwrap()
            .apply(ResolvedDamage::new(100).unwrap())
            .unwrap();
        app.update();
        let mode = app.world().get::<TriggerClientMode>(owner).unwrap().0;
        assert_eq!(mode, 7);
        assert!(trigger.accepts_player(mode));
        {
            let mut life = app.world_mut().get_mut::<Vitality>(owner).unwrap();
            let episode = life.downed_episode().unwrap();
            let token = life.begin_self_revive(owner, episode).unwrap();
            life.complete_self_revive(owner, token, ReviveRestoration::new(100, 100).unwrap())
                .unwrap();
        }
        app.update();
        assert_eq!(app.world().get::<TriggerClientMode>(owner).unwrap().0, 0);
        {
            let mut life = app.world_mut().get_mut::<Vitality>(owner).unwrap();
            life.apply(ResolvedDamage::new(100).unwrap()).unwrap();
            let expiry = life.downed_expiry(owner).unwrap();
            life.expire_downed(owner, expiry).unwrap();
        }
        app.update();
        let mode = app.world().get::<TriggerClientMode>(owner).unwrap().0;
        assert!(!trigger.accepts_player(mode));
        app.world_mut().resource_mut::<CurrentZone>().0 = Some(ZoneId("zone.start".into()));
        app.world_mut()
            .resource_mut::<PlayerZoneContacts>()
            .0
            .insert(ZoneId("zone.start".into()));
        app.world_mut().despawn(owner);
        app.update();
        assert!(app.world().resource::<CurrentZone>().0.is_none());
        assert!(app.world().resource::<PlayerZoneContacts>().0.is_empty());
        let replacement = app
            .world_mut()
            .spawn((
                player(),
                Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloSelfRevive).unwrap(),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<TriggerClientMode>(replacement).unwrap().0,
            0
        );
    }
}
