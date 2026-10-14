use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::{GameStep, SessionEntity},
    life::{LifeState, Vitality},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;

const ORIGINS: [Vec3; 2] = [
    Vec3::new(68.723002, -7.9248, -118.373398),
    Vec3::new(33.632902, 7.6708, 94.288102),
];

#[derive(Default)]
struct RewardState {
    queued: bool,
    wait: f32,
    elapsed: Option<f32>,
    reward: Option<Entity>,
    claimed: bool,
}
#[derive(Resource, Default)]
pub(super) struct CaptureRewards {
    states: [RewardState; 2],
    latched: bool,
}
impl CaptureRewards {
    pub(super) fn queue(&mut self, index: usize, seconds: f32) {
        if !self.states[index].queued {
            self.states[index].queued = true;
            self.states[index].wait = seconds;
        }
    }
}
#[derive(Component)]
struct Chest {
    index: usize,
    source: Handle<Gltf>,
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
struct Reward {
    index: usize,
}
pub(super) struct CaptureRewardsPlugin;
impl Plugin for CaptureRewardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CaptureRewards>()
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
    extras: Query<(Entity, &bevy::gltf::GltfExtras)>,
    children: Query<&Children>,
) {
    *rewards = default();
    for (entity, metadata) in &extras {
        if serde_json::from_str::<serde_json::Value>(&metadata.value)
            .ok()
            .is_some_and(|v| {
                v["static_instance"] == true && v["source_model"] == "p6_zm_tm_challenge_box"
            })
        {
            commands.entity(entity).insert(Visibility::Hidden);
            for child in children.iter_descendants(entity) {
                commands.entity(child).insert(ColliderDisabled);
            }
        }
    }
    if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_challenge_box") {
        for (index, position) in ORIGINS.into_iter().enumerate() {
            commands.spawn((
                SessionEntity,
                Chest {
                    index,
                    source: server.load(scene.split('#').next().unwrap_or(scene).to_owned()),
                    glow: None,
                    sound: None,
                    glow_ramps: Vec::new(),
                },
                crate::rendering::NativeArtInputs::default(),
                crate::rendering::opaque::NativeShaderStart(0.),
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(position)
                    .with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
            ));
        }
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut rewards: ResMut<CaptureRewards>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut chests: Query<(Entity, &mut Chest, &mut crate::rendering::NativeArtInputs)>,
    mut visuals: Query<(&Reward, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (index, state) in rewards.states.iter_mut().enumerate() {
        if !state.queued {
            continue;
        }
        if state.elapsed.is_none() {
            state.wait -= dt;
            if state.wait <= 0. {
                state.elapsed = Some(0.);
            } else {
                continue;
            }
        }
        let elapsed = state.elapsed.as_mut().unwrap();
        *elapsed += dt;
        if *elapsed >= 0.5 && state.reward.is_none() && !state.claimed {
            let scene = if index == 0 {
                game.content
                    .powerups
                    .iter()
                    .find(|p| p.id.0 == "powerup.double_points")
                    .and_then(|p| {
                        game.content
                            .presentations
                            .iter()
                            .find(|v| p.presentation.as_ref() == Some(&v.id))
                    })
                    .and_then(|p| p.scene.as_ref())
            } else {
                crate::assets::native_scene(&game, "t6_wpn_zmb_staff_tip_fire_world")
            };
            if let Some(scene) = scene {
                let entity = commands
                    .spawn((
                        SessionEntity,
                        Reward { index },
                        WorldAssetRoot(server.load(scene.clone())),
                        Transform::from_translation(ORIGINS[index]),
                        Visibility::Inherited,
                    ))
                    .id();
                state.reward = Some(entity);
                crate::audio::emit(
                    &mut commands,
                    "zmb_spawn_powerup",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
    }
    for (reward, mut pose) in &mut visuals {
        let elapsed = rewards.states[reward.index].elapsed.unwrap_or(0.);
        pose.translation =
            ORIGINS[reward.index] + Vec3::Y * ((elapsed - 0.5).clamp(0., 2.) * 0.635);
        pose.rotate_y(dt);
    }
    for (entity, mut chest, mut inputs) in &mut chests {
        chest
            .glow_ramps
            .retain_mut(|ramp| ramp.tick(dt, &mut inputs));
        let elapsed = rewards.states[chest.index].elapsed.unwrap_or(100.);
        let glowing = (0.75..2.25).contains(&elapsed);
        if glowing && chest.glow.is_none() {
            inputs.script_vectors[1] = [0.; 4];
            inputs.script_available |= 1 << 1;
            chest.glow_ramps.push(BoxGlowRamp::new(true));
            chest.glow = Some(crate::native_fx::attach(
                &mut commands,
                "maps/zombie_tomb/fx_tomb_challenge_fire",
                entity,
                Transform::default(),
            ));
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_footprintbox_glow_lp",
                crate::audio::SoundSource::attached(entity),
            );
            commands.entity(sound).insert(crate::audio::SoundFade {
                remaining: 1.,
                duration: 1.,
                out: false,
            });
            chest.sound = Some(sound);
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
        let (clip, seconds) = if elapsed < 1.5 {
            ("o_zombie_dlc4_challenge_box_open", elapsed)
        } else {
            ("o_zombie_dlc4_challenge_box_close", elapsed - 1.5)
        };
        commands
            .entity(entity)
            .insert(crate::animation::NativeAnimation {
                source: chest.source.clone(),
                clip: clip.into(),
                seconds,
                looping: false,
                restart: 0,
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
    server: Res<AssetServer>,
    spatial: SpatialQuery,
    targets: Query<(Entity, &Reward, &Transform)>,
    players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        Option<&crate::actions::HandReservation>,
        Has<crate::perks::PerkDrinking>,
    )>,
    mut parts: ResMut<crate::environment::parts::Parts>,
    mut pickups: MessageWriter<crate::powerups::DronePickup>,
) {
    if !input.use_held {
        rewards.latched = false;
    }
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, reservation, drinking)) = players.single() else {
        return;
    };
    if health.state() != LifeState::Standing || reservation.is_some() || drinking {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    for (entity, reward, position) in &targets {
        let state = &rewards.states[reward.index];
        if state.claimed || state.elapsed.is_none_or(|t| t < 2.5) {
            continue;
        }
        let delta = position.translation - eye;
        let Ok(direction) = Dir3::new(delta) else {
            continue;
        };
        if delta.length() > 2.54
            || forward.dot(*direction) < 0.7
            || spatial
                .cast_ray(
                    eye,
                    direction,
                    (delta.length() - 0.1).max(0.),
                    true,
                    &crate::environment::solids([owner]),
                )
                .is_some()
        {
            continue;
        }
        focus.target = Some(entity);
        focus.prompt = format!(
            "Press {} to take {}",
            settings.label(crate::input::Action::Interact),
            if reward.index == 0 {
                "Double Points"
            } else {
                "Fire Staff part"
            }
        );
        if input.use_held && !rewards.latched {
            if reward.index == 0 {
                if let Some(pickup) = crate::powerups::spawn_named(
                    &mut commands,
                    &game,
                    &server,
                    "double_points",
                    position.translation,
                ) {
                    pickups.write(crate::powerups::DronePickup { owner, pickup });
                }
            } else {
                parts.pieces.insert(
                    "part.elemental_staff_fire.upper_staff".into(),
                    crate::environment::parts::PartState::Owned {
                        player: owner.to_bits(),
                    },
                );
            }
            rewards.states[reward.index].claimed = true;
            rewards.latched = true;
            commands.entity(entity).despawn();
            focus.accepted += 1;
        }
        return;
    }
}
