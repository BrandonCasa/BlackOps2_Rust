use crate::{
    GameState,
    assets::GameDefinition,
    damage::{DamageQueue, EnemyDeath, Impact, ImpactSource},
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
    pub headshots: u32,
    pub generators: u32,
    pub spent: u64,
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
    pub fn eligible(&self, index: usize) -> bool {
        !self.claimed[index]
            && match index {
                0 => self.headshots >= 115,
                1 => self.generators >= 6,
                2 => self.spent >= 30000,
                _ => self.chests >= 4,
            }
    }
}
#[derive(Component, Default)]
pub struct IronFists {
    pub unlocked: bool,
    pub upgraded: bool,
    pub element: Option<crate::quests::Element>,
}
#[derive(Component)]
pub(crate) struct ChallengeBox;
#[derive(Component)]
struct SoulChest(usize);
pub struct RitualPlugin;
impl Plugin for RitualPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Rituals>()
            .register_required_components::<Player, IronFists>()
            .add_systems(
                OnEnter(GameState::PreparingShaders),
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
fn track(
    mut commands: Commands,
    mut rituals: ResMut<Rituals>,
    mut deaths: MessageReader<EnemyDeath>,
    mut stomps: MessageReader<crate::robots::RobotStomp>,
    enemies: Query<
        (
            &Transform,
            &Collider,
            Option<&crate::rounds::PopulationKind>,
        ),
        With<crate::enemies::Zombie>,
    >,
    volumes: Query<(
        &crate::environment::triggers::MapTrigger,
        &crate::environment::triggers::TriggerState,
    )>,
    query: SpatialQuery,
    game: Res<GameDefinition>,
    generators: Res<crate::generators::Generators>,
    ledger: Res<crate::economy::PointLedger>,
    boxes: Query<(Entity, &SoulChest)>,
    players: Query<(&Vitality, &IronFists), With<Player>>,
) {
    rituals.generators = generators.completions;
    rituals.spent = ledger.spent;
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
            }
        }
    }
    for death in deaths.read() {
        if death.0.impact.region == HitRegion::Head
            && death.0.impact.source.player_attributed()
            && matches!(death.0.impact.category, DamageCategory::Bullet)
            && enemies
                .get(death.0.impact.target)
                .is_ok_and(|(_, _, kind)| {
                    kind != Some(&crate::rounds::PopulationKind::GeneratorDefender)
                })
        {
            rituals.headshots += 1;
        }
        if !death.0.can_supply_soul() {
            continue;
        }
        let Ok((pose, shape, _)) = enemies.get(death.0.impact.target) else {
            continue;
        };
        let touched = query.shape_intersections(
            shape,
            pose.translation,
            pose.rotation,
            &SpatialQueryFilter::from_mask(crate::environment::triggers::TRIGGER_LAYER),
        );
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
            rituals.souls[i] += 1;
            if rituals.souls[i] >= 30 {
                rituals.complete[i] = true;
                rituals.chests += 1;
                for (entity, chest) in &boxes {
                    if chest.0 == i {
                        commands.entity(entity).insert(Visibility::Hidden);
                    }
                }
                crate::audio::emit(
                    &mut commands,
                    "zmb_footprintbox_disappear",
                    crate::audio::SoundSource::World(pose.translation),
                );
            }
            break;
        }
    }
    if players
        .iter()
        .any(|(v, fists)| v.state() == LifeState::HumanTerminal && fists.unlocked)
    {
        rituals.claimed[3] = false;
    }
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
    let available: Vec<_> = (0..4).filter(|i| rituals.eligible(*i)).collect();
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
            let bases = ["weapon.scar_zm", "weapon.galil_zm", "weapon.mp44_zm"];
            if rituals.weapon_reward.is_none() {
                rituals.weapon_reward = game
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == bases[session.random() as usize % 3])
                    .and_then(|w| w.upgraded.clone());
            }
            let weapon = rituals
                .weapon_reward
                .as_ref()
                .and_then(|id| game.content.weapons.iter().find(|w| w.id == *id));
            weapon.is_some_and(|w| {
                let accepted = inventory.receive(w).is_ok();
                if accepted {
                    inventory.fill_received(w);
                }
                accepted
            })
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
fn punches(mut players: Query<(&Vitality, &mut WeaponInventory, &mut IronFists)>) {
    for (health, mut inventory, mut fists) in &mut players {
        if health.state() == LifeState::HumanTerminal {
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
            attacker: owner,
            target,
            source: ImpactSource::Weapon(weapon.clone()),
            category: DamageCategory::Melee,
            region: HitRegion::Torso,
            amount,
        });
    }
}
