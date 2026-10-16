use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    player::Player,
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    world_serialization::{WorldAssetRoot, WorldInstanceReady},
};
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
    latched: bool,
    playing: [bool; 6],
    song: Option<Song>,
}
impl Secrets {
    pub(crate) fn logs_playing(&self, robot: usize) -> bool {
        self.playing.get(3 + robot).copied().unwrap_or(false)
    }

    fn available(&self, target: Target, prone: bool, music_busy: bool) -> bool {
        match target {
            Target::Rock(i) => !self.rocks[i] && !music_busy,
            Target::Radio(i) => !self.radios[i] && !music_busy,
            Target::Log(i) => !self.playing[i],
            Target::Floor(i) => {
                !self.floors[i]
                    && !music_busy
                    && prone
                    && self.floors.iter().filter(|used| **used).count() < 3
                    && (i < 2 || self.floors[..2].iter().all(|used| *used))
            }
        }
    }

    fn activate(&mut self, target: Target) -> Option<(&'static str, f32)> {
        match target {
            Target::Rock(i) => {
                self.rocks[i] = true;
                self.rocks
                    .iter()
                    .all(|used| *used)
                    .then_some(("mus_zmb_secret_song", 310.))
            }
            Target::Radio(i) => {
                self.radios[i] = true;
                self.radios
                    .iter()
                    .all(|used| *used)
                    .then_some(("mus_zmb_secret_song_a7x", 352.))
            }
            Target::Floor(i) => {
                self.floors[i] = true;
                (self.floors.iter().filter(|used| **used).count() == 3)
                    .then_some(("mus_zmb_secret_song_aether", 135.))
            }
            Target::Log(i) => {
                self.logs[i] = true;
                self.playing[i] = true;
                None
            }
        }
    }
}
struct Song {
    alias: &'static str,
    owner: Entity,
    origin: Vec3,
    delay: f32,
    request: Option<Entity>,
    started: bool,
    remaining: f32,
}
#[derive(Component)]
struct RockLoop(Entity);
#[derive(Component)]
struct Poster;
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
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (rock_loops, tick, bind_poster, poster.after(bind_poster))
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
fn setup(
    mut commands: Commands,
    mut state: ResMut<Secrets>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    *state = default();
    for (target, position) in [
        (Target::Log(4), [-162.840162, 4.0132, 167.189658]),
        (Target::Log(3), [-166.955724, 4.0005, 166.742872]),
        (Target::Log(5), [-149.065234, 4.0132, 163.519612]),
        (Target::Log(1), [62.516766, -8.636, -131.839716]),
        (Target::Log(0), [55.78475, -6.81355, -115.539266]),
        (Target::Log(2), [-17.872202, -6.2992, -59.365388]),
        (Target::Radio(0), [-140.779246, 3.8227, 163.552632]),
        (Target::Radio(1), [243.573808, -11.7729, 206.118968]),
        (Target::Radio(2), [3.340608, -12.7127, -4.72313]),
        (Target::Rock(0), [68.128642, -7.679741, -113.18621]),
        (Target::Rock(1), [18.314492, 7.264044, 2.227753]),
        (Target::Rock(2), [-17.120819, -2.857068, -64.431418]),
        (Target::Floor(0), [55.0672, -7.3406, -117.2718]),
        (Target::Floor(1), [55.118, -7.3406, -125.8062]),
        (Target::Floor(2), [-62.4586, 6.1722, -4.4704]),
        (Target::Floor(3), [-70.9168, 6.1722, -4.445]),
    ] {
        let mut site = commands.spawn((
            SessionEntity,
            Secret(target),
            Transform::from_translation(Vec3::from_array(position)),
        ));
        if let Target::Radio(i) = target {
            let yaw: f32 = [215., 9.99998, 272.6][i];
            site.insert(
                Transform::from_translation(Vec3::from_array(position))
                    .with_rotation(Quat::from_rotation_y(yaw.to_radians())),
            );
            if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_radio_01") {
                site.insert(WorldAssetRoot(server.load(scene.clone())))
                    .observe(radio_ready);
            } else {
                warn!("Hidden radio is missing its original p6_zm_tm_radio_01 model");
            }
        }
    }
    if scare_image(&game).is_none() {
        warn!("Church jumpscare is missing the original zm_tm_wth_dog UI image");
    }
}

fn radio_ready(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    let Some(cover) = children.iter_descendants(event.entity).find(|entity| {
        names
            .get(*entity)
            .is_ok_and(|name| name.as_str() == "tag_j_cover")
    }) else {
        warn!("Hidden radio model is missing its original tag_j_cover attachment");
        return;
    };
    if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_radio_01_panel2_blood") {
        commands.spawn((
            ChildOf(cover),
            WorldAssetRoot(server.load(scene.clone())),
            // The exported socket already contains the source-to-Y-up rotation.
            // Native panel geometry applies that rotation when it is drawn.
            Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ));
    } else {
        warn!("Hidden radio is missing its original blood-covered panel model");
    }
}

fn scare_image(game: &GameDefinition) -> Option<&str> {
    game.content
        .ui_assets
        .iter()
        .find(|image| image.id.0 == "ui.zm_tm_wth_dog" || image.path.contains("zm_tm_wth_dog"))
        .map(|image| image.path.as_str())
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
    mut music: ResMut<crate::audio::MusicOverride>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    sites: Query<
        (
            Entity,
            &Secret,
            &Transform,
            Option<&LogSequence>,
            Option<&RockLoop>,
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
    let feet = pose.translation - Vec3::Y * player.height * 0.5;
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let candidate = sites
        .iter()
        .filter(|(_, secret, _, log, _)| {
            log.is_none() && state.available(secret.0, player.prone, music.0.is_some())
        })
        .filter_map(|(entity, secret, location, _, looping)| {
            let delta = location.translation - eye;
            let usable = match secret.0 {
                // fake_use uses a 64-inch origin sphere and horizontal facing,
                // independent of view pitch and line of sight.
                Target::Rock(_) | Target::Floor(_) => {
                    feet.distance_squared(location.translation) < 1.6256_f32.powi(2)
                        && forward
                            .with_y(0.)
                            .normalize_or_zero()
                            .dot((location.translation - feet).with_y(0.).normalize_or_zero())
                            > 0.9
                }
                Target::Radio(_) | Target::Log(_) => {
                    let direction = Dir3::new(delta).ok()?;
                    let radius: f32 = if matches!(secret.0, Target::Radio(_)) {
                        50.
                    } else {
                        36.
                    };
                    let offset = feet - location.translation;
                    // _zm_unitrigger uses script_height, which defaults to 64 inches.
                    // The side scripts' height field does not override that value.
                    offset.xz().length_squared() < (radius * 0.0254).powi(2)
                        && offset.y < 1.6256
                        && offset.y + player.height > 0.
                        && forward.dot(*direction) > 0.8
                        && spatial
                            .cast_ray(
                                eye,
                                direction,
                                (delta.length() - 0.1).max(0.),
                                true,
                                &crate::environment::solids([owner]),
                            )
                            .is_none()
                }
            };
            usable.then_some((
                entity,
                secret.0,
                location.translation,
                looping,
                delta.length_squared(),
            ))
        })
        .min_by(|a, b| a.4.total_cmp(&b.4));
    let Some((entity, target, position, looping, _)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    focus.prompt = if matches!(target, Target::Log(_)) {
        format!(
            "Press {} to play Maxis audio log",
            settings.label(crate::input::Action::Interact)
        )
    } else {
        String::new()
    };
    if !input.use_held || state.latched {
        return;
    }
    state.latched = true;
    if let Target::Log(i) = target {
        let lines: std::collections::VecDeque<_> = (1..=if i == 1 || i == 5 { 2 } else { 3 })
            .map(|part| format!("vox_maxi_audio_log_{}_{}_0", i + 1, part))
            .collect();
        if lines
            .iter()
            .any(|alias| crate::audio::resolve_alias(&game, alias).is_none())
        {
            warn!(
                "Maxis audio log {} is missing a required original voice line",
                i + 1
            );
            return;
        }
        commands.entity(entity).insert(LogSequence {
            index: i,
            lines,
            request: None,
            retry: 0.,
        });
    } else {
        crate::audio::emit(
            &mut commands,
            "zmb_meteor_activate",
            crate::audio::SoundSource::attached(owner),
        );
    }
    if let Some(looping) = looping {
        commands.entity(looping.0).try_despawn();
    }
    focus.accepted += 1;
    if let Some((alias, seconds)) = state.activate(target) {
        state.song = Some(Song {
            alias,
            owner: entity,
            origin: if matches!(target, Target::Radio(_)) {
                Vec3::ZERO
            } else {
                position
            },
            delay: 1.,
            request: None,
            started: false,
            remaining: seconds,
        });
        music.0 = Some(entity);
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
    sounds: Query<(Entity, &crate::audio::PlayingSound)>,
    players: Query<(
        Entity,
        &Player,
        &Transform,
        &crate::weapons::WeaponInventory,
    )>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    scares: Query<Entity, With<Scare>>,
) {
    use crate::audio::PlaybackStatus;
    let dt = time.delta_secs();
    for event in events.read() {
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
                PlaybackStatus::Rejected => {
                    song.request = None;
                    song.delay = 0.25;
                }
                PlaybackStatus::Cancelled => {
                    song.remaining = 0.;
                }
                _ => {}
            }
        }
    }
    for (_, pose, mut log) in &mut logs {
        log.retry = (log.retry - dt).max(0.);
        if log.request.is_none()
            && log.retry == 0.
            && let Some(alias) = log.lines.front()
        {
            log.request = Some(crate::audio::emit(
                &mut commands,
                alias.clone(),
                crate::audio::SoundSource::World(pose.translation),
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
            song.request = Some(crate::audio::emit(
                &mut commands,
                song.alias,
                crate::audio::SoundSource::World(song.origin),
            ));
            song.started = true;
        }
        if song.started {
            song.remaining = (song.remaining - dt).max(0.);
        }
    }
    if state.song.as_ref().is_some_and(|song| song.remaining == 0.)
        && let Some(song) = state.song.take()
    {
        if let Some(request) = song.request {
            commands.entity(request).try_despawn();
        }
        if music.0 == Some(song.owner) {
            music.0 = None;
        }
    }
    let Ok((owner, player, pose, inventory)) = players.single() else {
        return;
    };
    let held = actions.state(crate::input::Action::Aim).held;
    if state.scare_seconds > 0. {
        state.scare_seconds -= dt;
        if state.scare_seconds <= 0. || !held {
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
        crate::audio::SoundSource::attached(owner),
    );
    if let Ok(camera) = cameras.single()
        && let Some(image) = scare_image(&game)
    {
        commands.spawn((
            SessionEntity,
            Scare,
            UiTargetCamera(camera),
            GlobalZIndex(1000),
            ImageNode::new(server.load(image.to_owned())),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                ..default()
            },
        ));
    }
}
fn bind_poster(
    mut commands: Commands,
    brushes: Query<
        (Entity, &crate::environment::brushes::MapBrush),
        Added<crate::environment::brushes::MapBrush>,
    >,
) {
    for (entity, brush) in &brushes {
        if brush.id == "brush.4423f73c.58b0fe1348ff1365bedbfb88.0" {
            commands.entity(entity).insert((
                Poster,
                crate::environment::support::ShootableEnvironment,
                crate::environment::support::EnvironmentOwner::Quest,
            ));
        }
    }
}

fn poster(
    mut commands: Commands,
    mut state: ResMut<Secrets>,
    mut hits: MessageReader<crate::environment::support::EnvironmentalHit>,
    posters: Query<(), With<Poster>>,
) {
    if state.poster_removed {
        return;
    }
    for hit in hits.read() {
        if posters.contains(hit.target) {
            state.poster_damage = state.poster_damage.saturating_add(hit.amount);
            if state.poster_damage >= 1000 {
                state.poster_removed = true;
                commands
                    .entity(hit.target)
                    .remove::<(Sensor, crate::environment::support::ShootableEnvironment)>()
                    .insert((RigidBody::Dynamic, CollisionLayers::new(1, LayerMask::ALL)));
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aether_requires_two_ones_then_either_five_and_cannot_replay() {
        for five in [2, 3] {
            let mut state = Secrets::default();
            assert!(!state.available(Target::Floor(five), true, false));
            assert!(!state.available(Target::Floor(0), false, false));
            assert!(!state.available(Target::Floor(0), true, true));
            assert_eq!(state.activate(Target::Floor(1)), None);
            assert!(!state.available(Target::Floor(five), true, false));
            assert_eq!(state.activate(Target::Floor(0)), None);
            assert!(state.available(Target::Floor(five), true, false));
            assert_eq!(
                state.activate(Target::Floor(five)),
                Some(("mus_zmb_secret_song_aether", 135.))
            );
            for floor in 0..4 {
                assert!(!state.available(Target::Floor(floor), true, false));
            }
        }
    }

    #[test]
    fn meteorites_and_radios_complete_once_in_any_order() {
        let mut state = Secrets::default();
        assert_eq!(state.activate(Target::Rock(2)), None);
        assert_eq!(state.activate(Target::Rock(0)), None);
        assert_eq!(
            state.activate(Target::Rock(1)),
            Some(("mus_zmb_secret_song", 310.))
        );
        assert!(!state.available(Target::Rock(0), false, false));
        assert!(!state.available(Target::Radio(0), false, true));
        assert_eq!(state.activate(Target::Radio(1)), None);
        assert_eq!(state.activate(Target::Radio(0)), None);
        assert_eq!(
            state.activate(Target::Radio(2)),
            Some(("mus_zmb_secret_song_a7x", 352.))
        );
        assert!(!state.available(Target::Radio(1), false, false));
    }

    #[test]
    fn maxis_logs_are_replayable_after_the_full_sequence() {
        let mut state = Secrets::default();
        assert!(state.available(Target::Log(3), false, true));
        state.activate(Target::Log(3));
        assert!(state.logs_playing(0));
        assert!(!state.available(Target::Log(3), false, false));
        state.playing[3] = false;
        assert!(state.logs[3]);
        assert!(state.available(Target::Log(3), false, true));
    }
}
