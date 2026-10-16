use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    player::Player,
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldInstanceReady};

#[derive(Resource, Default)]
pub(super) struct CaptureRewards {
    initial: CaptureReward,
    church: CaptureReward,
}
#[derive(Default)]
struct CaptureReward {
    queued: bool,
    wait: f32,
    elapsed: Option<f32>,
    reward: Option<Entity>,
    sound: Option<Entity>,
    claimed: Option<(Entity, f32)>,
    latched: bool,
}
impl CaptureReward {
    fn queue(&mut self, seconds: f32) {
        if !self.queued {
            self.queued = true;
            self.wait = seconds;
        }
    }
    fn close_elapsed(&self) -> Option<f32> {
        self.claimed
            .and_then(|(_, elapsed)| (elapsed >= 1.1).then_some(elapsed - 1.1))
    }
}
impl CaptureRewards {
    pub(super) fn captured(&mut self, number: u8) {
        self.initial.queue(2.);
        if number == 6 {
            self.church.queue(0.);
        }
    }
    fn get_mut(&mut self, kind: RewardKind) -> &mut CaptureReward {
        match kind {
            RewardKind::Initial => &mut self.initial,
            RewardKind::Church => &mut self.church,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum RewardKind {
    Initial,
    Church,
}
const KINDS: [RewardKind; 2] = [RewardKind::Initial, RewardKind::Church];
const FIRE_PART: &str = "part.elemental_staff_fire.upper_staff";
const CHURCH_BOX: [f32; 3] = [33.632902, 7.6708, 94.288102];
struct Profile<'a> {
    position: [f32; 3],
    yaw: f32,
    presentation: Option<&'a str>,
    powerup: Option<&'a crate::origins::ContentId>,
    source_trigger: Option<&'a str>,
}
fn profile(game: &GameDefinition, kind: RewardKind) -> Option<Profile<'_>> {
    match kind {
        RewardKind::Initial => {
            let reward = game
                .content
                .generators
                .iter()
                .find_map(|g| g.initial_reward.as_ref())?;
            Some(Profile {
                position: reward.position,
                yaw: reward.yaw,
                presentation: Some(&reward.presentation.0),
                powerup: Some(&reward.powerup),
                source_trigger: Some(&reward.source_trigger),
            })
        }
        // zm_tomb_main_quest::staff_unlock_with_zone_capture and the original
        // church_capture challenge box, rather than the hidden piece spawn.
        RewardKind::Church => Some(Profile {
            position: CHURCH_BOX,
            yaw: -std::f32::consts::FRAC_PI_2,
            presentation: None,
            powerup: None,
            source_trigger: None,
        }),
    }
}
#[derive(Component)]
#[require(crate::animation::SceneClips)]
struct Chest {
    kind: RewardKind,
    source: Handle<Gltf>,
    origin_tag: Option<Entity>,
    glow: Option<Entity>,
    sound: Option<Entity>,
    glow_ramps: Vec<BoxGlowRamp>,
}
pub(crate) struct BoxGlowRamp {
    fading_in: bool,
    timer: f32,
    timer_wait: f32,
    sample_wait: f32,
}
impl BoxGlowRamp {
    pub(crate) fn new(fading_in: bool) -> Self {
        Self {
            fading_in,
            timer: 0.,
            timer_wait: 0.016,
            sample_wait: 0.11,
        }
    }
    pub(crate) fn tick(
        &mut self,
        dt: f32,
        inputs: &mut crate::presentation::cues::CueParameters,
    ) -> bool {
        self.timer_wait -= dt;
        if self.timer_wait <= 0. {
            self.timer += 0.016;
            self.timer_wait = 0.016;
        }
        self.sample_wait -= dt;
        if self.sample_wait <= 0. {
            inputs.luminous_channel = Some((1, 0));
            inputs.script_vectors[1] = [
                if self.fading_in {
                    1. - self.timer
                } else {
                    self.timer
                },
                0.,
                0.,
                0.,
            ];
            self.sample_wait = 0.11;
            return self.timer < 1.;
        }
        true
    }
}
#[derive(Component)]
pub(crate) struct Reward {
    kind: RewardKind,
}
impl Reward {
    pub(crate) fn part(&self) -> Option<(&'static str, Vec3)> {
        (self.kind == RewardKind::Church).then_some((FIRE_PART, Vec3::from_array(CHURCH_BOX)))
    }
}
pub(super) struct CaptureRewardsPlugin;
impl Plugin for CaptureRewardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptureRewards>()
            .add_observer(ready)
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                tick.in_set(GameStep::Zombies)
                    .after(super::tick)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                collect
                    .in_set(GameStep::Interactions)
                    .after(crate::tank::interact)
                    .before(crate::rituals::claim)
                    .before(crate::powerups::collect)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut rewards: ResMut<CaptureRewards>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    props: Query<(Entity, &Name, &GlobalTransform), With<crate::assets::StaticPropRoot>>,
    children: Query<&Children>,
) {
    *rewards = default();
    for kind in KINDS {
        let Some(reward) = profile(&game, kind) else {
            continue;
        };
        let position = Vec3::from_array(reward.position);
        for (entity, name, pose) in &props {
            if pose.translation().distance_squared(position) < 0.0001
                && name.as_str() == "p6_zm_tm_challenge_box"
            {
                commands.entity(entity).insert(Visibility::Hidden);
                for child in children.iter_descendants(entity) {
                    commands.entity(child).insert(ColliderDisabled);
                }
            }
        }
        let scene = reward
            .presentation
            .and_then(|id| {
                game.content
                    .presentations
                    .iter()
                    .find(|p| p.id.0 == id)
                    .and_then(|p| p.scene.as_ref())
            })
            .or_else(|| crate::assets::native_scene(&game, "p6_zm_tm_challenge_box"));
        if let Some(scene) = scene {
            commands.spawn((
                SessionEntity,
                Chest {
                    kind,
                    source: server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.split('#').next().unwrap_or(scene).to_owned()),
                    origin_tag: None,
                    glow: None,
                    sound: None,
                    glow_ramps: Vec::new(),
                },
                crate::presentation::cues::CueParameters::default(),
                WorldAssetRoot(
                    server
                        .load_builder()
                        .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                            settings.load_animations = false;
                        })
                        .load(scene.clone()),
                ),
                Transform::from_translation(position)
                    .with_rotation(Quat::from_rotation_y(reward.yaw)),
            ));
        }
    }
}

fn ready(
    event: On<WorldInstanceReady>,
    mut chests: Query<&mut Chest>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    if let Ok(mut chest) = chests.get_mut(event.entity) {
        chest.origin_tag = children.iter_descendants(event.entity).find(|entity| {
            names
                .get(*entity)
                .is_ok_and(|name| name.as_str() == "tag_origin")
        });
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut rewards: ResMut<CaptureRewards>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut assets: crate::animation::ClipAssets,
    mut chests: Query<(
        Entity,
        &mut Chest,
        &mut crate::presentation::cues::CueParameters,
        &mut crate::animation::SceneClips,
        &mut AnimationPlayer,
        &mut AnimationGraphHandle,
    )>,
    mut visuals: Query<(&Reward, &mut Transform)>,
    mut parts: ResMut<crate::environment::parts::Parts>,
) {
    let dt = time.delta_secs();
    for kind in KINDS {
        let Some(profile) = profile(&game, kind) else {
            continue;
        };
        let rewards = rewards.get_mut(kind);
        if !rewards.queued {
            continue;
        }
        if rewards.elapsed.is_none() {
            rewards.wait -= dt;
            if rewards.wait <= 0. {
                rewards.elapsed = Some(0.);
            } else {
                continue;
            }
        }
        *rewards.elapsed.as_mut().unwrap() += dt;
        let elapsed = rewards.elapsed.unwrap();
        if elapsed >= 0.5 && rewards.reward.is_none() && rewards.claimed.is_none() {
            let scene = profile
                .powerup
                .and_then(|id| {
                    game.content
                        .powerups
                        .iter()
                        .find(|p| p.id == *id)
                        .and_then(|p| p.presentation.as_ref())
                })
                .and_then(|id| game.content.presentations.iter().find(|p| p.id == *id))
                .and_then(|p| p.scene.as_ref())
                .or_else(|| {
                    (kind == RewardKind::Church)
                        .then(|| {
                            crate::assets::native_scene(&game, "t6_wpn_zmb_staff_tip_fire_world")
                        })
                        .flatten()
                });
            if let Some(scene) = scene {
                let entity = commands
                    .spawn((
                        SessionEntity,
                        Reward { kind },
                        WorldAssetRoot(
                            server
                                .load_builder()
                                .with_settings(|settings: &mut bevy::gltf::GltfLoaderSettings| {
                                    settings.load_animations = false;
                                })
                                .load(scene.clone()),
                        ),
                        Transform::from_translation(Vec3::from_array(profile.position))
                            .with_rotation(Quat::from_rotation_y(
                                profile.yaw + std::f32::consts::PI,
                            )),
                        Visibility::Inherited,
                    ))
                    .id();
                rewards.reward = Some(entity);
                if kind == RewardKind::Church {
                    parts.reveal(FIRE_PART, entity);
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_spawn_powerup",
                    crate::audio::SoundSource::attached(entity),
                );
                let sound = crate::audio::emit_loop(
                    &mut commands,
                    "zmb_spawn_powerup_loop",
                    crate::audio::SoundSource::attached(entity),
                );
                commands.entity(sound).insert(crate::audio::SoundControl {
                    volume: 0.5,
                    pitch: 1.,
                });
                rewards.sound = Some(sound);
            }
        }
        for (reward, mut pose) in &mut visuals {
            if reward.kind != kind {
                continue;
            }
            pose.translation = Vec3::from_array(profile.position)
                + Vec3::Y * (50. * 0.0254 * ((elapsed - 0.5) / 2.).clamp(0., 1.));
        }
        if let Some((owner, previous)) = rewards.claimed {
            let elapsed = previous + dt;
            rewards.claimed = Some((owner, elapsed));
            if previous < 0.1 && elapsed >= 0.1 {
                if let Some(reward) = rewards.reward.take() {
                    commands.entity(reward).try_despawn();
                }
                if let Some(sound) = rewards.sound.take() {
                    commands.entity(sound).insert(crate::audio::SoundFade {
                        remaining: 0.1,
                        duration: 0.1,
                        out: true,
                    });
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_powerup_grabbed",
                    crate::audio::SoundSource::attached(owner),
                );
            }
        }
        let close = rewards.close_elapsed();
        for (entity, mut chest, mut inputs, mut clips, mut player, mut graph) in &mut chests {
            if chest.kind != kind {
                continue;
            }
            chest
                .glow_ramps
                .retain_mut(|ramp| ramp.tick(dt, &mut inputs));
            let glowing = elapsed >= 0.75 && close.is_none_or(|elapsed| elapsed < 0.75);
            if glowing
                && chest.glow.is_none()
                && let Some(origin) = chest.origin_tag
            {
                inputs.luminous_channel = Some((1, 0));
                inputs.script_vectors[1] = [0.; 4];
                inputs.script_available |= 1 << 1;
                chest.glow_ramps.push(BoxGlowRamp::new(true));
                chest.glow = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_challenge_fire",
                    origin,
                    Transform::default(),
                ));
                chest.sound = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_footprintbox_glow_lp",
                    crate::audio::SoundSource::attached(entity),
                ));
            } else if !glowing {
                if let Some(glow) = chest.glow.take() {
                    inputs.luminous_channel = Some((1, 0));
                    inputs.script_vectors[1] = [0.; 4];
                    inputs.script_available |= 1 << 1;
                    chest.glow_ramps.push(BoxGlowRamp::new(false));
                    commands.entity(glow).try_despawn();
                }
                if let Some(sound) = chest.sound.take() {
                    commands.entity(sound).try_despawn();
                }
            }
            let (clip, seconds) = close
                .map_or(("o_zombie_dlc4_challenge_box_open", elapsed), |elapsed| {
                    ("o_zombie_dlc4_challenge_box_close", elapsed)
                });
            let path = crate::animation::clip_path(&game, &chest.source, clip)
                .expect("Challenge chest source clip binding");
            // zm_tomb_challenges clears the old clip immediately, then SetAnim
            // uses its native default 0.2-second incoming goal time.
            clips.sample_transition(
                Some(path),
                seconds,
                false,
                crate::animation::ClipBlend {
                    incoming: 0.2,
                    outgoing: 0.,
                },
                1.,
                dt,
                &mut player,
                &mut graph,
                &mut assets,
            );
        }
    }
}
fn collect(
    mut commands: Commands,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut rewards: ResMut<CaptureRewards>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    game: Res<GameDefinition>,
    spatial: SpatialQuery,
    players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &crate::access::TriggerClientMode,
        Option<&crate::actions::HandReservation>,
        Has<crate::perks::PerkDrinking>,
        &crate::weapons::WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
    )>,
    triggers: Query<(
        Entity,
        &crate::environment::triggers::MapTrigger,
        &Collider,
        &Transform,
        &crate::environment::triggers::TriggerState,
    )>,
    mut effects: ResMut<crate::powerups::PowerupEffects>,
    mut drops: ResMut<crate::powerups::Drops>,
    mut collected: MessageWriter<crate::powerups::PowerupCollected>,
    mut parts: ResMut<crate::environment::parts::Parts>,
) {
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, mode, reservation, drinking, weapons, grenades, tactical)) =
        players.single()
    else {
        return;
    };
    if health.state() != LifeState::Standing
        || reservation.is_some()
        || drinking
        || grenades.busy()
        || tactical.busy()
        || matches!(
            weapons.action,
            crate::weapons::WeaponAction::Equipping { .. }
                | crate::weapons::WeaponAction::Unequipping { .. }
        )
    {
        return;
    }
    for kind in KINDS {
        let Some(profile) = profile(&game, kind) else {
            continue;
        };
        let rewards = rewards.get_mut(kind);
        if !input.use_held {
            rewards.latched = false;
        }
        if rewards.claimed.is_some() || rewards.elapsed.is_none_or(|elapsed| elapsed < 2.5) {
            continue;
        }
        let Some(reward) = rewards.reward else {
            continue;
        };
        if pose
            .translation
            .distance_squared(Vec3::from_array(profile.position))
            > 16.
        {
            continue;
        }
        let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
        let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
        let usable = if let Some(id) = profile.source_trigger {
            let Some((e, _, shape, pose, state)) =
                triggers.iter().find(|(_, t, _, _, _)| t.0 == id)
            else {
                continue;
            };
            crate::environment::support::use_candidate(
                e,
                shape,
                pose,
                state,
                &spatial,
                owner,
                mode.0,
                eye,
                forward,
                false,
                game.tuning.use_radius,
            )
            .is_some()
        } else {
            // The 64-inch unitrigger is centered on the box origin, which can be
            // below the floor. Trace to its nearest face instead of its buried center.
            let delta = church_use_point(eye) - eye;
            let distance = delta.length();
            mode.0 == 0
                && distance <= game.tuning.use_radius
                && Dir3::new(delta).is_ok_and(|direction| {
                    spatial
                        .cast_ray(
                            eye,
                            direction,
                            (distance - 0.01).max(0.),
                            true,
                            &crate::environment::solids([owner, reward]),
                        )
                        .is_none()
                })
        };
        if !usable {
            continue;
        }
        focus.target = Some(reward);
        focus.prompt = format!(
            "Press {} to take {}",
            settings.label(crate::input::Action::Interact),
            if kind == RewardKind::Church {
                "the fire staff part"
            } else {
                "Double Points"
            }
        );
        if input.use_held && !rewards.latched {
            let accepted = if kind == RewardKind::Church {
                parts.take(FIRE_PART, reward, owner)
            } else if let Some(powerup) = game
                .content
                .powerups
                .iter()
                .find(|powerup| Some(&powerup.id) == profile.powerup)
            {
                effects.double_points = powerup.duration;
                crate::audio::emit(
                    &mut commands,
                    "vox_zmba_powerup_doublepoints_0",
                    crate::audio::SoundSource::attached(owner),
                );
                collected.write(crate::powerups::PowerupCollected(powerup.id.clone()));
                drops.collected += 1;
                true
            } else {
                false
            };
            if accepted {
                rewards.claimed = Some((owner, 0.));
                rewards.latched = true;
                focus.accepted += 1;
            }
        }
        break;
    }
}
fn church_use_point(eye: Vec3) -> Vec3 {
    let origin = Vec3::from_array(CHURCH_BOX);
    eye.clamp(origin - Vec3::splat(0.8128), origin + Vec3::splat(0.8128))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn church_use_ray_reaches_the_volume_above_the_native_floor() {
        let eye = Vec3::new(32.2329, 9.378765, 94.2881);
        let point = church_use_point(eye);
        assert!(point.y > 7.778765);
        assert!((point.y - (CHURCH_BOX[1] + 0.8128)).abs() < 0.00001);
        assert!(point.distance(eye) < 1.6256);
    }
    #[test]
    fn only_church_capture_opens_the_fire_reward_and_recaptures_do_not_reset_it() {
        let mut rewards = CaptureRewards::default();
        for number in 1..6 {
            rewards.captured(number);
            assert!(!rewards.church.queued);
        }
        rewards.captured(6);
        assert!(rewards.church.queued);
        assert_eq!(rewards.church.wait, 0.);
        rewards.church.claimed = Some((Entity::PLACEHOLDER, 3.));
        rewards.captured(6);
        assert_eq!(rewards.church.claimed, Some((Entity::PLACEHOLDER, 3.)));
    }
    #[test]
    fn initial_reward_waits_for_grab_before_source_close_delay() {
        let mut rewards = CaptureReward {
            elapsed: Some(120.),
            ..default()
        };
        assert!(rewards.close_elapsed().is_none());
        rewards.claimed = Some((Entity::PLACEHOLDER, 1.099));
        assert!(rewards.close_elapsed().is_none());
        rewards.claimed = Some((Entity::PLACEHOLDER, 1.6));
        assert!((rewards.close_elapsed().unwrap() - 0.5).abs() < 0.00001);
    }
}
