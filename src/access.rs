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
#[serde(default)]
pub struct AccessDefinition {
    pub doors: Vec<DoorDefinition>,
    pub volumes: Vec<ZoneVolumeDefinition>,
    pub spawns: Vec<SpawnDefinition>,
    pub spawners: Vec<ActorSpawnerDefinition>,
    pub entrances: Vec<SpawnEntranceDefinition>,
    pub flag_links: Vec<ZoneFlagLinkDefinition>,
    pub trigger_activations: Vec<TriggerActivationDefinition>,
}
#[derive(Clone, serde::Deserialize)]
pub struct ZoneFlagLinkDefinition {
    pub flag: String,
    pub connections: Vec<ContentId>,
}
#[derive(Clone, serde::Deserialize)]
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
#[derive(Message, Clone, Debug, PartialEq, Eq)]
pub struct NewZoneActive(pub ZoneId);
#[derive(Clone, serde::Deserialize)]
pub struct ZoneVolumeDefinition {
    pub zone: ZoneId,
    pub volume: VolumeDefinition,
}
#[derive(Clone, serde::Deserialize)]
pub struct SpawnDefinition {
    pub id: String,
    pub zone: ZoneId,
    pub position: [f32; 3],
    pub rising: bool,
    #[serde(default)]
    pub find_flesh: bool,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub desired_origin: Option<[f32; 3]>,
}
#[derive(Clone, serde::Deserialize)]
pub struct ActorSpawnerDefinition {
    pub id: String,
    pub archetype: String,
    pub position: [f32; 3],
    pub yaw: f32,
    pub desired_origin: Option<[f32; 3]>,
}
#[derive(Clone, serde::Deserialize)]
pub struct SpawnEntranceDefinition {
    pub node: ContentId,
    pub position: [f32; 3],
}
#[derive(Clone, serde::Deserialize)]
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
pub struct DoorVisual {
    pub presentation: ContentId,
    pub position: [f32; 3],
    pub yaw: f32,
    pub offset: [f32; 3],
}
#[derive(Resource, Default)]
pub struct ZoneAccess {
    pub open: BTreeSet<ContentId>,
    pub reachable: BTreeSet<ZoneId>,
    pub activated: BTreeSet<ZoneId>,
    pub purchases: u32,
    pub flags: BTreeSet<String>,
    pub occupied: BTreeSet<ZoneId>,
    pub active: BTreeSet<ZoneId>,
}
impl ZoneAccess {
    pub fn opened(&self, id: &ContentId) -> bool {
        self.open.contains(id)
    }
    pub(crate) fn set_flag(
        &mut self,
        content: &ContentCatalog,
        definition: &AccessDefinition,
        flag: &str,
    ) -> bool {
        if !self.flags.insert(flag.to_owned()) {
            return false;
        }
        let mut changed = false;
        for link in definition
            .flag_links
            .iter()
            .filter(|link| link.flag == flag)
        {
            for connection in &link.connections {
                changed |= self.open.insert(connection.clone());
            }
        }
        if changed {
            self.refresh(content);
        }
        true
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

#[derive(Resource)]
pub(crate) struct ZoneScan {
    timer: Timer,
    cursor: usize,
    choke: usize,
    previous: Option<ZoneId>,
    occupied: BTreeSet<ZoneId>,
    active: BTreeSet<ZoneId>,
}
impl Default for ZoneScan {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(0., TimerMode::Once),
            cursor: 0,
            choke: 0,
            previous: None,
            occupied: BTreeSet::new(),
            active: BTreeSet::new(),
        }
    }
}
impl ZoneScan {
    fn update(
        &mut self,
        delta: std::time::Duration,
        content: &ContentCatalog,
        contacts: &BTreeSet<ZoneId>,
        access: &mut ZoneAccess,
    ) -> (Vec<NewZoneActive>, bool) {
        self.timer.tick(delta);
        if !self.timer.is_finished() {
            return (Vec::new(), false);
        }
        if self.cursor == 0 {
            self.occupied.clear();
            self.active.clear();
        }
        let mut events = Vec::new();
        while let Some(zone) = content.zones.get(self.cursor) {
            self.cursor += 1;
            if !access.reachable.contains(&zone.id) {
                continue;
            }
            if contacts.contains(&zone.id) {
                self.occupied.insert(zone.id.clone());
                self.active.insert(zone.id.clone());
                if self.previous.as_ref() != Some(&zone.id) {
                    events.push(NewZoneActive(zone.id.clone()));
                    self.previous = Some(zone.id.clone());
                }
                for connection in &content.zone_connections {
                    if !access.open.contains(&connection.id) {
                        continue;
                    }
                    let adjacent = if connection.from == zone.id {
                        Some(&connection.to)
                    } else if connection.to == zone.id {
                        Some(&connection.from)
                    } else {
                        None
                    };
                    if let Some(adjacent) = adjacent
                        && access.reachable.contains(adjacent)
                    {
                        self.active.insert(adjacent.clone());
                    }
                }
            }
            self.choke += 1;
            if self.choke == 3 {
                self.choke = 0;
                self.timer = Timer::new(std::time::Duration::from_millis(50), TimerMode::Once);
                return (events, false);
            }
        }
        if self.active.is_empty()
            && let Some(initial) = content.zones.iter().find(|zone| zone.initially_open)
        {
            self.active.insert(initial.id.clone());
            self.occupied.insert(initial.id.clone());
        }
        let changed = access.active != self.active || access.occupied != self.occupied;
        if access.active != self.active {
            access.active.clone_from(&self.active);
        }
        if access.occupied != self.occupied {
            access.occupied.clone_from(&self.occupied);
        }
        self.cursor = 0;
        self.timer = Timer::from_seconds(1., TimerMode::Once);
        (events, changed)
    }
}

fn scan_zones(
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    contacts: Res<PlayerZoneContacts>,
    mut scan: ResMut<ZoneScan>,
    mut access: ResMut<ZoneAccess>,
    mut active: MessageWriter<NewZoneActive>,
) {
    let (events, changed) = scan.update(
        time.delta(),
        &game.content,
        &contacts.0,
        access.bypass_change_detection(),
    );
    active.write_batch(events);
    if changed {
        access.set_changed();
    }
}

fn clear_removed_player_contacts(
    players: Query<&crate::life::Vitality, With<Player>>,
    mut zone: ResMut<CurrentZone>,
    mut contacts: ResMut<PlayerZoneContacts>,
) {
    if !players.iter().any(|life| {
        matches!(
            life.state(),
            crate::life::LifeState::Standing | crate::life::LifeState::Downed
        )
    }) {
        zone.0 = None;
        contacts.0.clear();
    }
}
const ZONE_SENSOR_LAYER: u32 = 8;
#[derive(Component)]
struct ZoneSensor {
    zone: ZoneId,
}
fn locate(
    query: SpatialQuery,
    players: Query<(&Transform, &Collider, &crate::life::Vitality), With<Player>>,
    volumes: Query<&ZoneSensor, Without<Player>>,
    mut zone: ResMut<CurrentZone>,
    mut contacts: ResMut<PlayerZoneContacts>,
    access: Res<ZoneAccess>,
    game: Res<GameDefinition>,
) {
    let Ok((pose, shape, life)) = players.single() else {
        zone.0 = None;
        contacts.0.clear();
        return;
    };
    if !matches!(
        life.state(),
        crate::life::LifeState::Standing | crate::life::LifeState::Downed
    ) {
        zone.0 = None;
        contacts.0.clear();
        return;
    }
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
    contacts.0 = touching;
    zone.0 = candidate;
}
pub struct AccessPlugin;
impl Plugin for AccessPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneAccess>()
            .register_required_components::<Player, TriggerClientMode>()
            .add_message::<ZoneFlagSet>()
            .add_message::<NewZoneActive>()
            .add_observer(activate_triggers)
            .init_resource::<CurrentZone>()
            .init_resource::<PlayerZoneContacts>()
            .init_resource::<ZoneScan>()
            .init_resource::<DoorEvidence>()
            .add_systems(
                OnEnter(GameState::StartingSession),
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
                synchronize_client_modes
                    .after(GameStep::PlayerLife)
                    .before(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (apply_flags, locate, scan_zones)
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
    mut access: ResMut<ZoneAccess>,
    mut triggers: Query<(
        &crate::environment::triggers::MapTrigger,
        &mut crate::environment::triggers::TriggerState,
    )>,
    mut flags: MessageWriter<ZoneFlagSet>,
) {
    let Ok((trigger, mut state)) = triggers.get_mut(event.trigger) else {
        return;
    };
    if !state.enabled {
        return;
    }
    let Some(activation) = game
        .access
        .trigger_activations
        .iter()
        .find(|activation| activation.trigger == trigger.0)
    else {
        return;
    };
    state.enabled = false;
    for flag in &activation.flags {
        access.set_flag(&game.content, &game.access, flag);
        flags.write(ZoneFlagSet(flag.clone()));
    }
    commands.entity(event.trigger).try_despawn();
}
pub(crate) fn apply_flags(
    mut flags: MessageReader<ZoneFlagSet>,
    game: Res<GameDefinition>,
    mut access: ResMut<ZoneAccess>,
) {
    for ZoneFlagSet(flag) in flags.read() {
        access.set_flag(&game.content, &game.access, flag);
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
    mut scan: ResMut<ZoneScan>,
) {
    if !players.is_empty() {
        return;
    }
    zone.0 = None;
    contacts.0.clear();
    *scan = default();
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
        let ease = if door.seconds < 0.5 {
            t
        } else if t < 0.25 {
            t * t * (8. / 3.)
        } else if t > 0.75 {
            1. - (1. - t).powi(2) * (8. / 3.)
        } else {
            (t - 0.125) * (4. / 3.)
        };
        transform.translation = prop.start + prop.offset * ease;
        if door.debris {
            commands.entity(entity).despawn();
            evidence.animated += 1;
        } else if t >= 1. {
            commands.entity(entity).remove::<DoorProp>();
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

    fn source_zones() -> (ContentCatalog, ZoneAccess) {
        let content: ContentCatalog = serde_json::from_value(serde_json::json!({
            "weapons":[],"enemies":[],"perks":[],"zones":[
                {"id":"zone.start","initially_open":true},
                {"id":"zone.start_a","initially_open":false},
                {"id":"zone.start_b","initially_open":false},
                {"id":"zone.deferred","initially_open":false}],
            "definitions":[],"placements":[],"interactions":[],"zone_connections":[
                {"id":"connection.start_a","from":"zone.start","to":"zone.start_a","initially_open":true},
                {"id":"connection.start_b","from":"zone.start","to":"zone.start_b","initially_open":true},
                {"id":"connection.a_b","from":"zone.start_a","to":"zone.start_b","initially_open":true}],
            "navigation_nodes":[],"navigation_links":[],"presentations":[],"sounds":[],"animations":[],"ui_assets":[]
        })).unwrap();
        let mut access = ZoneAccess::default();
        access.open.extend(
            content
                .zone_connections
                .iter()
                .map(|connection| connection.id.clone()),
        );
        access.refresh(&content);
        (content, access)
    }

    #[test]
    fn disabled_zone_sensor_never_rewinds_player_movement() {
        let (content, access) = source_zones();
        let mut game: GameDefinition =
            serde_json::from_str(include_str!("../gameplay/origins/game.json")).unwrap();
        game.content = content;
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            TransformPlugin,
            AssetPlugin::default(),
            PhysicsPlugins::new(PostUpdate),
        ))
        .init_asset::<Mesh>()
        .insert_resource(Time::<Fixed>::from_hz(60.))
        .insert_resource(bevy::time::TimeUpdateStrategy::FixedTimesteps(1))
        .insert_resource(game)
        .insert_resource(access)
        .init_resource::<CurrentZone>()
        .init_resource::<PlayerZoneContacts>();
        for (id, x) in [("zone.start", 0.), ("zone.deferred", 4.)] {
            app.world_mut().spawn((
                ZoneSensor {
                    zone: ZoneId(id.into()),
                },
                RigidBody::Static,
                Collider::cuboid(2., 4., 2.),
                Sensor,
                CollisionLayers::new(ZONE_SENSOR_LAYER, LayerMask::NONE),
                Transform::from_xyz(x, 0., 0.),
            ));
        }
        let owner = app
            .world_mut()
            .spawn((
                player(),
                Collider::cuboid(0.7, 1.8, 0.7),
                Transform::IDENTITY,
                Vitality::human(100, 100, 1, PlayerLethalPolicy::SoloSelfRevive).unwrap(),
            ))
            .id();
        app.finish();
        app.cleanup();
        app.update();
        app.update();
        app.world_mut().run_system_cached(locate).unwrap();
        assert_eq!(
            app.world().resource::<CurrentZone>().0,
            Some(ZoneId("zone.start".into()))
        );

        let destination = Vec3::new(4., 0., 0.);
        let velocity = Vec3::new(2., 0., 0.);
        app.world_mut()
            .get_mut::<Transform>(owner)
            .unwrap()
            .translation = destination;
        app.world_mut().get_mut::<Player>(owner).unwrap().velocity = velocity;
        app.update();
        app.world_mut().run_system_cached(locate).unwrap();
        assert_eq!(
            app.world().get::<Transform>(owner).unwrap().translation,
            destination
        );
        assert_eq!(app.world().get::<Player>(owner).unwrap().velocity, velocity);
        assert!(app.world().resource::<CurrentZone>().0.is_none());
        assert_eq!(
            app.world().resource::<PlayerZoneContacts>().0,
            BTreeSet::from([ZoneId("zone.deferred".into())])
        );
    }

    #[test]
    fn overlapping_source_zones_preserve_scan_order_choke_and_active_neighbors() {
        use std::time::Duration;
        let (content, mut access) = source_zones();
        let mut scan = ZoneScan::default();
        let start = ZoneId("zone.start".into());
        let a = ZoneId("zone.start_a".into());
        let touching = BTreeSet::from([start.clone(), a.clone()]);
        assert_eq!(
            scan.update(Duration::ZERO, &content, &touching, &mut access)
                .0,
            vec![NewZoneActive(start.clone()), NewZoneActive(a.clone())]
        );
        assert!(access.active.is_empty());
        assert!(
            scan.update(Duration::from_millis(49), &content, &touching, &mut access)
                .0
                .is_empty()
        );
        assert!(access.occupied.is_empty());
        assert!(
            scan.update(Duration::from_millis(1), &content, &touching, &mut access)
                .0
                .is_empty()
        );
        assert_eq!(access.occupied, touching);
        assert_eq!(access.active, access.reachable);
        assert!(
            scan.update(Duration::from_millis(999), &content, &touching, &mut access)
                .0
                .is_empty()
        );
        assert_eq!(
            scan.update(Duration::from_millis(1), &content, &touching, &mut access)
                .0,
            vec![NewZoneActive(start), NewZoneActive(a)]
        );
    }

    #[test]
    fn source_zone_scan_ignores_disabled_contacts_and_uses_initial_fallback() {
        use std::time::Duration;
        let (content, mut access) = source_zones();
        let mut scan = ZoneScan::default();
        let start = ZoneId("zone.start".into());
        let disabled = BTreeSet::from([ZoneId("zone.deferred".into())]);
        assert!(
            scan.update(Duration::ZERO, &content, &disabled, &mut access)
                .0
                .is_empty()
        );
        assert!(
            scan.update(Duration::from_millis(50), &content, &disabled, &mut access)
                .0
                .is_empty()
        );
        assert_eq!(access.active, BTreeSet::from([start.clone()]));
        assert_eq!(access.occupied, BTreeSet::from([start.clone()]));
        assert_eq!(
            scan.update(
                Duration::from_secs(1),
                &content,
                &BTreeSet::from([start.clone()]),
                &mut access
            )
            .0,
            vec![NewZoneActive(start.clone())]
        );
        scan.update(
            Duration::from_millis(50),
            &content,
            &BTreeSet::from([start.clone()]),
            &mut access,
        );
        assert_eq!(access.active, access.reachable);
        assert!(
            scan.update(
                Duration::from_secs(1),
                &content,
                &BTreeSet::from([start]),
                &mut access
            )
            .0
            .is_empty()
        );
    }

    #[test]
    fn source_zone_flags_open_grouped_connections_once() {
        let (content, mut access) = source_zones();
        access.open.clear();
        access.refresh(&content);
        let definition = AccessDefinition {
            flag_links: vec![
                ZoneFlagLinkDefinition {
                    flag: "source.activation".into(),
                    connections: vec![ContentId("connection.start_a".into())],
                },
                ZoneFlagLinkDefinition {
                    flag: "source.activation".into(),
                    connections: vec![ContentId("connection.start_b".into())],
                },
            ],
            ..default()
        };
        assert_eq!(access.reachable.len(), 1);
        assert!(access.set_flag(&content, &definition, "source.activation"));
        assert_eq!(access.reachable.len(), 3);
        assert_eq!(access.open.len(), 2);
        assert!(!access.set_flag(&content, &definition, "source.activation"));
        assert_eq!(access.flags.len(), 1);
        assert_eq!(access.purchases, 0);
    }

    #[test]
    fn zone_scan_reports_only_changes_to_published_access() {
        use std::time::Duration;
        let (content, mut access) = source_zones();
        let mut scan = ZoneScan::default();
        let contacts = BTreeSet::from([ZoneId("zone.start".into())]);
        assert!(
            !scan
                .update(Duration::ZERO, &content, &contacts, &mut access)
                .1
        );
        assert!(
            scan.update(Duration::from_millis(50), &content, &contacts, &mut access)
                .1
        );
        assert!(
            !scan
                .update(Duration::from_secs(1), &content, &contacts, &mut access)
                .1
        );
        assert!(
            !scan
                .update(Duration::from_millis(50), &content, &contacts, &mut access)
                .1
        );
    }

    #[test]
    fn opening_keeps_sliding_doors_and_removes_debris_and_blockers_once() {
        use bevy::ecs::system::RunSystemOnce;
        use std::time::Duration;
        let mut game: GameDefinition =
            serde_json::from_str(include_str!("../gameplay/origins/game.json")).unwrap();
        let sliding = ContentId("connection.sliding".into());
        let debris = ContentId("connection.debris".into());
        game.audio.bindings.clear();
        game.access.doors = [(&sliding, false), (&debris, true)]
            .map(|(connection, debris)| DoorDefinition {
                connection: connection.clone(),
                connections: vec![connection.clone()],
                debris,
                seconds: 1.,
                props: Vec::new(),
                blocks: Vec::new(),
                sound: ContentId("sound.door.test".into()),
            })
            .to_vec();
        let mut world = World::new();
        world.insert_resource(game);
        world.init_resource::<Time<Fixed>>();
        world.init_resource::<DoorEvidence>();
        world.insert_resource(ZoneAccess {
            open: BTreeSet::from([sliding.clone(), debris.clone()]),
            ..default()
        });
        let spawn_prop = |world: &mut World, connection| {
            world
                .spawn((
                    DoorProp {
                        connection,
                        start: Vec3::ZERO,
                        offset: Vec3::X * 2.,
                        age: 0.,
                    },
                    Transform::default(),
                ))
                .id()
        };
        let door = spawn_prop(&mut world, sliding.clone());
        let rubble = spawn_prop(&mut world, debris);
        let blocker = world.spawn(DoorBlocker(sliding)).id();
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_millis(500));
        world.run_system_once(opening).unwrap();
        assert_eq!(world.get::<Transform>(door).unwrap().translation, Vec3::X);
        assert!(world.get_entity(rubble).is_err());
        assert!(world.get_entity(blocker).is_err());
        world.run_system_once(opening).unwrap();
        assert_eq!(
            world.get::<Transform>(door).unwrap().translation,
            Vec3::X * 2.
        );
        assert!(world.get::<DoorProp>(door).is_none());
        world.run_system_once(opening).unwrap();
        assert_eq!(
            world.get::<Transform>(door).unwrap().translation,
            Vec3::X * 2.
        );
        assert_eq!(world.resource::<DoorEvidence>().animated, 2);
        assert_eq!(world.resource::<DoorEvidence>().removed, 1);
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
        app.world_mut().resource_mut::<CurrentZone>().0 = Some(ZoneId("zone.start".into()));
        app.world_mut()
            .resource_mut::<PlayerZoneContacts>()
            .0
            .insert(ZoneId("zone.start".into()));
        app.world_mut()
            .get_mut::<Vitality>(owner)
            .unwrap()
            .apply(ResolvedDamage::new(100).unwrap())
            .unwrap();
        app.update();
        let mode = app.world().get::<TriggerClientMode>(owner).unwrap().0;
        assert_eq!(mode, 7);
        assert!(trigger.accepts_player(mode));
        assert_eq!(
            app.world().resource::<CurrentZone>().0,
            Some(ZoneId("zone.start".into()))
        );
        assert_eq!(app.world().resource::<PlayerZoneContacts>().0.len(), 1);
        {
            let mut life = app.world_mut().get_mut::<Vitality>(owner).unwrap();
            let episode = life.downed_episode().unwrap();
            let token = life.begin_self_revive(owner, episode).unwrap();
            life.complete_self_revive(owner, token, ReviveRestoration::new(100, 100).unwrap())
                .unwrap();
        }
        app.update();
        assert_eq!(app.world().get::<TriggerClientMode>(owner).unwrap().0, 0);
        assert_eq!(
            app.world().resource::<CurrentZone>().0,
            Some(ZoneId("zone.start".into()))
        );
        {
            let mut life = app.world_mut().get_mut::<Vitality>(owner).unwrap();
            life.apply(ResolvedDamage::new(100).unwrap()).unwrap();
            let expiry = life.downed_expiry(owner).unwrap();
            life.expire_downed(owner, expiry).unwrap();
        }
        app.update();
        let mode = app.world().get::<TriggerClientMode>(owner).unwrap().0;
        assert!(!trigger.accepts_player(mode));
        assert!(app.world().resource::<CurrentZone>().0.is_none());
        assert!(app.world().resource::<PlayerZoneContacts>().0.is_empty());
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
