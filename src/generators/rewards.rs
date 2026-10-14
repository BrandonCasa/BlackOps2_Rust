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
    queued: bool,
    wait: f32,
    elapsed: Option<f32>,
    reward: Option<Entity>,
    sound: Option<Entity>,
    claimed: Option<(Entity, f32)>,
    latched: bool,
}
impl CaptureRewards {
    pub(super) fn queue(&mut self, seconds: f32) {
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
#[derive(Component)]
struct Chest {
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
    pub(crate) fn tick(&mut self, dt: f32, inputs: &mut crate::rendering::NativeArtInputs) -> bool {
        self.timer_wait -= dt;
        if self.timer_wait <= 0. {
            self.timer += 0.016;
            self.timer_wait = 0.016;
        }
        self.sample_wait -= dt;
        if self.sample_wait <= 0. {
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
struct Reward;
pub(super) struct CaptureRewardsPlugin;
impl Plugin for CaptureRewardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptureRewards>()
            .add_observer(ready)
            .add_systems(
                OnEnter(GameState::PreparingShaders),
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
    extras: Query<(Entity, &bevy::gltf::GltfExtras, &GlobalTransform)>,
    children: Query<&Children>,
) {
    *rewards = default();
    let Some(reward) = game
        .content
        .generators
        .iter()
        .find_map(|generator| generator.initial_reward.as_ref())
    else {
        return;
    };
    let position = Vec3::from_array(reward.position);
    for (entity, metadata, pose) in &extras {
        if pose.translation().distance_squared(position) < 0.0001
            && serde_json::from_str::<serde_json::Value>(&metadata.value)
                .ok()
                .is_some_and(|value| {
                    value["static_instance"] == true
                        && value["source_model"] == "p6_zm_tm_challenge_box"
                })
        {
            commands.entity(entity).insert(Visibility::Hidden);
            for child in children.iter_descendants(entity) {
                commands.entity(child).insert(ColliderDisabled);
            }
        }
    }
    if let Some(scene) = game
        .content
        .presentations
        .iter()
        .find(|presentation| presentation.id == reward.presentation)
        .and_then(|presentation| presentation.scene.as_ref())
    {
        commands.spawn((
            SessionEntity,
            Chest {
                source: server.load(scene.split('#').next().unwrap_or(scene).to_owned()),
                origin_tag: None,
                glow: None,
                sound: None,
                glow_ramps: Vec::new(),
            },
            crate::rendering::NativeArtInputs::default(),
            crate::rendering::opaque::NativeShaderStart(0.),
            WorldAssetRoot(server.load(scene.clone())),
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(reward.yaw)),
        ));
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
    mut chests: Query<(Entity, &mut Chest, &mut crate::rendering::NativeArtInputs)>,
    mut visuals: Query<&mut Transform, With<Reward>>,
) {
    let Some(profile) = game
        .content
        .generators
        .iter()
        .find_map(|generator| generator.initial_reward.as_ref())
    else {
        return;
    };
    let dt = time.delta_secs();
    if !rewards.queued {
        return;
    }
    if rewards.elapsed.is_none() {
        rewards.wait -= dt;
        if rewards.wait <= 0. {
            rewards.elapsed = Some(0.);
        } else {
            return;
        }
    }
    *rewards.elapsed.as_mut().unwrap() += dt;
    let elapsed = rewards.elapsed.unwrap();
    if elapsed >= 0.5 && rewards.reward.is_none() && rewards.claimed.is_none() {
        if let Some(scene) = game
            .content
            .powerups
            .iter()
            .find(|powerup| powerup.id == profile.powerup)
            .and_then(|powerup| powerup.presentation.as_ref())
            .and_then(|id| {
                game.content
                    .presentations
                    .iter()
                    .find(|presentation| presentation.id == *id)
            })
            .and_then(|presentation| presentation.scene.as_ref())
        {
            let entity = commands
                .spawn((
                    SessionEntity,
                    Reward,
                    WorldAssetRoot(server.load(scene.clone())),
                    Transform::from_translation(Vec3::from_array(profile.position))
                        .with_rotation(Quat::from_rotation_y(profile.yaw + std::f32::consts::PI)),
                    Visibility::Inherited,
                ))
                .id();
            rewards.reward = Some(entity);
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
    for mut pose in &mut visuals {
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
    for (entity, mut chest, mut inputs) in &mut chests {
        chest
            .glow_ramps
            .retain_mut(|ramp| ramp.tick(dt, &mut inputs));
        let glowing = elapsed >= 0.75 && close.is_none_or(|elapsed| elapsed < 0.75);
        if glowing
            && chest.glow.is_none()
            && let Some(origin) = chest.origin_tag
        {
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
        commands
            .entity(entity)
            .insert(crate::animation::NativeAnimation {
                source: chest.source.clone(),
                clip: clip.into(),
                seconds,
                looping: false,
                restart: u64::from(close.is_some()),
            });
    }
}
fn collect(
    mut commands: Commands,
    input: Res<crate::input::GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut rewards: ResMut<CaptureRewards>,
    mut focus: ResMut<crate::interactions::InteractionFocus>,
    game: Res<GameDefinition>,
    session: Res<crate::gameplay::Session>,
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
) {
    if !input.use_held {
        rewards.latched = false;
    }
    if focus.target.is_some()
        || rewards.claimed.is_some()
        || rewards.elapsed.is_none_or(|elapsed| elapsed < 2.5)
    {
        return;
    }
    let Some(profile) = game
        .content
        .generators
        .iter()
        .find_map(|generator| generator.initial_reward.as_ref())
    else {
        return;
    };
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
    let Some((trigger, _, shape, trigger_pose, state)) = triggers
        .iter()
        .find(|(_, native, _, _, _)| native.0 == profile.source_trigger)
    else {
        return;
    };
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    if crate::environment::support::use_candidate(
        trigger,
        shape,
        trigger_pose,
        state,
        &spatial,
        owner,
        mode.0,
        eye,
        forward,
        false,
        game.tuning.use_radius,
    )
    .is_none()
    {
        return;
    }
    let Some(reward) = rewards.reward else {
        return;
    };
    focus.target = Some(reward);
    focus.prompt = format!(
        "Press {} to take Double Points",
        settings.label(crate::input::Action::Interact)
    );
    if input.use_held && !rewards.latched {
        if let Some(powerup) = game
            .content
            .powerups
            .iter()
            .find(|powerup| powerup.id == profile.powerup)
        {
            effects.double_points = powerup.duration;
            crate::audio::voice(
                &mut commands,
                &game,
                &session,
                "powerup_doublepoints",
                owner,
                drops.collected,
            );
            crate::audio::emit(
                &mut commands,
                "vox_zmba_powerup_doublepoints_0",
                crate::audio::SoundSource::attached(owner),
            );
            collected.write(crate::powerups::PowerupCollected(profile.powerup.clone()));
            drops.collected += 1;
            rewards.claimed = Some((owner, 0.));
            rewards.latched = true;
            focus.accepted += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn initial_reward_waits_for_grab_before_source_close_delay() {
        let mut rewards = CaptureRewards {
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
