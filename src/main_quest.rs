use crate::{
    GameState,
    gameplay::{GameStep, SessionEntity},
    input::GameInput,
    interactions::InteractionFocus,
    life::{LifeState, Vitality},
    player::Player,
    quests::{MainQuest, QuestStage},
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
#[derive(Resource, Default)]
pub struct QuestProgress {
    pub seal_open: bool,
    pub drone_consumed: bool,
    pub encounter_spawned: u32,
    pub encounter_killed: u32,
    drone_step: Option<usize>,
    drone_pause: f32,
    stage_delay: Option<f32>,
    staff_returns: Option<[f32; 4]>,
    collapse_delay: f32,
    spawn_timer: f32,
    latched: bool,
}
#[derive(Component)]
pub(crate) enum Target {
    Staff { index: usize, robot: Option<usize> },
    Button(usize),
}
#[derive(Component, Default)]
pub(crate) enum ButtonPhase {
    #[default]
    Dormant,
    Start(f32),
    Sync {
        sound: Entity,
        elapsed: f32,
        accepted: bool,
    },
    Gap(f32),
    Await {
        sound: Entity,
        elapsed: f32,
        accepted: bool,
    },
    Ready,
    Authorized(Entity),
    Cooldown(f32),
}
pub struct MainQuestPlugin;
impl Plugin for MainQuestPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<QuestProgress>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (buttons, interact)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::tablets::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                progress
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut progress: ResMut<QuestProgress>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
) {
    *progress = default();
    for (entity, brush) in &brushes {
        if game
            .environment
            .brush_entities
            .iter()
            .find(|b| b.id == brush.id)
            .is_some_and(|b| {
                b.source
                    .get("targetname")
                    .is_some_and(|n| n == "easter_mechzombie_spawn")
            })
        {
            commands
                .entity(entity)
                .insert(Visibility::Inherited)
                .remove::<ColliderDisabled>();
        }
    }
    let pedestals = [
        (0, None, Vec3::new(-0.015748, -19.0373, -4.283202)),
        (1, Some(2), Vec3::new(-144.878552, 4.0894, 170.043602)),
        (2, Some(0), Vec3::new(-171.700952, 4.0767, 170.056302)),
        (3, Some(1), Vec3::new(-158.086552, 4.0894, 170.056302)),
    ];
    for (index, robot, point) in pedestals {
        let mut entity = commands.spawn((
            SessionEntity,
            Target::Staff { index, robot },
            Transform::from_translation(point),
            Visibility::Hidden,
        ));
        if let Some(scene) = game
            .content
            .presentations
            .iter()
            .filter_map(|p| p.scene.as_ref())
            .find(|s| s.contains("p6_zm_tm_staff_holder_lod0"))
        {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    for (robot, point) in [
        Vec3::new(-171.704, 4.8768, 170.7261),
        Vec3::new(-158.0896, 4.8768, 170.688),
        Vec3::new(-144.8816, 4.8768, 170.7007),
    ]
    .into_iter()
    .enumerate()
    {
        let mut entity = commands.spawn((
            SessionEntity,
            Target::Button(robot),
            ButtonPhase::default(),
            Transform::from_translation(point),
            Visibility::Hidden,
        ));
        if let Some(scene) = game
            .content
            .presentations
            .iter()
            .filter_map(|p| p.scene.as_ref())
            .find(|s| s.contains("p_rus_alarm_button_lod0"))
        {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
}
fn progress(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut state: ResMut<QuestProgress>,
    mut quest: ResMut<MainQuest>,
    mut strikes: MessageReader<crate::tablets::BeaconStrike>,
    mut deaths: MessageReader<crate::damage::EnemyDeath>,
    mut drones: Query<(Entity, &mut crate::buildables::Drone, &Transform)>,
    panzers: Query<(&crate::panzer::Panzer, &Vitality, &Transform)>,
    mut buildables: ResMut<crate::buildables::BuildableState>,
    game: Res<crate::assets::GameDefinition>,
    server: Res<AssetServer>,
    brushes: Query<(Entity, &crate::environment::brushes::MapBrush)>,
    mut targets: Query<(&Target, &Transform, &mut Visibility)>,
    spatial: SpatialQuery,
    triggers: Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    (waves, mut session, generators, crafting): (
        Res<crate::panzer::PanzerWaves>,
        ResMut<crate::gameplay::Session>,
        Res<crate::generators::Generators>,
        Res<crate::crafting::Crafting>,
    ),
) {
    for (target, _, mut visible) in &mut targets {
        *visible = if quest.enabled
            && match target {
                Target::Staff { index, .. } => {
                    quest.stage == QuestStage::PlaceStaffs
                        || state.staff_returns.is_some()
                            && quest
                                .placed
                                .contains(&crate::staff_upgrades::ELEMENTS[*index])
                }
                Target::Button(_) => quest.stage == QuestStage::BreakSeal,
            } {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !quest.enabled {
        strikes.clear();
        deaths.clear();
        return;
    }
    if quest.stage == QuestStage::UpgradeStaffs {
        quest.update_prerequisites(
            [
                "elemental_staff_fire",
                "elemental_staff_water",
                "elemental_staff_air",
                "elemental_staff_lightning",
            ]
            .into_iter()
            .all(|staff| crafting.crafted.contains(staff)),
            generators.active_count() == 6,
        );
    }
    if let Some(remaining) = state.staff_returns.as_mut() {
        for (index, delay) in remaining.iter_mut().enumerate() {
            let before = *delay;
            *delay = (*delay - time.delta_secs()).max(0.);
            if before >= 3. && *delay < 3. {
                if let Some((_, pose, _)) = targets.iter().find(|(target, _, _)| matches!(target, Target::Staff { index: slot, .. } if *slot == index)) {
                    crate::audio::emit(&mut commands, "zmb_footprintbox_disappear", crate::audio::SoundSource::World(pose.translation));
                }
            }
            if *delay == 0. {
                quest.placed.remove(&crate::staff_upgrades::ELEMENTS[index]);
            }
        }
        if remaining.iter().all(|delay| *delay == 0.) {
            state.staff_returns = None;
        }
    }
    if quest.stage == QuestStage::PlaceStaffs && quest.placed.len() == 4 {
        if state.stage_delay.is_none() {
            // Solo wait_network_frame is 0.1 s after the source three-second alarm.
            state.stage_delay = Some(3.1);
            crate::audio::emit(
                &mut commands,
                "zmb_squest_robot_alarm_high",
                crate::audio::SoundSource::World(Vec3::new(-0.3556, 22.1234, 0.0254)),
            );
        } else if let Some(remaining) = state.stage_delay.as_mut() {
            *remaining = (*remaining - time.delta_secs()).max(0.);
            if *remaining == 0. {
                quest.transition(QuestStage::PlaceStaffs, QuestStage::BreakSeal);
                // Authored plinth order is wind, ice, lightning, fire, staggered by
                // three solo network frames; each staff returns three seconds later.
                state.staff_returns = Some([3.9, 3.3, 3., 3.6]);
                state.stage_delay = None;
            }
        }
    }
    for strike in strikes.read() {
        if !quest.strike_authorized() || state.seal_open || state.collapse_delay > 0. {
            continue;
        }
        let accepted = spatial
            .shape_intersections(
                &Collider::sphere(0.254),
                strike.point + Vec3::Y * 0.3048,
                Quat::IDENTITY,
                &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
            )
            .iter()
            .filter_map(|e| triggers.get(*e).ok())
            .filter(|(_, state)| state.enabled)
            .map(|(trigger, _)| trigger)
            .any(|v| {
                game.environment
                    .triggers
                    .iter()
                    .find(|t| t.id == v.0)
                    .is_some_and(|t| {
                        t.source
                            .get("targetname")
                            .is_some_and(|n| n == "fire_link_damage")
                    })
            });
        if accepted {
            state.collapse_delay = 3.;
            crate::audio::emit(
                &mut commands,
                "zmb_squest_robot_floor_collapse",
                crate::audio::SoundSource::World(Vec3::new(-79.248, 4.5974, -28.3464)),
            );
        }
    }
    if state.collapse_delay > 0. {
        state.collapse_delay = (state.collapse_delay - time.delta_secs()).max(0.);
        if state.collapse_delay == 0. {
            state.seal_open = true;
            quest.transition(QuestStage::BreakSeal, QuestStage::PanzerEncounter);
            for (entity, brush) in &brushes {
                if game
                    .environment
                    .brush_entities
                    .iter()
                    .find(|b| b.id == brush.id)
                    .is_some_and(|b| {
                        b.source
                            .get("targetname")
                            .is_some_and(|n| n == "easter_mechzombie_spawn")
                    })
                {
                    commands
                        .entity(entity)
                        .insert((Visibility::Hidden, ColliderDisabled));
                }
            }
        }
    }
    if quest.stage != QuestStage::PanzerEncounter {
        deaths.clear();
        return;
    }
    if !state.drone_consumed {
        let path = [
            Vec3::new(-70.7136, 8.9408, -15.6464),
            Vec3::new(-79.4512, 6.9088, -28.2448),
            Vec3::new(-79.4512, 0.2032, -28.2448),
        ];
        for (entity, mut drone, pose) in &mut drones {
            if state.drone_step.is_none()
                && (pose.translation - path[0]).xz().length_squared() < 12.7_f32.powi(2)
            {
                state.drone_step = Some(0);
            }
            if let Some(step) = state.drone_step {
                if state.drone_pause > 0. {
                    state.drone_pause = (state.drone_pause - time.delta_secs()).max(0.);
                    if state.drone_pause > 0. {
                        continue;
                    }
                    state.drone_step = Some(2);
                }
                let step = state.drone_step.unwrap_or(step);
                drone.returning = false;
                drone.quest_goal = Some(path[step]);
                if pose.translation.distance(path[step]) < 0.5 {
                    if step == 0 {
                        state.drone_step = Some(1);
                    } else if step == 1 {
                        state.drone_pause = 2.;
                    } else {
                        commands.entity(entity).despawn();
                        buildables.drone = None;
                        buildables.drone_disabled = true;
                        buildables.drone_cooldown = f32::MAX;
                        state.drone_consumed = true;
                        state.drone_step = None;
                    }
                }
            }
        }
    }
    for death in deaths.read() {
        if panzers
            .get(death.0.impact.target)
            .is_ok_and(|(panzer, _, _)| panzer.quest)
        {
            state.encounter_killed += 1;
            let reward = match state.encounter_killed {
                4 => Some("full_ammo"),
                8 => Some("nuke"),
                _ => None,
            };
            if let Some(reward) = reward {
                crate::powerups::spawn_named(
                    &mut commands,
                    &game,
                    &server,
                    reward,
                    panzers
                        .get(death.0.impact.target)
                        .map_or(Vec3::new(-79.4512, 3.8354, -28.2448), |(_, _, p)| {
                            p.translation
                        }),
                );
            }
        }
    }
    if state.drone_consumed && state.encounter_spawned < 8 {
        state.spawn_timer -= time.delta_secs();
        let alive = panzers
            .iter()
            .filter(|(panzer, health, _)| panzer.quest && health.state() == LifeState::ActorAlive)
            .count();
        if state.spawn_timer <= 0. {
            if alive < 4 {
                crate::panzer::spawn_quest(
                    &mut commands,
                    crate::panzer::health_for_wave(&waves),
                    state.encounter_spawned as usize % 4,
                );
                state.encounter_spawned += 1;
            }
            // The source waits after every attempt, including a full live cap.
            state.spawn_timer = 0.5 + session.random() as f32 / u32::MAX as f32 * 0.5;
        }
    }
    if state.encounter_killed >= 8 {
        if let Some(remaining) = state.stage_delay.as_mut() {
            *remaining -= time.delta_secs();
            if *remaining <= 0. {
                quest.transition(QuestStage::PanzerEncounter, QuestStage::RecoverDrone);
                state.stage_delay = None;
            }
        } else {
            state.stage_delay = Some(0.1);
        }
    }
}
fn buttons(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    quest: Res<MainQuest>,
    mut events: MessageReader<crate::audio::PlaybackEvent>,
    mut buttons: Query<(Entity, &Target, &mut ButtonPhase)>,
) {
    for event in events.read() {
        for (_, _, mut phase) in &mut buttons {
            let next = match *phase {
                ButtonPhase::Sync { sound, .. } if sound == event.entity => ButtonPhase::Gap(0.5),
                ButtonPhase::Await { sound, .. } if sound == event.entity => ButtonPhase::Ready,
                _ => continue,
            };
            match event.status {
                crate::audio::PlaybackStatus::Completed => *phase = next,
                crate::audio::PlaybackStatus::Accepted => match &mut *phase {
                    ButtonPhase::Sync { accepted, .. } | ButtonPhase::Await { accepted, .. } => {
                        *accepted = true;
                    }
                    _ => unreachable!(),
                },
                crate::audio::PlaybackStatus::Rejected
                | crate::audio::PlaybackStatus::Cancelled => match &mut *phase {
                    ButtonPhase::Sync { accepted, .. } | ButtonPhase::Await { accepted, .. } => {
                        *accepted = false;
                    }
                    _ => unreachable!(),
                },
            }
        }
    }
    for (entity, target, mut phase) in &mut buttons {
        if !quest.enabled || quest.stage != QuestStage::BreakSeal {
            if let ButtonPhase::Authorized(sound) = *phase {
                commands.entity(sound).try_despawn();
            }
            *phase = ButtonPhase::Dormant;
            continue;
        }
        match *phase {
            ButtonPhase::Dormant => {
                let Target::Button(robot) = target else {
                    unreachable!()
                };
                // Fire-link structs are authored in robot order 2, 1, 0.
                *phase = ButtonPhase::Start((2 - *robot) as f32 * 0.1);
            }
            ButtonPhase::Start(mut remaining) => {
                remaining -= time.delta_secs();
                *phase = if remaining <= 0. {
                    ButtonPhase::Sync {
                        sound: crate::audio::emit(
                            &mut commands,
                            "vox_maxi_robot_sync_0",
                            crate::audio::SoundSource::attached(entity),
                        ),
                        elapsed: 0.,
                        accepted: false,
                    }
                } else {
                    ButtonPhase::Start(remaining)
                };
            }
            ButtonPhase::Gap(mut remaining) => {
                remaining -= time.delta_secs();
                *phase = if remaining <= 0. {
                    ButtonPhase::Await {
                        sound: crate::audio::emit(
                            &mut commands,
                            "vox_maxi_robot_await_0",
                            crate::audio::SoundSource::attached(entity),
                        ),
                        elapsed: 0.,
                        accepted: false,
                    }
                } else {
                    ButtonPhase::Gap(remaining)
                };
            }
            ButtonPhase::Authorized(sound) if !quest.strike_authorized() => {
                commands.entity(sound).try_despawn();
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_robot_button_deactivate",
                    crate::audio::SoundSource::attached(entity),
                );
                crate::audio::emit(
                    &mut commands,
                    "vox_maxi_robot_deactivated_0",
                    crate::audio::SoundSource::attached(entity),
                );
                *phase = ButtonPhase::Ready;
            }
            ButtonPhase::Cooldown(mut remaining) => {
                remaining -= time.delta_secs();
                *phase = if remaining <= 0. {
                    ButtonPhase::Ready
                } else {
                    ButtonPhase::Cooldown(remaining)
                };
            }
            _ => {}
        }
        // Native playsoundwithnotify stores an unknown duration (0064b3d0);
        // the server entity tick (00600e30) notifies after 5000ms if it remains
        // unknown. Rejected/inaudible voices therefore still release the gate.
        // Accepted voices retain their encoded playback completion above.
        // Cancelled voices use the unknown-duration path: the native client
        // duration update is not recovered, so its post-cancellation deadline
        // cannot yet be reproduced exactly.
        let syncing = matches!(*phase, ButtonPhase::Sync { .. });
        if let ButtonPhase::Sync {
            elapsed, accepted, ..
        }
        | ButtonPhase::Await {
            elapsed, accepted, ..
        } = &mut *phase
        {
            *elapsed += time.delta_secs();
            if !*accepted && *elapsed >= 5. {
                *phase = if syncing {
                    ButtonPhase::Gap(0.5)
                } else {
                    ButtonPhase::Ready
                };
            }
        }
    }
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut state: ResMut<QuestProgress>,
    mut quest: ResMut<MainQuest>,
    robots: Res<crate::robots::Robots>,
    mut focus: ResMut<InteractionFocus>,
    game: Res<crate::assets::GameDefinition>,
    spatial: SpatialQuery,
    targets: Query<(Entity, &Target, &Transform), Without<Player>>,
    mut buttons: Query<&mut ButtonPhase>,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut WeaponInventory,
        Has<crate::perks::PerkDrinking>,
        Option<&crate::actions::HandReservation>,
    )>,
) {
    if !input.use_held {
        state.latched = false;
    }
    if !quest.enabled || focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, mut inventory, drinking, reservation)) =
        players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let selected = targets
        .iter()
        .filter(|(target_entity, target, _)| match target {
            Target::Staff { index, robot } => {
                quest.stage == QuestStage::PlaceStaffs
                    && !quest
                        .placed
                        .contains(&crate::staff_upgrades::ELEMENTS[*index])
                    && robot.is_none_or(|i| {
                        robots
                            .occupancy
                            .is_some_and(|o| o.robot == i && o.player == owner)
                    })
            }
            Target::Button(i) => {
                quest.stage == QuestStage::BreakSeal
                    && buttons
                        .get(*target_entity)
                        .is_ok_and(|phase| matches!(phase, ButtonPhase::Ready))
                    && robots
                        .occupancy
                        .is_some_and(|o| o.robot == *i && o.player == owner)
            }
        })
        .filter_map(|(e, t, p)| {
            let delta = p.translation + Vec3::Y * 0.6 - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            (distance < 2.54
                && forward.dot(*direction) > 0.7
                && spatial
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.1).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((e, t, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, target, _)) = selected else {
        return;
    };
    focus.target = Some(entity);
    focus.hold_seconds = 0.;
    focus.progress = 0.;
    let key = settings.label(crate::input::Action::Interact);
    focus.prompt = match target {
        Target::Staff { .. } => format!("Press {key} to place upgraded staff"),
        Target::Button(_) => format!("Press {key} to authorize G-Strike"),
    };
    if !input.use_held
        || state.latched
        || drinking
        || reservation.is_some()
        || inventory.action != WeaponAction::Ready
    {
        return;
    }
    match target {
        Target::Staff { index, .. } => {
            let id = crate::staff_upgrades::WEAPONS[*index].replace("_zm", "_upgraded_zm");
            if !inventory.active().is_some_and(|w| w.definition.0 == id)
                || inventory.deposit_active().is_err()
            {
                return;
            }
            quest.placed.insert(crate::staff_upgrades::ELEMENTS[*index]);
            if inventory.active().is_none()
                && let Some(fists) = game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.zombie_fists_zm")
            {
                let _ = inventory.grant(fists);
            }
        }
        Target::Button(_) => {
            let Ok(mut phase) = buttons.get_mut(entity) else {
                return;
            };
            if !matches!(*phase, ButtonPhase::Ready) {
                return;
            }
            if robots.triple_active && quest.authorize_strike() {
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_robot_button_activate",
                    crate::audio::SoundSource::attached(entity),
                );
                crate::audio::emit(
                    &mut commands,
                    "vox_maxi_robot_activated_0",
                    crate::audio::SoundSource::attached(entity),
                );
                *phase = ButtonPhase::Authorized(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_squest_robot_button_timer",
                    crate::audio::SoundSource::attached(entity),
                ));
            } else {
                *phase = ButtonPhase::Cooldown(3.);
                crate::audio::emit(
                    &mut commands,
                    "vox_maxi_robot_abort_0",
                    crate::audio::SoundSource::attached(entity),
                );
                crate::audio::emit(
                    &mut commands,
                    "zmb_squest_robot_button_deactivate",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
    }
    state.latched = true;
    focus.accepted += 1;
    crate::audio::emit(
        &mut commands,
        match target {
            Target::Staff { .. } => "zmb_squest_robot_place_staff",
            Target::Button(_) => "zmb_squest_robot_button",
        },
        crate::audio::SoundSource::attached(entity),
    );
}
