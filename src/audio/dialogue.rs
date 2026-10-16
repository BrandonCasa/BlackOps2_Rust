use super::*;
use crate::{
    gameplay::Session,
    life::{LifeState, Vitality},
    player::Player,
};

#[derive(Message)]
struct VoiceRequest {
    speaker: Entity,
    event: String,
}

#[derive(Default)]
struct Speaker {
    active: Option<Entity>,
    last_finished: Option<f64>,
    until: f64,
    exert_until: f64,
    wall_buy: bool,
    box_uses: u32,
    pools: BTreeMap<String, Vec<String>>,
}

#[derive(Resource, Default)]
struct Dialogue(BTreeMap<Entity, Speaker>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<Dialogue>()
        .add_message::<VoiceRequest>()
        .add_systems(OnEnter(GameState::StartingSession), reset)
        .add_systems(PostUpdate, speak.before(AudioPlayback));
}

fn reset(mut dialogue: ResMut<Dialogue>, mut requests: ResMut<Messages<VoiceRequest>>) {
    *dialogue = default();
    requests.clear();
}

pub(crate) fn voice(commands: &mut Commands, event: &str, speaker: Entity) {
    commands.write_message(VoiceRequest {
        speaker,
        event: event.into(),
    });
}

// _zm_audio::zmbvoxgetlinevariant: shuffle exact numeric variants without repeats.
pub(super) fn choose(
    pools: &mut BTreeMap<String, Vec<String>>,
    game: &GameDefinition,
    session: &mut Session,
    prefix: &str,
) -> Option<String> {
    let pool = pools.entry(prefix.into()).or_default();
    if pool.is_empty() {
        pool.extend(
            game.audio
                .aliases
                .range(prefix.to_owned()..)
                .map(|(name, _)| name)
                .take_while(|name| name.starts_with(prefix))
                .filter(|name| {
                    !(prefix == "vox_sam_generic_encourage" && name.ends_with("_6"))
                        && name.strip_prefix(prefix).is_some_and(|tail| {
                            tail.strip_prefix('_').is_some_and(|variant| {
                                !variant.is_empty() && variant.bytes().all(|c| c.is_ascii_digit())
                            })
                        })
                })
                .cloned(),
        );
    }
    if pool.is_empty() {
        return None;
    }
    Some(pool.remove(session.random() as usize % pool.len()))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(in crate::audio) fn game() -> GameDefinition {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gameplay/origins");
        let mut game: GameDefinition =
            serde_json::from_slice(&std::fs::read(root.join("game.json")).unwrap()).unwrap();
        let audio: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("audio.json")).unwrap()).unwrap();
        game.audio = serde_json::from_value(audio["audio"].clone()).unwrap();
        game
    }

    pub(in crate::audio) fn player(world: &mut World) -> Entity {
        world
            .spawn((
                Player {
                    yaw: 0.,
                    pitch: 0.,
                    velocity: Vec3::ZERO,
                    height: 1.8,
                    grounded: true,
                    crouched: false,
                    prone: false,
                    eye_height: 1.6,
                },
                Vitality::human(
                    100,
                    100,
                    50,
                    crate::life::PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                )
                .unwrap(),
                Transform::default(),
            ))
            .id()
    }

    fn requests(app: &mut App) -> Vec<(Entity, String)> {
        let world = app.world_mut();
        let requests: Vec<_> = world
            .query::<(Entity, &SoundRequest)>()
            .iter(world)
            .map(|(entity, sound)| (entity, sound.alias.clone()))
            .collect();
        for (entity, _) in &requests {
            world.despawn(*entity);
        }
        requests
    }

    #[test]
    fn voice_variants_do_not_cross_into_other_events_or_repeat_before_exhaustion() {
        let game = game();
        let mut pools = BTreeMap::new();
        let mut session = Session::default();
        for (prefix, count) in [
            ("vox_plr_0_heal_revived", 5),
            ("vox_plr_0_kill_fire", 5),
            ("vox_sam_generic_encourage", 6),
        ] {
            let chosen: BTreeSet<_> = (0..count)
                .map(|_| choose(&mut pools, &game, &mut session, prefix).unwrap())
                .collect();
            assert_eq!(chosen.len(), count);
            assert!(chosen.iter().all(|name| {
                name.strip_prefix(prefix).unwrap()[1..]
                    .bytes()
                    .all(|c| c.is_ascii_digit())
            }));
            assert!(!chosen.contains("vox_sam_generic_encourage_6"));
            assert!(chosen.contains(&choose(&mut pools, &game, &mut session, prefix).unwrap()));
        }
    }

    #[test]
    fn story_blocks_barks_and_speaking_cooldown_follows_playback_completion() {
        let mut app = App::new();
        app.insert_resource(game())
            .insert_resource(State::new(GameState::Playing))
            .init_resource::<Time>()
            .init_resource::<Session>()
            .init_resource::<Dialogue>()
            .init_resource::<StoryReservation>()
            .add_message::<VoiceRequest>()
            .add_message::<PlaybackEvent>()
            .add_systems(Update, speak);
        let player = player(app.world_mut());
        let request = |app: &mut App| {
            app.world_mut().write_message(VoiceRequest {
                speaker: player,
                event: "ammo_out".into(),
            });
        };
        app.world_mut().resource_mut::<StoryReservation>().0 = Some(player);
        request(&mut app);
        app.update();
        assert!(requests(&mut app).is_empty());
        app.world_mut().resource_mut::<StoryReservation>().0 = None;
        request(&mut app);
        request(&mut app);
        app.update();
        let first = requests(&mut app);
        assert_eq!(first.len(), 1);
        app.world_mut().write_message(PlaybackEvent {
            entity: first[0].0,
            status: PlaybackStatus::Completed,
        });
        request(&mut app);
        app.update();
        assert!(requests(&mut app).is_empty());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs_f64(0.3));
        request(&mut app);
        app.update();
        let second = requests(&mut app);
        assert_eq!(second.len(), 1);
        app.world_mut().write_message(PlaybackEvent {
            entity: second[0].0,
            status: PlaybackStatus::Completed,
        });
        app.update();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(14));
        request(&mut app);
        app.update();
        assert!(requests(&mut app).is_empty());
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(std::time::Duration::from_secs(1));
        request(&mut app);
        app.update();
        assert_eq!(requests(&mut app).len(), 1);
    }
}

fn speak(
    mut commands: Commands,
    game: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut dialogue: ResMut<Dialogue>,
    mut requests: MessageReader<VoiceRequest>,
    mut playback: MessageReader<PlaybackEvent>,
    reservation: Res<StoryReservation>,
    players: Query<(&Vitality, &Transform), With<Player>>,
    sounds: Query<&PlayingSound>,
    transforms: TransformHelper,
) {
    let now = time.elapsed_secs_f64();
    for event in playback.read() {
        for speaker in dialogue.0.values_mut() {
            if speaker.active != Some(event.entity) {
                continue;
            }
            match event.status {
                PlaybackStatus::Completed | PlaybackStatus::Cancelled => {
                    speaker.active = None;
                    // _zm_audio::do_player_or_npc_playvox holds the speaker for
                    // 15 seconds after two lines finish less than five seconds apart.
                    speaker.until = now
                        + if speaker.last_finished.is_some_and(|last| now < last + 5.) {
                            15.
                        } else {
                            0.25
                        };
                    speaker.last_finished = Some(speaker.until);
                }
                PlaybackStatus::Rejected => speaker.active = None,
                PlaybackStatus::Accepted => {}
            }
        }
    }
    if **state != GameState::Playing {
        requests.clear();
        return;
    }
    let Some(game) = game else { return };
    dialogue.0.retain(|entity, _| players.contains(*entity));
    for request in requests.read() {
        let Ok((health, pose)) = players.get(request.speaker) else {
            continue;
        };
        let speaker = dialogue.0.entry(request.speaker).or_default();
        let event = match request.event.as_str() {
            "wall_buy" => {
                let first = !std::mem::replace(&mut speaker.wall_buy, true);
                if first {
                    "discover_wall_buy"
                } else {
                    "generic_wall_buy"
                }
            }
            "use_box" => {
                speaker.box_uses += 1;
                match speaker.box_uses {
                    1 => "use_box_intro",
                    2 => "use_box_2nd_time",
                    _ => continue,
                }
            }
            "take_box_weapon" => match speaker.box_uses {
                1 => "take_weapon_intro",
                2 => "take_weapon_2nd_time",
                _ => continue,
            },
            other => other,
        };
        let exert = event.starts_with("exert_");
        if matches!(
            event,
            "dig_gun" | "dig_grenade" | "dig_zombie" | "dig_powerup" | "dig_cash"
        ) && session.random() % 100 >= 15
        {
            continue;
        }
        if health.state() != LifeState::Standing
            || reservation.0.is_some()
            || speaker.active.is_some()
            || now < speaker.until
            || exert && now < speaker.exert_until
            || sounds.iter().any(|sound| {
                sound.channel == Channel::Dialogue
                    && (matches!(sound.source, SoundSource::Entity { entity, .. } if entity == request.speaker)
                        || sound.alias.starts_with("vox_plr_")
                            && sound.source.position(&transforms).is_some_and(|position| {
                                position.distance_squared(pose.translation) < (1000.0_f32 * 0.0254).powi(2)
                            }))
            })
        {
            continue;
        }
        let prefix = format!("vox_plr_{}_{}", session.character, event);
        if let Some(alias) = choose(&mut speaker.pools, &game, &mut session, &prefix) {
            let sound = emit(&mut commands, alias, SoundSource::attached(request.speaker));
            if exert {
                // _zm_audio::playerexert uses a separate 1.5-3 second throttle.
                speaker.exert_until =
                    now + 1.5 + session.random() as f64 / (u32::MAX as f64 + 1.) * 1.5;
            } else {
                speaker.active = Some(sound);
            }
        }
    }
}
