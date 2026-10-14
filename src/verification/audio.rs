use super::*;
use bevy::audio::AudioSinkPlayback;
#[derive(Resource, Default)]
struct Coverage {
    phase: u8,
    age: f32,
    paused: bool,
    resumed: bool,
    settings: bool,
    checked_frames: u32,
    source_errors: u32,
    volume_errors: u32,
    backend_errors: u32,
    moving: std::collections::BTreeMap<String, u32>,
    previous: std::collections::BTreeMap<Entity, Vec3>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>()
        .add_systems(Update, exercise.after(crate::audio::AudioPlayback))
        .init_resource::<SourceExercise>()
        .add_systems(
            FixedUpdate,
            sources
                .in_set(crate::gameplay::GameStep::Movement)
                .before(crate::player::move_player),
        )
        .add_systems(Last, (observe, record).chain());
}
fn exercise(
    verification: Option<Res<Verification>>,
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut coverage: ResMut<Coverage>,
    mut settings: ResMut<crate::audio::AudioSettings>,
    spatial: Query<(&SpatialAudioSink, &crate::audio::PlayingSound)>,
    regular: Query<(&AudioSink, &crate::audio::PlayingSound)>,
    evidence: Res<crate::audio::AudioEvidence>,
    game: Option<Res<crate::assets::GameDefinition>>,
) {
    let Some(v) = verification else {
        return;
    };
    if !v.barriers && !v.audio {
        return;
    }
    coverage.age += time.delta_secs();
    let pauseable = |sound: &crate::audio::PlayingSound| {
        game.as_ref()
            .and_then(|game| sound.native_definition(game))
            .is_none_or(|native| native.pauseable)
    };
    match coverage.phase {
        0 if **state == GameState::Playing
            && coverage.age > 1.
            && evidence.started > 0
            && !regular.is_empty() =>
        {
            next.set(GameState::Paused);
            coverage.phase = 1;
            coverage.age = 0.;
        }
        1 if **state == GameState::Paused
            && coverage.age > 0.2
            && !regular.is_empty()
            && spatial
                .iter()
                .all(|(s, sound)| !pauseable(sound) || s.is_paused())
            && regular
                .iter()
                .all(|(s, sound)| !pauseable(sound) || s.is_paused()) =>
        {
            coverage.paused = spatial
                .iter()
                .all(|(s, sound)| !pauseable(sound) || s.is_paused())
                && regular
                    .iter()
                    .all(|(s, sound)| !pauseable(sound) || s.is_paused())
                && !regular.is_empty();
            settings.music = 0.35;
            settings.effects = 0.5;
            settings.subtitles = false;
            coverage.settings = true;
            next.set(GameState::Playing);
            coverage.phase = 2;
            coverage.age = 0.;
        }
        2 if **state == GameState::Playing
            && coverage.age > 0.2
            && !regular.is_empty()
            && spatial.iter().all(|(s, _)| !s.is_paused())
            && regular.iter().all(|(s, _)| !s.is_paused()) =>
        {
            coverage.resumed = spatial.iter().all(|(s, _)| !s.is_paused())
                && regular.iter().all(|(s, _)| !s.is_paused());
            *settings = default();
            coverage.phase = 3;
        }
        _ => {}
    }
}
fn record(
    options: Res<LaunchOptions>,
    mut exits: MessageReader<AppExit>,
    evidence: Res<crate::audio::AudioEvidence>,
    coverage: Res<Coverage>,
    source_exercise: Res<SourceExercise>,
    mut last_phase: Local<Option<u8>>,
    spatial: Query<&SpatialAudioSink>,
    regular: Query<&AudioSink>,
) {
    if options.verify_audio && *last_phase != Some(source_exercise.phase) {
        *last_phase = Some(source_exercise.phase);
        if let Some(output) = &options.verify {
            let _ = std::fs::write(output.join("source-progress.json"), serde_json::json!({"phase":source_exercise.phase,"rows":source_exercise.rows,"source_errors":coverage.source_errors,"volume_errors":coverage.volume_errors,"backend_errors":coverage.backend_errors}).to_string());
        }
    }
    if exits.read().next().is_none() {
        return;
    }
    let Some(output) = &options.verify else {
        return;
    };
    let lifecycle = if options.verify_barriers || options.verify_audio {
        coverage.paused && coverage.resumed && coverage.settings
    } else {
        true
    };
    let passed = evidence.started > 0
        && !evidence.sinks.is_empty()
        && evidence.peak <= 64
        && lifecycle
        && coverage.source_errors == 0
        && coverage.volume_errors == 0
        && coverage.backend_errors == 0
        && (!options.verify_audio || source_exercise.rows.iter().all(|r| r["passed"] == true));
    let receipt = serde_json::json!({"passed":passed,"requested":evidence.requested,"started":evidence.started,"rejected":evidence.rejected,"peak":evidence.peak,"paused":evidence.paused,"resumed":evidence.resumed,"owner_stops":evidence.owner_stops,"frozen_tails":evidence.frozen_tails,"checked_frames":coverage.checked_frames,"source_errors":coverage.source_errors,"volume_errors":coverage.volume_errors,"backend_errors":coverage.backend_errors,"moving_alias_frames":coverage.moving,"source_cases":source_exercise.rows,"events":evidence.events,"sinks":evidence.sinks,"remaining_sinks":regular.iter().count()+spatial.iter().count(),"pause_exercised":coverage.paused,"resume_exercised":coverage.resumed,"settings_exercised":coverage.settings,"speech_transcripts_available":false});
    let _ = std::fs::write(output.join("audio.json"), receipt.to_string());
}

#[derive(Resource, Default)]
struct SourceExercise {
    phase: u8,
    age: f32,
    initial: Vec3,
    owner: Option<Entity>,
    attached_loop: Option<Entity>,
    world_loop: Option<Entity>,
    probe_owner: Option<Entity>,
    probe_loop: Option<Entity>,
    probe_tail: Option<Entity>,
    anchor: Vec3,
    rows: Vec<serde_json::Value>,
    sampled: bool,
}
fn sources(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    state: Res<State<GameState>>,
    time: Res<Time<Fixed>>,
    mut next: ResMut<NextState<GameState>>,
    mut c: ResMut<SourceExercise>,
    mut session: ResMut<Session>,
    mut input: ResMut<GameInput>,
    mut actors: ParamSet<(
        Query<(Entity, &mut Transform, &mut Player)>,
        Query<&mut Transform, Without<Player>>,
    )>,
    enemies: Query<Entity, With<Zombie>>,
    evidence: Res<crate::audio::AudioEvidence>,
    coverage: Res<Coverage>,
) {
    let Some(v) = v else {
        return;
    };
    if !v.audio {
        return;
    }
    let mut player_query = actors.p0();
    let Ok((owner, mut pose, mut player)) = player_query.single_mut() else {
        return;
    };
    input.clear();
    c.age += time.delta_secs();
    if c.phase < 8 && **state == GameState::Playing {
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(1000., TimerMode::Once));
        for entity in &enemies {
            commands.entity(entity).despawn();
        }
    }
    let mut advance = None;
    match c.phase {
        0 if **state == GameState::Playing && c.age > 1. => {
            c.owner = Some(owner);
            c.initial = pose.translation;
            c.anchor = pose.translation
                + Vec3::Y * (player.eye_height - player.height * 0.5)
                + Vec3::Z * 3.;
            c.attached_loop = Some(crate::audio::emit_loop(
                &mut commands,
                "chr_breathing_hurt",
                crate::audio::SoundSource::attached(owner),
            ));
            c.world_loop = Some(crate::audio::emit_loop(
                &mut commands,
                "zmb_board_slam",
                crate::audio::SoundSource::World(c.anchor),
            ));
            let emitter = commands
                .spawn((
                    crate::gameplay::SessionEntity,
                    Transform::from_translation(c.initial),
                ))
                .id();
            c.probe_owner = Some(emitter);
            c.probe_loop = Some(crate::audio::emit_loop(
                &mut commands,
                "zmb_capturezone_vat_loop",
                crate::audio::SoundSource::Entity {
                    entity: emitter,
                    offset: Vec3::new(0.4, 0.3, 0.),
                },
            ));
            advance = Some(1);
        }
        1 if **state == GameState::Playing => {
            input.movement = Vec2::X;
            input.fire = c.age < 0.2;
            input.reload = c.age > 0.5 && c.age < 0.6;
            if c.age > 3. {
                let moved = pose.translation.distance(c.initial);
                c.rows.push(serde_json::json!({"case":"player_movement_with_audio","passed":moved>0.2 && session.shots>0,"meters":moved,"shots":session.shots}));
                advance = Some(2);
            }
        }
        2..=5 if **state == GameState::Playing => {
            let distance = match c.phase {
                2 | 5 => 3.,
                3 => (5.715 + 25.4) / 2.,
                _ => 26.4,
            };
            pose.translation =
                c.anchor - Vec3::Z * distance - Vec3::Y * (player.eye_height - player.height * 0.5);
            player.velocity = Vec3::ZERO;
            if c.age > 0.6 && c.sampled {
                advance = Some(c.phase + 1);
            }
        }
        6 if **state == GameState::Playing => {
            pose.translation = c.initial;
            player.velocity = Vec3::ZERO;
            if c.age < 0.02 {
                if let Some(emitter) = c.probe_owner {
                    c.probe_tail = Some(crate::audio::emit(
                        &mut commands,
                        "zmb_repair_boards",
                        crate::audio::SoundSource::attached(emitter),
                    ));
                }
            }
            if c.age > 0.4 {
                if let Some(emitter) = c.probe_owner {
                    commands.entity(emitter).despawn();
                }
                advance = Some(7);
            }
        }
        7 if **state == GameState::Playing && c.age > 0.15 && c.sampled => {
            c.rows.push(serde_json::json!({"case":"moving_player_events","passed":coverage.moving.keys().any(|n| n.starts_with("vox_plr_")) && coverage.moving.keys().any(|n| n.starts_with("fly_")),"aliases":coverage.moving}));
            c.rows.push(serde_json::json!({"case":"source_invariants","passed":coverage.checked_frames>20 && coverage.source_errors==0 && coverage.volume_errors==0 && coverage.backend_errors==0,"frames":coverage.checked_frames}));
            c.rows.push(serde_json::json!({"case":"pause_settings_resume","passed":coverage.paused && coverage.resumed && coverage.settings}));
            next.set(GameState::GameOver);
            advance = Some(8);
        }
        8 if **state == GameState::GameOver => {
            commands.write_message(crate::input::RestartGame);
            advance = Some(9);
        }
        9 if **state == GameState::Playing && c.age > 0.8 => {
            let original_owner = c.owner;
            c.rows.push(serde_json::json!({"case":"fresh_session","passed":Some(owner)!=original_owner && session.points==500 && evidence.peak<=64}));
            let passed = c.rows.iter().all(|r| r["passed"] == true);
            commands.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
            advance = Some(10);
        }
        _ => {}
    }
    drop(pose);
    drop(player);
    drop(player_query);
    if **state == GameState::Playing && c.phase < 6 {
        if let Some(emitter) = c.probe_owner {
            if let Ok(mut pose) = actors.p1().get_mut(emitter) {
                pose.translation = c.initial + Vec3::X * c.age.sin();
                pose.rotation = Quat::from_rotation_y(c.age);
            }
        }
    }
    if let Some(phase) = advance {
        c.phase = phase;
        c.age = 0.;
        c.sampled = false;
    }
    if c.age > 20. && c.phase < 10 {
        let phase = c.phase;
        c.rows
            .push(serde_json::json!({"case":"phase_timeout","passed":false,"phase":phase}));
        commands.write_message(AppExit::error());
    }
}
fn observe(
    options: Res<LaunchOptions>,
    state: Res<State<GameState>>,
    settings: Res<crate::audio::AudioSettings>,
    mut coverage: ResMut<Coverage>,
    mut c: ResMut<SourceExercise>,
    poses: Query<&GlobalTransform>,
    camera: Query<&GlobalTransform, With<crate::rendering::GameCamera>>,
    game: Option<Res<crate::assets::GameDefinition>>,
    sounds: Query<
        (
            Entity,
            &crate::audio::PlayingSound,
            &GlobalTransform,
            &PlaybackSettings,
            Option<&SpatialAudioSink>,
            Option<&AudioSink>,
            &crate::audio::SemanticPlayback,
        ),
        Without<crate::audio::RetiringSound>,
    >,
) {
    use crate::audio::SoundSource;
    let Ok(listener) = camera.single() else {
        return;
    };
    coverage.previous.retain(|e, _| sounds.contains(*e));
    for (entity, sound, pose, playback, spatial, regular, semantic) in &sounds {
        let position = pose.translation();
        let expected = match sound.source {
            SoundSource::World(p) => Some(p),
            SoundSource::Entity { entity, offset } => {
                poses.get(entity).ok().map(|p| p.transform_point(offset))
            }
        };
        if let Some(expected) = expected {
            coverage.checked_frames += 1;
            if expected.distance(position) > 0.0001 {
                coverage.source_errors += 1;
            }
        }
        if **state == GameState::Playing
            && coverage
                .previous
                .insert(entity, position)
                .is_some_and(|p| p.distance(position) > 0.001)
        {
            *coverage.moving.entry(sound.alias.clone()).or_default() += 1;
        }
        let distance = listener.translation().distance(position);
        let channel_gain = settings.master
            * match sound.channel {
                crate::audio::Channel::Music => settings.music,
                crate::audio::Channel::Dialogue => settings.dialogue,
                _ => settings.effects,
            };
        let falloff = if playback.spatial {
            crate::audio::attenuation(distance, sound.distance_min, sound.distance)
        } else {
            1.
        };
        let native = game.as_ref().and_then(|game| sound.native_definition(game));
        let expected_volume = if native.is_some() {
            playback.volume.to_linear()
        } else {
            sound.base * channel_gain * falloff
        };
        if (playback.volume.to_linear() - expected_volume).abs() > 0.0001
            || spatial.is_some_and(|s| (s.volume().to_linear() - expected_volume).abs() > 0.0001)
            || regular.is_some_and(|s| (s.volume().to_linear() - expected_volume).abs() > 0.0001)
            || semantic
                .dry_gain()
                .is_some_and(|gain| !gain.is_finite() || (gain - expected_volume).abs() > 0.0001)
        {
            coverage.volume_errors += 1;
        }
        if playback.spatial && distance < sound.distance {
            let scale = playback.spatial_scale.map_or(1., |s| s.0.x);
            if (distance + 0.1) * scale > 1. {
                coverage.backend_errors += 1;
            }
        }
    }
    if !options.verify_audio || c.sampled || c.age < 0.25 {
        return;
    }
    if (2..=5).contains(&c.phase) {
        if let Some(entity) = c.world_loop {
            if let Ok((_, sound, _, playback, spatial, regular, semantic)) = sounds.get(entity) {
                let phase = c.phase;
                if let Some(actual) = semantic.dry_gain() {
                    let synchronized = regular
                        .is_some_and(|sink| (sink.volume().to_linear() - actual).abs() < 0.0001);
                    c.rows.push(serde_json::json!({"case":"routed_native_gain","phase":phase,"passed":actual.is_finite() && synchronized && (actual-playback.volume.to_linear()).abs()<0.0001,"gain":actual,"alias":sound.alias,"entity":entity.to_bits()}));
                    c.sampled = true;
                    return;
                }
                let Some(sink) = spatial else {
                    return;
                };
                let expected = match phase {
                    2 | 5 => 1.,
                    3 => 0.5,
                    _ => 0.,
                };
                let actual = sink.volume().to_linear() / 0.22387211;
                c.rows.push(serde_json::json!({"case":match phase {2=>"native_minimum_radius",3=>"distance_falloff",4=>"outside_radius",_=>"same_loop_return"},"passed":(actual-expected).abs()<0.02,"gain":actual,"entity":entity.to_bits(),"spatial_scale":playback.spatial_scale.map(|s|s.0.to_array())}));
                c.sampled = true;
            }
        }
    } else if c.phase == 7 {
        let stopped = c.probe_loop.is_some_and(|e| !sounds.contains(e));
        let frozen = c
            .probe_tail
            .and_then(|e| sounds.get(e).ok())
            .is_some_and(|(_, s, ..)| matches!(s.source, SoundSource::World(_)) && !s.looping);
        c.rows.push(serde_json::json!({"case":"retired_owner_loop_and_tail","passed":stopped && frozen,"loop_stopped":stopped,"tail_frozen":frozen}));
        c.sampled = true;
    }
}
