use crate::{
    GameState,
    assets::GameDefinition,
    damage::{DamageQueue, Impact, ImpactSource},
    gameplay::{GameStep, Session, SessionEntity},
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection},
    life::{LifeState, Vitality},
    origins::{DamageCategory, WeaponId},
    player::Player,
    weapons::{HitRegion, WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;
mod source;
#[derive(Resource, Default)]
pub struct Rituals {
    pub generators: u32,
    pub chests: u32,
    pub claimed: [bool; 4],
    pub souls: [u32; 4],
    pub complete: [bool; 4],
    pub resets: u32,
    weapon_reward: Option<WeaponId>,
    selected: usize,
    latched: bool,
}
impl Rituals {
    pub fn eligible(&self, index: usize, session: &Session) -> bool {
        !self.claimed[index]
            && match index {
                0 => session.challenge_headshots >= 115,
                1 => self.generators >= 6,
                2 => session.spent >= 30000,
                _ => self.chests >= 4,
            }
    }

    fn packed_weapon<'a>(
        &mut self,
        weapons: &'a [crate::origins::WeaponDefinition],
        session: &mut Session,
    ) -> &'a crate::origins::WeaponDefinition {
        if self.weapon_reward.is_none() {
            let bases = ["weapon.scar_zm", "weapon.galil_zm", "weapon.mp44_zm"];
            let selected = bases[session.random() as usize % bases.len()];
            self.weapon_reward = Some(
                weapons
                    .iter()
                    .find(|weapon| weapon.id.0 == selected)
                    .and_then(|weapon| weapon.upgraded.clone())
                    .expect("Origins headshot challenge upgraded weapon"),
            );
        }
        weapons
            .iter()
            .find(|weapon| Some(&weapon.id) == self.weapon_reward.as_ref())
            .expect("Origins headshot challenge weapon definition")
    }
}
#[derive(Component, Default)]
pub struct IronFists {
    pub unlocked: bool,
    pub upgraded: bool,
    pub element: Option<crate::quests::Element>,
    pub(crate) souls: u32,
}
#[derive(Component)]
pub(crate) struct ChallengeBox;
#[derive(Component)]
pub(crate) struct SoulChest(usize);
#[derive(Component)]
pub(crate) struct SoulBoxGlow {
    elapsed: f32,
    pending: Vec<(f32, bool)>,
    active: bool,
    ramps: Vec<crate::generators::BoxGlowRamp>,
    effect: Option<Entity>,
}
impl Default for SoulBoxGlow {
    fn default() -> Self {
        Self {
            elapsed: 0.,
            pending: vec![(1., true), (2., false)],
            active: false,
            ramps: Vec::new(),
            effect: None,
        }
    }
}
impl SoulBoxGlow {
    fn queue(&mut self, delay: f32, active: bool) {
        self.pending.push((self.elapsed + delay, active));
    }

    fn advance(
        &mut self,
        dt: f32,
        inputs: &mut crate::presentation::cues::CueParameters,
    ) -> Vec<bool> {
        self.ramps.retain_mut(|ramp| ramp.tick(dt, inputs));
        self.elapsed += dt;
        self.pending.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut changed = Vec::new();
        while self
            .pending
            .first()
            .is_some_and(|(time, _)| *time <= self.elapsed)
        {
            let (_, active) = self.pending.remove(0);
            if self.active != active {
                self.active = active;
                inputs.luminous_channel = Some((1, 0));
                inputs.script_vectors[1] = [0.; 4];
                inputs.script_available |= 1 << 1;
                self.ramps.push(crate::generators::BoxGlowRamp::new(active));
                changed.push(active);
            }
        }
        changed
    }
}
pub struct RitualPlugin;
impl Plugin for RitualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Rituals>()
            .register_required_components::<Player, IronFists>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                setup.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                track
                    .in_set(GameStep::Rewards)
                    .after(crate::damage::reward_deaths)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                soul_box_glow.after(crate::quests::souls).run_if(
                    in_state(GameState::StartingSession).or_else(in_state(GameState::Playing)),
                ),
            )
            .add_systems(
                FixedUpdate,
                claim
                    .in_set(GameStep::Interactions)
                    .after(crate::robots::evacuate)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                punches
                    .in_set(GameStep::Combat)
                    .before(crate::gameplay::combat)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn setup(
    mut commands: Commands,
    mut rituals: ResMut<Rituals>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
) {
    *rituals = default();
    for (i, (position, _, _)) in source::BOXES.iter().enumerate() {
        let mut entity = commands.spawn((
            SessionEntity,
            SoulChest(i),
            SoulBoxGlow::default(),
            crate::presentation::cues::CueParameters::default(),
            Transform::from_translation(Vec3::from_array(*position)).with_rotation(
                Quat::from_rotation_y(-[94.8_f32, 355., 180., 170.][i].to_radians()),
            ),
            Visibility::Inherited,
        ));
        if let Some(scene) = crate::assets::native_scene(&game, "p6_zm_tm_soul_box") {
            entity.insert(WorldAssetRoot(server.load(scene.clone())));
        }
    }
    for position in [
        [68.723002, -7.9248, -118.373398],
        [33.632902, 7.6708, 94.288102],
    ] {
        commands.spawn((
            SessionEntity,
            ChallengeBox,
            Transform::from_translation(Vec3::from_array(position)),
        ));
    }
}
fn soul_box_glow(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut boxes: Query<(
        Entity,
        &mut SoulBoxGlow,
        &mut crate::presentation::cues::CueParameters,
    )>,
) {
    for (entity, mut glow, mut inputs) in &mut boxes {
        for active in glow.advance(time.delta_secs(), &mut inputs) {
            if active {
                glow.effect = Some(crate::native_fx::attach(
                    &mut commands,
                    "maps/zombie_tomb/fx_tomb_challenge_fire",
                    entity,
                    Transform::default(),
                ));
            } else if let Some(effect) = glow.effect.take() {
                commands.entity(effect).try_despawn();
            }
        }
    }
}

pub(crate) fn track(
    mut rituals: ResMut<Rituals>,
    mut captures: MessageReader<crate::generators::GeneratorCaptured>,
    mut stomps: MessageReader<crate::robots::RobotStomp>,
    mut boxes: Query<(Entity, &SoulChest, &mut SoulBoxGlow)>,
    players: Query<Entity, With<Player>>,
) {
    for capture in captures.read() {
        if players.contains(capture.player) {
            rituals.generators += 1;
        }
    }
    for event in stomps.read() {
        for (i, (_, _, center)) in source::BOXES.iter().enumerate() {
            if !rituals.complete[i]
                && event
                    .0
                    .xz()
                    .distance_squared(Vec3::from_array(*center).xz())
                    < 6.5_f32.powi(2)
            {
                rituals.souls[i] = 0;
                rituals.resets += 1;
                for (_, chest, mut glow) in &mut boxes {
                    if chest.0 == i {
                        glow.queue(0., false);
                    }
                }
            }
        }
    }
}

pub(crate) fn collect_soul(
    commands: &mut Commands,
    rituals: &mut Rituals,
    pose: &Transform,
    touched: &[Entity],
    volumes: &Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    game: &GameDefinition,
    boxes: &mut Query<(Entity, &SoulChest, &mut SoulBoxGlow)>,
) -> bool {
    for (i, (_, target, _)) in source::BOXES.iter().enumerate() {
        if rituals.complete[i] {
            continue;
        }
        let inside = touched
            .iter()
            .filter_map(|e| volumes.get(*e).ok())
            .filter(|(_, state)| state.enabled)
            .map(|(volume, _)| volume)
            .any(|volume| {
                game.environment
                    .triggers
                    .iter()
                    .find(|t| t.id == volume.0)
                    .is_some_and(|t| {
                        t.source
                            .get("targetname")
                            .is_some_and(|name| name.as_str() == *target)
                    })
            });
        if !inside {
            continue;
        }
        let first = rituals.souls[i] == 0;
        rituals.souls[i] += 1;
        if first {
            for (_, chest, mut glow) in boxes.iter_mut() {
                if chest.0 == i {
                    glow.queue(1., true);
                }
            }
        }
        if rituals.souls[i] >= 30 {
            rituals.complete[i] = true;
            rituals.chests += 1;
            for (entity, chest, mut glow) in boxes.iter_mut() {
                if chest.0 == i {
                    glow.queue(0.5, false);
                    commands.entity(entity).insert(Visibility::Hidden);
                }
            }
            crate::audio::emit(
                commands,
                "zmb_footprintbox_disappear",
                crate::audio::SoundSource::World(pose.translation),
            );
        }
        return true;
    }
    false
}
pub(crate) fn claim(
    mut commands: Commands,
    input: Res<GameInput>,
    settings: Res<crate::input::ControlSettings>,
    mut rituals: ResMut<Rituals>,
    mut focus: ResMut<InteractionFocus>,
    game: Res<GameDefinition>,
    server: Res<AssetServer>,
    mut session: ResMut<Session>,
    boxes: Query<(Entity, &Transform), (With<ChallengeBox>, Without<Player>)>,
    mut players: Query<
        (
            Entity,
            &Player,
            &Transform,
            &Vitality,
            &mut WeaponInventory,
            &crate::solo_revive::PerkInventory,
            &mut IronFists,
            Has<crate::perks::PerkDrinking>,
        ),
        With<Player>,
    >,
) {
    if !input.use_held {
        rituals.latched = false;
    }
    if focus.target.is_some() {
        return;
    }
    let Ok((owner, player, pose, health, mut inventory, perks, mut fists, drinking)) =
        players.single_mut()
    else {
        return;
    };
    if health.state() != LifeState::Standing {
        return;
    }
    let Some((entity, _)) = boxes
        .iter()
        .find(|(_, t)| t.translation.distance_squared(pose.translation) < 3.0_f32.powi(2))
    else {
        return;
    };
    let available: Vec<_> = (0..4).filter(|i| rituals.eligible(*i, &session)).collect();
    if available.is_empty() {
        return;
    }
    if input.buildable {
        rituals.selected = (rituals.selected + 1) % available.len();
    }
    let selected = available[rituals.selected % available.len()];
    focus.target = Some(entity);
    focus.progress = 0.;
    focus.hold_seconds = 0.;
    let reward = [
        "upgraded weapon",
        "Max Ammo",
        "Double Tap II",
        "One Inch Punch",
    ][selected];
    focus.prompt = format!(
        "Press {} to claim {reward}; {} cycles rewards",
        settings.label(crate::input::Action::Interact),
        settings.label(crate::input::Action::Buildable)
    );
    if !input.use_held || rituals.latched {
        return;
    }
    rituals.latched = true;
    if inventory.action != WeaponAction::Ready || drinking {
        focus.rejection = Some(InteractionRejection::Busy);
        return;
    }
    let accepted = match selected {
        0 => {
            let weapon = rituals.packed_weapon(&game.content.weapons, &mut session);
            let accepted = inventory.receive(weapon).is_ok();
            if accepted {
                inventory.fill_received(weapon);
            }
            accepted
        }
        1 => crate::environment::digging::spawn_powerup(
            &mut commands,
            &server,
            &game,
            "full_ammo",
            pose.translation - Vec3::Y * player.height * 0.5,
        )
        .is_some(),
        2 => {
            if perks.owned.iter().any(|p| p.0 == "perk.double_tap") {
                false
            } else if let Some(perk) = game
                .content
                .perks
                .iter()
                .find(|p| p.id.0 == "perk.double_tap")
                && let Ok(drink) = crate::perks::PerkDrinking::new(perk, &inventory, &game)
            {
                commands.entity(owner).insert(drink);
                true
            } else {
                false
            }
        }
        _ => {
            fists.unlocked = true;
            true
        }
    };
    if accepted {
        rituals.claimed[selected] = true;
        focus.accepted += 1;
        crate::audio::emit(
            &mut commands,
            "zmb_powerup_grabbed",
            crate::audio::SoundSource::attached(owner),
        );
    } else {
        focus.rejection = Some(InteractionRejection::Unavailable);
    }
}
fn punches(
    mut rituals: ResMut<Rituals>,
    mut players: Query<(&Vitality, &mut WeaponInventory, &mut IronFists)>,
) {
    for (health, mut inventory, mut fists) in &mut players {
        if health.state() == LifeState::HumanTerminal && fists.unlocked {
            rituals.claimed[3] = false;
            fists.unlocked = false;
        }
        let weapon = fists.unlocked.then(|| {
            let id = if fists.upgraded {
                match fists.element {
                    Some(crate::quests::Element::Fire) => "weapon.one_inch_punch_fire_zm",
                    Some(crate::quests::Element::Ice) => "weapon.one_inch_punch_ice_zm",
                    Some(crate::quests::Element::Wind) => "weapon.one_inch_punch_air_zm",
                    Some(crate::quests::Element::Lightning) => "weapon.one_inch_punch_lightning_zm",
                    _ => "weapon.one_inch_punch_upgraded_zm",
                }
            } else {
                "weapon.one_inch_punch_zm"
            };
            WeaponId(id.into())
        });
        inventory.set_melee_override(weapon);
    }
}
pub fn punch(
    enemies: &crate::hit_regions::RayTargets,
    zombies: &Query<(), With<crate::enemies::Zombie>>,
    query: &SpatialQuery,
    damage: &mut DamageQueue,
    owner: Entity,
    origin: Vec3,
    yaw: f32,
    weapon: &WeaponId,
) {
    let forward = Quat::from_rotation_y(yaw) * Vec3::NEG_Z;
    let upgraded = weapon.0.contains("upgraded")
        || weapon.0.contains("_fire_")
        || weapon.0.contains("_ice_")
        || weapon.0.contains("_air_")
        || weapon.0.contains("_lightning_");
    let full_squared = 1.6256_f32.powi(2) * if weapon.0.contains("_air_") { 2. } else { 1. };
    for (target, vitality, pose, _) in enemies.iter() {
        let delta = pose.translation - origin;
        if !zombies.contains(target)
            || vitality.state() != LifeState::ActorAlive
            || delta.length_squared() > 2.54_f32.powi(2)
            || delta.with_y(0.).normalize_or_zero().dot(forward) <= 35_f32.to_radians().cos()
        {
            continue;
        }
        if let Ok(direction) = Dir3::new(delta)
            && query
                .cast_ray(
                    origin,
                    direction,
                    (delta.length() - 0.35).max(0.),
                    true,
                    &crate::environment::solids([owner, target]),
                )
                .is_some()
        {
            continue;
        }
        let amount = if upgraded { 11275 } else { 2250 }
            / if delta.length_squared() <= full_squared {
                1
            } else {
                2
            };
        damage.0.push(Impact {
            point: None,
            attacker: owner,
            target,
            source: ImpactSource::Weapon(weapon.clone()),
            category: DamageCategory::Melee,
            region: HitRegion::Torso,
            amount,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bleeding_out_makes_the_completed_one_inch_punch_reward_claimable_again() {
        let mut health = Vitality::human(
            100,
            100,
            50,
            crate::life::PlayerLethalPolicy::SoloOrLastEligiblePlayer,
        )
        .unwrap();
        health.kill_human().unwrap();
        let mut app = App::new();
        app.insert_resource(Rituals {
            chests: 4,
            claimed: [false, false, false, true],
            ..default()
        })
        .add_systems(Update, punches);
        let player = app
            .world_mut()
            .spawn((
                health,
                WeaponInventory::default(),
                IronFists {
                    unlocked: true,
                    souls: 20,
                    ..default()
                },
            ))
            .id();
        app.update();
        assert!(!app.world().get::<IronFists>(player).unwrap().unlocked);
        assert_eq!(app.world().get::<IronFists>(player).unwrap().souls, 20);
        assert!(
            app.world()
                .resource::<Rituals>()
                .eligible(3, &Session::default())
        );
    }

    #[test]
    fn headshot_reward_draws_once_independent_of_weapon_catalog_order() {
        let weapons: Vec<crate::origins::WeaponDefinition> = [
            ("weapon.unrelated", None),
            ("weapon.mp44_zm", Some("weapon.mp44_upgraded_zm")),
            ("weapon.galil_zm", Some("weapon.galil_upgraded_zm")),
            ("weapon.scar_zm", Some("weapon.scar_upgraded_zm")),
            ("weapon.galil_upgraded_zm", None),
            ("weapon.scar_upgraded_zm", None),
            ("weapon.mp44_upgraded_zm", None),
        ]
        .into_iter()
        .map(|(id, upgraded)| {
            serde_json::from_value(serde_json::json!({
                "id": id, "name": id, "damage": 100, "category": "bullet",
                "magazine": 30, "reserve": 180, "fire_interval": 0.1,
                "reload_seconds": 2.0, "upgraded": upgraded
            }))
            .unwrap()
        })
        .collect();
        let expected_weapons = [
            "weapon.scar_upgraded_zm",
            "weapon.galil_upgraded_zm",
            "weapon.mp44_upgraded_zm",
        ];
        for seed in 0..64 {
            let mut session = Session { seed, ..default() };
            let mut expected = session.clone();
            let expected_weapon = expected_weapons[expected.random() as usize % 3];
            let mut rituals = Rituals::default();
            assert_eq!(
                rituals.packed_weapon(&weapons, &mut session).id.0,
                expected_weapon
            );
            assert_eq!(session.seed, expected.seed);
            let mut reordered = weapons.clone();
            reordered.reverse();
            assert_eq!(
                rituals.packed_weapon(&reordered, &mut session).id.0,
                expected_weapon
            );
            assert_eq!(
                session.seed, expected.seed,
                "retry must preserve the selected weapon"
            );
        }
    }

    #[test]
    fn soul_box_material_maps_source_slot_on_startup_souls_completion_and_stomp() {
        let mut glow = SoulBoxGlow::default();
        let mut inputs = crate::presentation::cues::CueParameters::default();
        assert!(glow.advance(0.99, &mut inputs).is_empty());
        assert_eq!(inputs.script_available, 0);
        assert_eq!(glow.advance(0.01, &mut inputs), [true]);
        assert_eq!(inputs.script_available, 1 << 1);
        assert_eq!(inputs.script_vectors[1], [0.; 4]);
        for _ in 0..7 {
            glow.advance(0.016, &mut inputs);
        }
        assert!((inputs.script_vectors[1][0] - 0.888).abs() < 0.000001);
        glow.queue(0., true);
        assert!(glow.advance(0., &mut inputs).is_empty());
        assert!((inputs.script_vectors[1][0] - 0.888).abs() < 0.000001);
        assert_eq!(glow.advance(0.9, &mut inputs), [false]);
        assert_eq!(inputs.script_vectors[1], [0.; 4]);
        glow.queue(1., true);
        assert!(glow.advance(0.99, &mut inputs).is_empty());
        assert_eq!(glow.advance(0.02, &mut inputs), [true]);
        glow.queue(0.5, false);
        assert!(glow.advance(0.49, &mut inputs).is_empty());
        assert_eq!(glow.advance(0.02, &mut inputs), [false]);
        glow.queue(1., true);
        assert_eq!(glow.advance(1., &mut inputs), [true]);
        glow.queue(0., false);
        assert_eq!(glow.advance(0., &mut inputs), [false]);
        assert_eq!(inputs.script_vectors[1], [0.; 4]);
        assert_eq!(inputs.script_available, 1 << 1);
    }
}
