use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Component, Clone, Copy)]
struct Secret(Target);
#[derive(Clone, Copy)]
enum Target {
    Rock(usize),
    Radio(usize),
    Log(usize),
    Floor(usize),
}
#[derive(Component)]
struct Scare;
#[derive(Resource, Default)]
pub struct Secrets {
    pub rocks: [bool; 3],
    pub radios: [bool; 3],
    pub logs: [bool; 6],
    pub floors: [bool; 4],
    pub poster_removed: bool,
    pub scared: bool,
    poster_damage: u32,
    aiming: f32,
    scare_seconds: f32,
    music: f32,
    latched: bool,
    playing: [bool; 6],
    pending: std::collections::BTreeMap<Entity, (Entity, Target, Vec3)>,
    song: Option<Song>,
}
impl Secrets {
    pub(crate) fn logs_playing(&self, robot: usize) -> bool {
        self.playing.get(3 + robot).copied().unwrap_or(false)
    }
}
struct Song {
    alias: String,
    owner: Entity,
    origin: Vec3,
    delay: f32,
    request: Option<Entity>,
    started: bool,
}
#[derive(Component)]
struct PendingSecret;
#[derive(Component)]
struct RockLoop(Entity);
#[derive(Component)]
struct LogSequence {
    index: usize,
    lines: std::collections::VecDeque<String>,
    request: Option<Entity>,
    retry: f32,
}
pub struct SecretPlugin;
impl Plugin for SecretPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Secrets>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (rock_loops, tick, poster)
                    .in_set(GameStep::Rewards)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                interact
                    .in_set(GameStep::Interactions)
                    .after(crate::side_rewards::collect)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(mut commands: Commands, mut state: ResMut<Secrets>) {
    *state = default();
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(4)),
        Transform::from_translation(Vec3::from_array([-162.840162, 4.0132, 167.189658])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(3)),
        Transform::from_translation(Vec3::from_array([-166.955724, 4.0005, 166.742872])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(5)),
        Transform::from_translation(Vec3::from_array([-149.065234, 4.0132, 163.519612])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(1)),
        Transform::from_translation(Vec3::from_array([62.516766, -8.636, -131.839716])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(0)),
        Transform::from_translation(Vec3::from_array([55.78475, -6.81355, -115.539266])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Log(2)),
        Transform::from_translation(Vec3::from_array([-17.872202, -6.2992, -59.365388])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Radio(0)),
        Transform::from_translation(Vec3::from_array([-140.779246, 3.8227, 163.552632])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Radio(1)),
        Transform::from_translation(Vec3::from_array([243.573808, -11.7729, 206.118968])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Radio(2)),
        Transform::from_translation(Vec3::from_array([3.340608, -12.7127, -4.72313])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Rock(0)),
        Transform::from_translation(Vec3::from_array([68.128642, -7.679741, -113.18621])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Rock(1)),
        Transform::from_translation(Vec3::from_array([18.314492, 7.264044, 2.227753])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Rock(2)),
        Transform::from_translation(Vec3::from_array([-17.120819, -2.857068, -64.431418])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Floor(0)),
        Transform::from_translation(Vec3::from_array([55.0672, -7.3406, -117.2718])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Floor(1)),
        Transform::from_translation(Vec3::from_array([55.118, -7.3406, -125.8062])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Floor(2)),
        Transform::from_translation(Vec3::from_array([-62.4586, 6.1722, -4.4704])),
    ));
    commands.spawn((
        SessionEntity,
        Secret(Target::Floor(3)),
        Transform::from_translation(Vec3::from_array([-70.9168, 6.1722, -4.445])),
    ));
}
fn rock_loops(
    mut commands: Commands,
    game: Res<GameDefinition>,
    sites: Query<(Entity, &Secret), Without<RockLoop>>,
) {
    for (entity, secret) in &sites {
        if matches!(secret.0, Target::Rock(_))
            && crate::audio::resolve_alias(&game, "zmb_meteor_loop").is_some()
        {
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_meteor_loop",
                crate::audio::SoundSource::attached(entity),
            );
            commands.entity(entity).insert(RockLoop(sound));
        }
    }
}
fn interact(
    mut commands: Commands,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut state: ResMut<Secrets>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    sites: Query<
        (
            Entity,
            &Secret,
            &Transform,
            Option<&PendingSecret>,
            Option<&LogSequence>,
        ),
        Without<Player>,
    >,
    players: Query<(Entity, &Player, &Transform, &crate::life::Vitality)>,
) {
    if !input.use_held {
        state.latched = false;
    }
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health)) = players.single() else {
        return;
    };
    if health.state() != crate::life::LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let candidate = sites
        .iter()
        .filter(|(_, secret, _, pending, log)| {
            pending.is_none()
                && log.is_none()
                && match secret.0 {
                    Target::Rock(i) => !state.rocks[i] && state.music <= 0.,
                    Target::Radio(i) => !state.radios[i] && state.music <= 0.,
                    Target::Log(i) => !state.playing[i],
                    Target::Floor(i) => {
                        !state.floors[i]
                            && state.music <= 0.
                            && player.prone
                            && (i < 2 || state.floors[..2].iter().all(|v| *v))
                    }
                }
        })
        .filter_map(|(entity, secret, location, ..)| {
            let delta = location.translation - eye;
            let direction = Dir3::new(delta).ok()?;
            (delta.length() < 2.
                && forward.dot(*direction) > 0.8
                && spatial
                    .cast_ray(
                        eye,
                        direction,
                        (delta.length() - 0.1).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((entity, secret.0, location.translation, delta.length()))
        })
        .min_by(|a, b| a.3.total_cmp(&b.3));
    let Some((entity, target, position, _)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    focus.prompt = format!(
        "Press {} to investigate",
        settings.label(crate::input::Action::Interact)
    );
    if !input.use_held || state.latched {
        return;
    }
    state.latched = true;
    let alias = match target {
        Target::Log(i) => format!("vox_maxi_audio_log_{}_1_0", i + 1),
        _ => "zmb_meteor_activate".into(),
    };
    if crate::audio::resolve_alias(&game, &alias).is_none() {
        return;
    }
    let song = match target {
        Target::Rock(_) => Some("mus_zmb_secret_song"),
        Target::Radio(_) => Some("mus_zmb_secret_song_a7x"),
        Target::Floor(_) => Some("mus_zmb_secret_song_aether"),
        _ => None,
    };
    if song.is_some_and(|alias| crate::audio::resolve_alias(&game, alias).is_none()) {
        return;
    }
    let request = crate::audio::emit_owned(
        &mut commands,
        alias,
        crate::audio::SoundSource::World(position),
        entity,
    );
    state.pending.insert(request, (entity, target, position));
    commands.entity(entity).insert(PendingSecret);
    if let Target::Log(i) = target {
        commands.entity(entity).insert(LogSequence {
            index: i,
            lines: (1..=if i == 1 || i == 5 { 2 } else { 3 })
                .map(|part| format!("vox_maxi_audio_log_{}_{}_0", i + 1, part))
                .collect(),
            request: Some(request),
            retry: 0.,
        });
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    zone: Res<crate::access::CurrentZone>,
    actions: Res<crate::input::Actions>,
    mut state: ResMut<Secrets>,
    mut music: ResMut<crate::audio::MusicOverride>,
    mut events: MessageReader<crate::audio::PlaybackEvent>,
    mut logs: Query<(Entity, &Transform, &mut LogSequence)>,
    rock_loops: Query<&RockLoop>,
    sounds: Query<(Entity, &crate::audio::PlayingSound)>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    players: Query<(&Player, &Transform, &crate::weapons::WeaponInventory)>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    scares: Query<Entity, With<Scare>>,
) {
    use crate::audio::PlaybackStatus;
    let dt = time.delta_secs();
    for event in events.read() {
        if let Some((site, target, position)) = state.pending.remove(&event.entity) {
            if event.status == PlaybackStatus::Accepted {
                commands.entity(site).remove::<PendingSecret>();
                focus.accepted += 1;
                let mut song = None;
                match target {
                    Target::Rock(i) => {
                        state.rocks[i] = true;
                        if let Ok(looping) = rock_loops.get(site) {
                            commands.entity(looping.0).try_despawn();
                        }
                        if state.rocks.iter().all(|v| *v) {
                            song = Some(("mus_zmb_secret_song", 310.));
                        }
                    }
                    Target::Radio(i) => {
                        state.radios[i] = true;
                        if state.radios.iter().all(|v| *v) {
                            song = Some(("mus_zmb_secret_song_a7x", 352.));
                        }
                    }
                    Target::Floor(i) => {
                        state.floors[i] = true;
                        if state.floors.iter().filter(|v| **v).count() == 3 {
                            song = Some(("mus_zmb_secret_song_aether", 135.));
                        }
                    }
                    Target::Log(i) => {
                        state.logs[i] = true;
                        state.playing[i] = true;
                    }
                }
                if let Some((alias, seconds)) = song {
                    state.music = seconds;
                    state.song = Some(Song {
                        alias: alias.into(),
                        owner: site,
                        origin: position,
                        delay: 1.,
                        request: None,
                        started: false,
                    });
                    music.0 = Some(site);
                }
            } else if matches!(
                event.status,
                PlaybackStatus::Rejected | PlaybackStatus::Cancelled
            ) {
                commands.entity(site).remove::<PendingSecret>();
                if matches!(target, Target::Log(_)) {
                    commands.entity(site).remove::<LogSequence>();
                }
            } else {
                state.pending.insert(event.entity, (site, target, position));
            }
        }
        for (entity, _, mut log) in &mut logs {
            if log.request == Some(event.entity) {
                match event.status {
                    PlaybackStatus::Completed => {
                        log.lines.pop_front();
                        log.request = None;
                        if log.lines.is_empty() {
                            state.playing[log.index] = false;
                            commands.entity(entity).remove::<LogSequence>();
                        }
                    }
                    PlaybackStatus::Rejected => {
                        log.request = None;
                        log.retry = 0.25;
                    }
                    PlaybackStatus::Cancelled => {
                        state.playing[log.index] = false;
                        commands.entity(entity).remove::<LogSequence>();
                    }
                    PlaybackStatus::Accepted => {}
                }
            }
        }
        if let Some(song) = state.song.as_mut()
            && song.request == Some(event.entity)
        {
            match event.status {
                PlaybackStatus::Accepted => song.started = true,
                PlaybackStatus::Rejected => {
                    song.request = None;
                    song.delay = 0.25;
                }
                PlaybackStatus::Cancelled => {
                    state.music = 0.;
                }
                _ => {}
            }
        }
    }
    for (entity, pose, mut log) in &mut logs {
        log.retry = (log.retry - dt).max(0.);
        if log.request.is_none()
            && log.retry == 0.
            && let Some(alias) = log.lines.front()
        {
            log.request = Some(crate::audio::emit_owned(
                &mut commands,
                alias.clone(),
                crate::audio::SoundSource::World(pose.translation),
                entity,
            ));
        }
    }
    if let Some(song) = state.song.as_mut() {
        song.delay = (song.delay - dt).max(0.);
        if song.request.is_none() && song.delay == 0. {
            for (entity, sound) in &sounds {
                if sound.channel == crate::audio::Channel::Music
                    && sound.alias.starts_with("mus_zombie_")
                {
                    commands.entity(entity).insert(crate::audio::RetiringSound);
                }
            }
            song.request = Some(crate::audio::emit_owned(
                &mut commands,
                song.alias.clone(),
                crate::audio::SoundSource::World(song.origin),
                song.owner,
            ));
        }
        if song.started {
            state.music = (state.music - dt).max(0.);
        }
    }
    if state.music == 0.
        && let Some(song) = state.song.take()
    {
        if let Some(request) = song.request {
            commands.entity(request).try_despawn();
        }
        if music.0 == Some(song.owner) {
            music.0 = None;
        }
    }
    let Ok((player, pose, inventory)) = players.single() else {
        return;
    };
    if state.scare_seconds > 0. {
        state.scare_seconds -= dt;
        if state.scare_seconds <= 0. || inventory.ads < 0.9 {
            state.scare_seconds = 0.;
            for entity in &scares {
                commands.entity(entity).despawn();
            }
        }
    }
    if state.scared {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let held = actions.state(crate::input::Action::Aim).held;
    state.aiming = if held { state.aiming + dt } else { 0. };
    let valid = held
        && state.aiming >= 1.25
        && inventory
            .active()
            .is_some_and(|w| w.definition.0.contains("dsr50"))
        && inventory.attachment() != "is"
        && zone.0.as_ref().is_some_and(|z| z.0.ends_with("nml_18"))
        && forward.dot((Vec3::new(30.0228, 25.4, 59.5884) - eye).normalize_or_zero()) > 0.998;
    if !valid || crate::audio::resolve_alias(&game, "zmb_easteregg_scarydog").is_none() {
        return;
    }
    state.scared = true;
    state.scare_seconds = 0.25;
    crate::audio::emit(
        &mut commands,
        "zmb_easteregg_scarydog",
        crate::audio::SoundSource::World(eye),
    );
    if let Ok(camera) = cameras.single()
        && let Some(image) = game
            .content
            .ui_assets
            .iter()
            .find(|a| a.id.0.contains("zm_tm_wth_dog") || a.path.contains("zm_tm_wth_dog"))
    {
        commands.spawn((
            SessionEntity,
            Scare,
            UiTargetCamera(camera),
            GlobalZIndex(1000),
            ImageNode::new(server.load(image.path.clone())),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
        ));
    }
}
fn poster(
    mut commands: Commands,
    mut state: ResMut<Secrets>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
) {
    if state.poster_removed {
        return;
    }
    for (entity, brush) in &brushes {
        if brush.id == "brush.4423f73c.58b0fe1348ff1365bedbfb88.0" {
            commands.entity(entity).insert((
                crate::environment::support::ShootableEnvironment,
                crate::environment::support::EnvironmentOwner::Quest,
            ));
        }
    }
    for hit in hits.read() {
        if brushes
            .get(hit.target)
            .is_ok_and(|(_, brush)| brush.id == "brush.4423f73c.58b0fe1348ff1365bedbfb88.0")
        {
            state.poster_damage = state.poster_damage.saturating_add(hit.amount);
            if state.poster_damage >= 1000 {
                state.poster_removed = true;
                commands
                    .entity(hit.target)
                    .insert((Visibility::Hidden, ColliderDisabled));
            }
        }
    }
}
