use crate::{
    GameState,
    assets::GameDefinition,
    economy::{PointLedger, TransactionId},
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    origins::PerkId,
    player::Player,
    solo_revive::PerkInventory,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::{prelude::*, world_serialization::WorldAssetRoot};

const MACHINES: [[f32; 3]; 6] = [
    [-6.8072, -7.5184, -94.2848],
    [23.5712, 7.6708, 87.1728],
    [67.564, 3.0988, -1.940306],
    [-59.944, 5.842, -4.4704],
    [19.6088, -3.2512, -64.6176],
    [75.3872, -9.3472, -136.3472],
];
#[derive(Component)]
pub(crate) struct Machine(usize);
#[derive(Component)]
struct MachineVisual {
    source: Option<Handle<Gltf>>,
    active: bool,
    mode: u8,
    age: f32,
    restart: u64,
    location_clock: f32,
    light_mode: u8,
    lights: Vec<Entity>,
    ball: Option<Entity>,
    roll: u32,
    cycling: bool,
    cycle_age: f32,
    pulse_clock: f32,
    motor: Option<Entity>,
    vortex: Option<Entity>,
    vortex_stop: Option<f32>,
    steam: f32,
    steam_clock: f32,
}
#[derive(Clone, Debug)]
pub enum BottlePhase {
    Idle,
    Rolling {
        owner: Entity,
        perk: PerkId,
        remaining: f32,
        bear: bool,
        purchase: TransactionId,
    },
    Available {
        owner: Entity,
        perk: PerkId,
        remaining: f32,
        purchase: TransactionId,
    },
    Moving(f32),
}
#[derive(Resource)]
pub struct Wunderfizz {
    pub active: usize,
    pub phase: BottlePhase,
    pub rolls: u32,
    pub collected: u32,
    pub expired: u32,
    pub moves: u32,
    uses: u32,
    move_after: u32,
    latched: bool,
    rotation: Vec<PerkId>,
}
impl Default for Wunderfizz {
    fn default() -> Self {
        Self {
            active: 2,
            phase: BottlePhase::Idle,
            rolls: 0,
            collected: 0,
            expired: 0,
            moves: 0,
            uses: 0,
            move_after: 4,
            latched: false,
            rotation: Vec::new(),
        }
    }
}
#[derive(Component)]
pub struct PerkBottle {
    pub perk: PerkId,
    pub remaining: f32,
}
pub struct VendorPlugin;
impl Plugin for VendorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Wunderfizz>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                (
                    tick,
                    interact,
                    ground_bottles,
                    presentation,
                    machine_presentation,
                )
                    .chain()
                    .in_set(GameStep::Interactions)
                    .after(crate::tank::interact)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                PostUpdate,
                ball_visibility
                    .in_set(crate::animation::AnimationPoseModifiers)
                    .after(bevy::app::AnimationSystems)
                    .before(bevy::transform::TransformSystems::Propagate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut state: ResMut<Wunderfizz>,
    mut session: ResMut<Session>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    *state = default();
    state.active = 2 + session.random() as usize % 2;
    state.move_after = 4 + session.random() % 3;
    state.rotation = [
        "juggernog",
        "quick_revive",
        "speed_cola",
        "double_tap",
        "stamin_up",
        "deadshot",
        "mule_kick",
        "phd_flopper",
        "electric_cherry",
    ]
    .into_iter()
    .map(|id| PerkId(format!("perk.{id}")))
    .collect();
    for (i, position) in MACHINES.iter().enumerate() {
        let mut entity = commands.spawn((
            Machine(i),
            SessionEntity,
            Transform::from_translation(Vec3::from_array(*position)).with_rotation(
                Quat::from_rotation_y([0_f32, 0., 210., 270., 315., 270.][i].to_radians()),
            ),
            Visibility::Inherited,
        ));
        let mut source = None;
        if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_vending_diesel_magic") {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
            source = Some(server.load(scene.split('#').next().unwrap_or(scene).to_owned()));
        }
        entity.insert(MachineVisual {
            source,
            active: false,
            mode: u8::MAX,
            age: 0.,
            restart: 0,
            location_clock: 0.,
            light_mode: u8::MAX,
            lights: Vec::new(),
            ball: None,
            roll: 0,
            cycling: false,
            cycle_age: 0.,
            pulse_clock: 0.,
            motor: None,
            vortex: None,
            vortex_stop: None,
            steam: 0.,
            steam_clock: 0.,
        });
        commands.spawn((
            VendorBottle {
                index: i,
                scene: None,
            },
            SessionEntity,
            Transform::default(),
            Visibility::Hidden,
        ));
    }
}
fn powered(
    state: &Wunderfizz,
    game: &GameDefinition,
    generators: &crate::generators::Generators,
) -> bool {
    powered_index(state.active, game, generators)
}
fn powered_index(
    index: usize,
    game: &GameDefinition,
    generators: &crate::generators::Generators,
) -> bool {
    let point = Vec3::from_array(MACHINES[index]);
    game.content
        .generators
        .iter()
        .filter_map(|g| {
            game.content
                .placements
                .iter()
                .find(|p| p.id == g.placement)
                .map(|p| (&g.id, Vec3::from_array(p.position).distance_squared(point)))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .is_some_and(|(id, _)| generators.powered(id))
}
fn eligible(
    game: &GameDefinition,
    perks: &PerkInventory,
    id: &PerkId,
    pending: Option<&PerkId>,
) -> bool {
    if perks.owned.contains(id) || pending == Some(id) {
        return false;
    }
    if *id == game.solo_revive.perk {
        return perks.can_buy_solo(&game.solo_revive).is_ok();
    }
    game.content
        .perks
        .iter()
        .any(|perk| perk.id == *id && perk.has_supported_effect())
}
fn roll(
    game: &GameDefinition,
    perks: &PerkInventory,
    state: &mut Wunderfizz,
    session: &mut Session,
) -> Option<PerkId> {
    let mut keys: Vec<_> = (0..state.rotation.len()).collect();
    crate::economy::source_shuffle(&mut keys, session);
    let mut extra = Vec::new();
    for id in [
        "double_tap",
        "deadshot",
        "mule_kick",
        "phd_flopper",
        "electric_cherry",
    ] {
        if session.random() % 4 == 0 {
            extra.insert(0, PerkId(format!("perk.{id}")));
        }
    }
    crate::economy::source_shuffle(&mut extra, session);
    crate::economy::source_shuffle(&mut state.rotation, session);
    state.rotation.extend(extra);
    state
        .rotation
        .iter()
        .find(|id| eligible(game, perks, id, None))
        .cloned()
}
#[derive(Component)]
struct VendorBottle {
    index: usize,
    scene: Option<String>,
}
fn bottle_scene<'a>(game: &'a GameDefinition, perk: &PerkId) -> Option<&'a String> {
    let bottle = game
        .content
        .perks
        .iter()
        .find(|p| p.id == *perk)?
        .drink
        .as_ref()?;
    let weapon = game
        .content
        .weapons
        .iter()
        .find(|w| w.id == bottle.bottle)?;
    game.content
        .presentations
        .iter()
        .find(|p| Some(&p.id) == weapon.pickup.as_ref())?
        .scene
        .as_ref()
}
fn presentation(
    mut commands: Commands,
    state: Res<Wunderfizz>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut bottles: Query<(Entity, &mut VendorBottle, &mut Transform, &mut Visibility)>,
) {
    for (entity, mut bottle, mut pose, mut visibility) in &mut bottles {
        let (perk, elapsed, rolling, bear) = match &state.phase {
            BottlePhase::Rolling {
                perk,
                remaining,
                bear,
                ..
            } if bottle.index == state.active => (
                perk,
                (if *bear { 7. } else { 4. }) - *remaining,
                true,
                *bear,
            ),
            BottlePhase::Available {
                perk, remaining, ..
            } if bottle.index == state.active => (perk, 4. + 10. - *remaining, false, false),
            _ => {
                visibility.set_if_neq(Visibility::Hidden);
                continue;
            }
        };
        if elapsed < 1. {
            visibility.set_if_neq(Visibility::Hidden);
            continue;
        }
        let selected = if rolling && elapsed < 4. {
            &state.rotation[(((elapsed - 1.) / 0.2) as usize) % state.rotation.len()]
        } else {
            perk
        };
        let scene = if bear && elapsed >= 4. {
            crate::assets::native_scene(&game, "t6_wpn_zmb_perk_bottle_bear_world")
        } else {
            bottle_scene(&game, selected)
        };
        if let Some(scene) = scene {
            if bottle.scene.as_ref() != Some(scene) {
                commands
                    .entity(entity)
                    .insert(WorldAssetRoot(server.load(scene.clone())));
                bottle.scene = Some(scene.clone());
            }
            visibility.set_if_neq(Visibility::Inherited);
        }
        let yaw = [0_f32, 0., 210., 270., 315., 270.][bottle.index].to_radians();
        let movement = Quat::from_rotation_y(yaw) * Vec3::Z * 0.254;
        let outward = elapsed < 4.;
        let clock = if outward {
            (elapsed - 1.) / 3.
        } else {
            (elapsed - 4.) / 10.
        };
        let eased = source_motion(clock.clamp(0., 1.));
        let fraction = if outward { eased } else { 1. - eased };
        pose.translation = Vec3::from_array(MACHINES[bottle.index]) + Vec3::Y * (53. * 0.0254)
            - movement * (1. - fraction);
        pose.rotation = Quat::from_rotation_y(
            yaw + if outward {
                eased * std::f32::consts::TAU * 2.
            } else {
                eased * std::f32::consts::FRAC_PI_2
            },
        );
    }
}
fn source_motion(t: f32) -> f32 {
    if t < 0.5 {
        4. / 3. * t * t
    } else {
        (4. * t - 1.) / 3.
    }
}
fn machine_presentation(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    state: Res<Wunderfizz>,
    game: Res<GameDefinition>,
    generators: Res<crate::generators::Generators>,
    mut session: ResMut<Session>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut machines: Query<(
        Entity,
        &Machine,
        &Transform,
        &mut MachineVisual,
        Option<&crate::animation::NativeAnimation>,
    )>,
) {
    let dt = time.delta_secs();
    for (entity, machine, pose, mut visual, animation) in &mut machines {
        let active = state.active == machine.0 && !matches!(state.phase, BottlePhase::Moving(_));
        let cycling = active
            && matches!(
                state.phase,
                BottlePhase::Rolling { .. } | BottlePhase::Available { .. }
            );
        let leaving = active
            && matches!(state.phase, BottlePhase::Rolling { bear: true, remaining, .. } if remaining <= 3.);
        visual.age += dt;
        if active && !visual.active {
            visual.location_clock = 0.;
        }
        if !active && visual.active {
            visual.steam = 5.;
            visual.steam_clock = 0.;
        }
        let mode = if !active || leaving {
            3
        } else if cycling {
            2
        } else if !visual.active || (visual.mode == 1 && visual.age < 9.) {
            1
        } else {
            0
        };
        if mode != visual.mode {
            visual.mode = mode;
            visual.age = 0.;
            visual.restart += 1;
        }
        if mode == 1 && visual.age >= 5. && visual.age - dt < 5. {
            crate::audio::emit(
                &mut commands,
                "zmb_rand_perk_ball_drop",
                crate::audio::SoundSource::attached(entity),
            );
        }
        if let Some(source) = &visual.source {
            let (clip, duration, looping) = match mode {
                1 => ("turn_on", 9., false),
                2 => ("ballspin_loop", 1.4666667, true),
                3 => ("turn_off", 3.4666667, false),
                _ => ("on_idle", 5.9666667, true),
            };
            let request = crate::animation::NativeAnimation {
                source: source.clone(),
                clip: format!("o_zombie_dlc4_vending_diesel_{clip}"),
                seconds: if looping {
                    visual.age % duration
                } else {
                    visual.age.min(duration)
                },
                looping,
                restart: visual.restart,
            };
            if animation != Some(&request) {
                commands.entity(entity).insert(request);
            }
        }
        let light = if !powered_index(machine.0, &game, &generators) {
            0
        } else if active {
            2
        } else {
            1
        };
        if light != visual.light_mode {
            for old in visual.lights.drain(..) {
                commands.entity(old).despawn();
            }
            if let Some(old) = visual.ball.take() {
                commands.entity(old).despawn();
            }
            if light > 0 {
                visual.lights.push(crate::native_fx::attach(
                    &mut commands,
                    if light == 2 {
                        "maps/zombie_tomb/fx_tomb_dieselmagic_light_green"
                    } else {
                        "maps/zombie_tomb/fx_tomb_dieselmagic_light_red"
                    },
                    entity,
                    Transform::default(),
                ));
            }
            visual.light_mode = light;
        }
        if light == 2 && visual.ball.is_none() {
            if let Some(ball) = children.iter_descendants(entity).find(|child| {
                names
                    .get(*child)
                    .is_ok_and(|name| name.as_str() == "j_ball")
            }) {
                visual.ball = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_dieselmagic_light",
                    ball,
                    Transform::default(),
                ));
            }
        }
        if active {
            visual.location_clock -= dt;
            if visual.location_clock <= 0. {
                crate::native_fx::emit(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_dieselmagic_identify",
                    Transform::from_translation(pose.translation),
                );
                visual.location_clock += 3. + session.random() as f32 / u32::MAX as f32;
            }
        }
        if cycling && visual.roll != state.rolls {
            visual.roll = state.rolls;
            visual.cycle_age = 0.;
            visual.pulse_clock = 0.;
            visual.vortex_stop = None;
            crate::audio::emit(
                &mut commands,
                "zmb_rand_perk_start",
                crate::audio::SoundSource::attached(entity),
            );
            let sound = crate::audio::emit_loop(
                &mut commands,
                "zmb_rand_perk_loop",
                crate::audio::SoundSource::attached(entity),
            );
            commands.entity(sound).insert(crate::audio::SoundFade {
                remaining: 0.5,
                duration: 0.5,
                out: false,
            });
            visual.motor = Some(sound);
            crate::audio::emit(
                &mut commands,
                "zmb_rand_perk_vortex_sparks",
                crate::audio::SoundSource::attached(entity),
            );
        }
        if visual.motor.is_some() && (!cycling || leaving) {
            let sound = visual.motor.take().unwrap();
            commands.entity(sound).insert(crate::audio::SoundFade {
                remaining: 1.,
                duration: 1.,
                out: true,
            });
            crate::audio::emit(
                &mut commands,
                if leaving || !active {
                    "zmb_rand_perk_leave"
                } else {
                    "zmb_rand_perk_stop"
                },
                crate::audio::SoundSource::attached(entity),
            );
        }
        if cycling {
            visual.cycle_age += dt;
            if visual.cycle_age >= 0.5 && visual.vortex.is_none() {
                visual.vortex = Some(crate::audio::emit_loop(
                    &mut commands,
                    "zmb_rand_perk_vortex",
                    crate::audio::SoundSource::attached(entity),
                ));
            }
        }
        if visual.cycling && !cycling {
            crate::audio::emit(
                &mut commands,
                "zmb_rand_perk_vortex_sparks",
                crate::audio::SoundSource::attached(entity),
            );
            visual.vortex_stop = Some(0.5);
        }
        let electricity = cycling || visual.vortex_stop.is_some();
        if electricity {
            visual.pulse_clock -= dt;
            if visual.pulse_clock <= 0. {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_dieselmagic_on",
                    entity,
                    Transform::default(),
                );
                if cycling && visual.cycle_age >= 0.5 {
                    crate::native_fx::attach(
                        &mut commands,
                        "maps/zombie_tomb/fx_tomb_dieselmagic_portal",
                        entity,
                        Transform::default(),
                    );
                }
                visual.pulse_clock += 0.1;
            }
        }
        if let Some(remaining) = visual.vortex_stop.as_mut() {
            *remaining -= dt;
            if *remaining <= 0. {
                if let Some(sound) = visual.vortex.take() {
                    commands.entity(sound).despawn();
                }
                visual.vortex_stop = None;
            }
        }
        if visual.steam > 0. {
            visual.steam -= dt;
            visual.steam_clock -= dt;
            if visual.steam_clock <= 0. {
                crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_dieselmagic_steam",
                    entity,
                    Transform::default(),
                );
                visual.steam_clock += 0.1;
            }
        }
        visual.active = active;
        visual.cycling = cycling;
    }
}
fn ball_visibility(
    state: Res<Wunderfizz>,
    machines: Query<(Entity, &Machine)>,
    children: Query<&Children>,
    mut bones: Query<(&Name, &mut Transform)>,
) {
    for (entity, machine) in &machines {
        let active = machine.0 == state.active && !matches!(state.phase, BottlePhase::Moving(_));
        for child in children.iter_descendants(entity) {
            if let Ok((name, mut pose)) = bones.get_mut(child)
                && name.as_str() == "j_ball"
            {
                let scale = Vec3::splat(if active { 1. } else { 0. });
                if pose.scale != scale {
                    pose.scale = scale;
                }
            }
        }
    }
}
fn tick(
    time: Res<Time<Fixed>>,
    mut state: ResMut<Wunderfizz>,
    mut session: ResMut<Session>,
    mut ledger: ResMut<PointLedger>,
    players: Query<&PerkInventory, With<Player>>,
) {
    let dt = time.delta_secs();
    state.phase = match state.phase.clone() {
        BottlePhase::Rolling {
            owner,
            perk,
            mut remaining,
            bear,
            purchase,
        } => {
            remaining -= dt;
            if remaining > 0. {
                BottlePhase::Rolling {
                    owner,
                    perk,
                    remaining,
                    bear,
                    purchase,
                }
            } else if bear {
                ledger.refund(&mut session.points, purchase);
                BottlePhase::Moving(10.)
            } else {
                BottlePhase::Available {
                    owner,
                    perk,
                    remaining: 10.,
                    purchase,
                }
            }
        }
        BottlePhase::Available {
            owner,
            perk,
            mut remaining,
            purchase,
        } => {
            remaining -= dt;
            if remaining <= 0.
                || players
                    .get(owner)
                    .is_ok_and(|p| p.owned.len() >= p.purchase_limit())
            {
                if players
                    .get(owner)
                    .is_ok_and(|p| p.owned.len() >= p.purchase_limit())
                {
                    ledger.refund(&mut session.points, purchase);
                }
                state.expired += 1;
                BottlePhase::Idle
            } else {
                BottlePhase::Available {
                    owner,
                    perk,
                    remaining,
                    purchase,
                }
            }
        }
        BottlePhase::Moving(mut remaining) => {
            remaining -= dt;
            if remaining <= 0. {
                let previous = state.active;
                state.active = (previous + 1 + session.random() as usize % (MACHINES.len() - 1))
                    % MACHINES.len();
                state.uses = 0;
                state.move_after = 4 + session.random() % 3;
                state.moves += 1;
                BottlePhase::Idle
            } else {
                BottlePhase::Moving(remaining)
            }
        }
        BottlePhase::Idle => BottlePhase::Idle,
    };
}
pub(crate) fn interact(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    game: Res<GameDefinition>,
    generators: Res<crate::generators::Generators>,
    query: SpatialQuery,
    mut state: ResMut<Wunderfizz>,
    mut focus: ResMut<InteractionFocus>,
    mut session: ResMut<Session>,
    mut ledger: ResMut<PointLedger>,
    reserved: Query<(), With<crate::actions::HandReservation>>,
    machines: Query<(Entity, &Machine, &Transform), Without<Player>>,
    mut players: Query<
        (
            Entity,
            &Player,
            &Transform,
            &Vitality,
            &PerkInventory,
            &mut WeaponInventory,
            Has<super::PerkDrinking>,
            &crate::equipment::TacticalInventory,
            &crate::equipment::GrenadeInventory,
            &crate::mines::MineInventory,
        ),
        With<Player>,
    >,
) {
    if !input.use_held {
        state.latched = false;
    }
    let Ok((
        owner,
        player,
        pose,
        health,
        perks,
        mut inventory,
        drinking,
        tactical,
        grenades,
        mines,
    )) = players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing || focus.target.is_some() {
        return;
    }
    let eye = pose.translation + Vec3::Y * (player.eye_height - player.height * 0.5);
    let forward = Quat::from_euler(EulerRot::YXZ, player.yaw, player.pitch, 0.) * Vec3::NEG_Z;
    let candidate = machines
        .iter()
        .filter_map(|(entity, machine, t)| {
            let delta = t.translation + Vec3::Y * 1.2 - eye;
            let distance = delta.length();
            let direction = Dir3::new(delta).ok()?;
            (distance < 2.4
                && forward.dot(*direction) > 0.8
                && query
                    .cast_ray(
                        eye,
                        direction,
                        (distance - 0.1).max(0.),
                        true,
                        &crate::environment::solids([owner]),
                    )
                    .is_none())
            .then_some((entity, machine.0, distance))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2));
    let Some((entity, index, _)) = candidate else {
        return;
    };
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    if index != state.active {
        focus.prompt = "Wunderfizz is at another location".into();
        return;
    }
    if !powered(&state, &game, &generators) {
        focus.prompt = "Wunderfizz requires generator power".into();
        focus.rejection = Some(InteractionRejection::NoPower);
        return;
    }
    let key = settings.label(crate::input::Action::Interact);
    let pressed = input.use_held && !state.latched;
    if pressed {
        state.latched = true;
    }
    let busy = drinking
        || !matches!(
            inventory.action,
            WeaponAction::Ready | WeaponAction::Reloading { .. } | WeaponAction::ShellReload { .. }
        )
        || tactical.busy()
        || grenades.busy()
        || mines.busy()
        || reserved.contains(owner);
    match state.phase.clone() {
        BottlePhase::Idle => {
            focus.prompt = format!("Press {key} for Wunderfizz [Cost: 1500]");
            if !pressed {
                return;
            }
            if busy {
                focus.rejection = Some(InteractionRejection::Busy);
                return;
            }
            if perks.owned.len() >= perks.purchase_limit() {
                focus.rejection = Some(InteractionRejection::PerkLimit);
                return;
            }
            if !state
                .rotation
                .iter()
                .any(|id| eligible(&game, perks, id, None))
            {
                focus.rejection = Some(InteractionRejection::Unavailable);
                return;
            }
            let purchase = ledger.command(1500);
            if let Err(error) = ledger.accept(&mut session.points, purchase, || {
                Ok::<_, InteractionRejection>(())
            }) {
                focus.rejection = Some(error.into());
                return;
            }
            let perk = roll(&game, perks, &mut state, &mut session).expect("eligible vendor perk");
            state.uses += 1;
            state.rolls += 1;
            let bear = state.uses >= state.move_after;
            state.phase = BottlePhase::Rolling {
                owner,
                perk,
                remaining: if bear { 7. } else { 4. },
                bear,
                purchase: purchase.id,
            };
            focus.accepted += 1;
        }
        BottlePhase::Available {
            owner: buyer,
            perk,
            remaining,
            ..
        } => {
            let Some(definition) = game.content.perks.iter().find(|p| p.id == perk) else {
                return;
            };
            focus.prompt = format!("Press {key} to take {} [{remaining:.0}s]", definition.name);
            if !pressed || buyer != owner || busy {
                return;
            }
            if perks.owned.len() >= perks.purchase_limit() {
                focus.rejection = Some(InteractionRejection::PerkLimit);
                return;
            }
            match super::PerkDrinking::new(definition, &inventory, &game) {
                Ok(drink) => {
                    inventory.cancel_action();
                    commands.entity(owner).insert(drink);
                    state.phase = BottlePhase::Idle;
                    state.collected += 1;
                    focus.accepted += 1;
                }
                Err(error) => focus.rejection = Some(error),
            }
        }
        BottlePhase::Rolling { .. } => focus.prompt = "Wunderfizz rolling".into(),
        BottlePhase::Moving(_) => focus.prompt = "Wunderfizz relocating".into(),
    }
}
fn ground_bottles(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    mut bottles: Query<(Entity, &mut PerkBottle, &Transform), Without<Player>>,
    players: Query<
        (
            Entity,
            &Vitality,
            &Transform,
            &PerkInventory,
            &WeaponInventory,
            Has<super::PerkDrinking>,
        ),
        With<Player>,
    >,
) {
    let mut collecting = std::collections::HashSet::new();
    for (entity, mut bottle, pose) in &mut bottles {
        bottle.remaining -= time.delta_secs();
        if bottle.remaining <= 0. {
            commands.entity(entity).despawn();
            continue;
        }
        for (owner, health, body, perks, inventory, drinking) in &players {
            if collecting.contains(&owner)
                || health.state() != LifeState::Standing
                || drinking
                || perks.owned.contains(&bottle.perk)
                || body.translation.distance_squared(pose.translation) > 2.25
            {
                continue;
            }
            if let Some(perk) = game.content.perks.iter().find(|p| p.id == bottle.perk)
                && let Ok(drink) = super::PerkDrinking::new(perk, inventory, &game)
            {
                collecting.insert(owner);
                commands.entity(owner).insert(drink);
                commands.entity(entity).despawn();
                break;
            }
        }
    }
}

pub(crate) fn grant_random_perk(
    game: &GameDefinition,
    perks: &mut PerkInventory,
    pending: Option<&PerkId>,
    session: &mut Session,
) -> Option<PerkId> {
    let mut candidates: Vec<_> = game
        .content
        .interactions
        .iter()
        .filter_map(|site| {
            let crate::origins::InteractionAction::Perk { perk } = &site.action else {
                return None;
            };
            eligible(game, perks, perk, pending).then_some(perk.clone())
        })
        .collect();
    crate::economy::source_shuffle(&mut candidates, session);
    let perk = candidates.first()?.clone();
    if perk == game.solo_revive.perk {
        perks.buy_solo(&game.solo_revive).ok()?;
    } else {
        perks.owned.insert(perk.clone());
    }
    Some(perk)
}
