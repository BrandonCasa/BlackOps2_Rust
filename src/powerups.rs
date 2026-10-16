use crate::{
    GameState,
    assets::GameDefinition,
    damage::{DamageQueue, EnemyDeath, Impact, ImpactSource},
    gameplay::{GameStep, Session, SessionEntity, Zombie},
    life::{LifeState, Vitality},
    origins::content::{DamageCategory, PowerupDefinition},
    player::Player,
    weapons::{HitRegion, WeaponInventory},
};
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::{prelude::*, world_serialization::WorldAssetRoot};
use std::collections::{HashSet, VecDeque};

#[derive(Resource, Default)]
pub struct PowerupEffects {
    pub double_points: f32,
    pub insta_kill: f32,
    pub zombie_blood: f32,
    pub fire_sale: f32,
}
impl PowerupEffects {
    pub fn multiplier(&self) -> u32 {
        if self.double_points > 0. { 2 } else { 1 }
    }
}
#[derive(Message)]
pub(crate) struct DronePickup {
    pub owner: Entity,
    pub pickup: Entity,
}
#[derive(Message)]
pub(crate) struct PowerupCollected(pub crate::origins::ContentId);
#[derive(Component)]
pub struct Pickup {
    pub definition: usize,
    pub remaining: f32,
}
#[derive(Resource)]
pub struct Drops {
    #[cfg(feature = "dev-tools")]
    pub(crate) selection: Option<usize>,
    pub spawned: u32,
    pub collected: u32,
    pub expired: u32,
    round: u32,
    pub count: u32,
    threshold: f64,
    increment: f64,
    earned: bool,
    cycle: Vec<usize>,
}
impl Default for Drops {
    fn default() -> Self {
        Self {
            #[cfg(feature = "dev-tools")]
            selection: None,
            spawned: 0,
            collected: 0,
            expired: 0,
            round: 0,
            count: 0,
            threshold: 2500.,
            increment: 2000.,
            earned: false,
            cycle: Vec::new(),
        }
    }
}
impl Drops {
    fn next_powerup(
        &mut self,
        game: &GameDefinition,
        session: &mut Session,
        fire_sale_allowed: bool,
    ) -> Option<usize> {
        // get_valid_powerup advances past an unavailable Fire Sale, including
        // crossing the end of a shuffled cycle without losing the earned drop.
        for _ in 0..=game.content.powerups.len() {
            if self.cycle.is_empty() {
                self.cycle = game
                    .content
                    .powerups
                    .iter()
                    .enumerate()
                    .filter(|(_, powerup)| supported(powerup) && powerup.presentation.is_some())
                    .map(|(index, _)| index)
                    .collect();
                crate::economy::source_shuffle(&mut self.cycle, session);
            }
            #[cfg(feature = "dev-tools")]
            if let Some(index) = self.selection.take()
                && let Some(position) = self.cycle.iter().position(|&i| i == index)
            {
                let last = self.cycle.len() - 1;
                self.cycle.swap(position, last);
            }
            let index = self.cycle.pop()?;
            if game.content.powerups[index].id.0 != "powerup.fire_sale" || fire_sale_allowed {
                return Some(index);
            }
        }
        None
    }
}
#[cfg(feature = "dev-tools")]
impl Drops {
    pub(crate) fn select(&mut self, id: &str, game: &GameDefinition) -> Result<(), String> {
        let index = game
            .content
            .powerups
            .iter()
            .position(|p| {
                p.id.0 == id
                    && supported(p)
                    && p.presentation.as_ref().is_some_and(|id| {
                        game.content
                            .presentations
                            .iter()
                            .any(|p| p.id == *id && p.scene.is_some())
                    })
            })
            .ok_or("Powerup has no supported native pickup")?;
        if self.selection.is_some()
            || self.count >= 4
            || !self.cycle.is_empty() && !self.cycle.contains(&index)
        {
            return Err(
                "Powerup budget or cycle rejects this choice, or a selection is already pending"
                    .into(),
            );
        }
        self.selection = Some(index);
        Ok(())
    }
}

#[cfg(all(test, feature = "dev-tools"))]
#[test]
fn opportunity_powerup_rejections_preserve_budget_cycle_and_threshold() {
    let game: GameDefinition = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("gameplay/origins/game.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut drops = Drops::default();
    let blood = game
        .content
        .powerups
        .iter()
        .position(|p| p.id.0 == "powerup.zombie_blood")
        .unwrap();
    drops.cycle = vec![blood];
    assert!(drops.select("powerup.full_ammo", &game).is_err());
    drops.count = 4;
    assert!(drops.select("powerup.zombie_blood", &game).is_err());
    assert_eq!(
        (drops.threshold, drops.increment, drops.earned, drops.count),
        (2500., 2000., false, 4)
    );
    assert_eq!(drops.cycle, vec![blood]);
    assert_eq!(drops.selection, None);
    drops.count = 0;
    drops.select("powerup.zombie_blood", &game).unwrap();
    assert!(drops.select("powerup.zombie_blood", &game).is_err());
    assert_eq!(drops.selection, Some(blood));
    assert_eq!(drops.cycle, vec![blood]);
    assert_eq!(drops.spawned, 0);
}
#[cfg(test)]
#[test]
fn native_powerup_cycle_skips_unavailable_sales_without_losing_a_drop() {
    let game: GameDefinition =
        serde_json::from_str(include_str!("../gameplay/origins/game.json")).unwrap();
    let sale = game
        .content
        .powerups
        .iter()
        .position(|p| p.id.0 == "powerup.fire_sale")
        .unwrap();
    let mut session = Session::default();
    let mut drops = Drops::default();
    drops.cycle = vec![sale];
    let selected = drops.next_powerup(&game, &mut session, false).unwrap();
    assert_ne!(game.content.powerups[selected].id.0, "powerup.fire_sale");
    assert_eq!((drops.count, drops.spawned), (0, 0));
    drops.cycle = vec![sale];
    assert_eq!(drops.next_powerup(&game, &mut session, true), Some(sale));
}

#[derive(Component)]
struct Nuke {
    owner: Entity,
    origin: Vec3,
    remaining: f32,
    targets: Option<VecDeque<Entity>>,
}
#[derive(Component)]
pub(crate) struct NukeTarget;
pub struct PowerupPlugin;
impl Plugin for PowerupPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DronePickup>()
            .add_message::<PowerupCollected>()
            .init_resource::<PowerupEffects>()
            .init_resource::<Drops>()
            .add_systems(
                FixedUpdate,
                (pickup_fx, tick, collect, effect_audio)
                    .chain()
                    .in_set(GameStep::Interactions)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                detonate
                    .in_set(GameStep::Combat)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                drop_powerups
                    .in_set(GameStep::Zombies)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut effects: ResMut<PowerupEffects>,
    mut drops: ResMut<Drops>,
    mut pickups: Query<(Entity, &mut Pickup, &mut Transform, &mut Visibility)>,
    player: Query<&Vitality, With<Player>>,
) {
    let dt = time.delta_secs();
    effects.double_points = (effects.double_points - dt).max(0.);
    effects.insta_kill = (effects.insta_kill - dt).max(0.);
    effects.fire_sale = (effects.fire_sale - dt).max(0.);
    effects.zombie_blood = if player.iter().any(|v| v.state() == LifeState::Standing) {
        (effects.zombie_blood - dt).max(0.)
    } else {
        0.
    };
    for (entity, mut pickup, mut transform, mut visibility) in &mut pickups {
        pickup.remaining -= dt;
        transform.rotate_y(dt);
        if pickup.remaining <= 0. {
            crate::native_fx::emit(&mut commands, "misc/fx_zombie_powerup_off", *transform);
            commands.entity(entity).despawn();
            drops.expired += 1;
        } else {
            let elapsed = 26.5 - pickup.remaining;
            let phase = if elapsed < 15. {
                0
            } else if elapsed < 22.5 {
                ((elapsed - 15.) / 0.5) as u32
            } else if elapsed < 25. {
                15 + ((elapsed - 22.5) / 0.25) as u32
            } else {
                25 + ((elapsed - 25.) / 0.1) as u32
            };
            *visibility = if phase % 2 == 0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
fn pickup_fx(
    mut commands: Commands,
    game: Res<GameDefinition>,
    pickups: Query<(Entity, &Pickup), Added<Pickup>>,
) {
    for (entity, pickup) in &pickups {
        let name = &game.content.powerups[pickup.definition].id.0;
        let effect = if matches!(
            name.as_str(),
            "powerup.zombie_blood" | "powerup.bonus_points_player"
        ) {
            "misc/fx_zombie_powerup_solo_on"
        } else {
            "misc/fx_zombie_powerup_on"
        };
        crate::native_fx::attach(&mut commands, effect, entity, Transform::default());
    }
}
fn supported(definition: &PowerupDefinition) -> bool {
    matches!(
        definition.id.0.as_str(),
        "powerup.nuke"
            | "powerup.full_ammo"
            | "powerup.insta_kill"
            | "powerup.double_points"
            | "powerup.fire_sale"
            | "powerup.zombie_blood"
    )
}
fn sound(commands: &mut Commands, game: &GameDefinition, alias: &str, position: Vec3) {
    if game.audio.aliases.contains_key(alias) {
        crate::audio::emit(commands, alias, crate::audio::SoundSource::World(position));
    }
}
pub(crate) fn spawn_named(
    commands: &mut Commands,
    game: &GameDefinition,
    server: &AssetServer,
    name: &str,
    position: Vec3,
) -> Option<Entity> {
    let id = if name.starts_with("powerup.") {
        name.to_string()
    } else {
        format!("powerup.{name}")
    };
    let index = game
        .content
        .powerups
        .iter()
        .position(|powerup| powerup.id.0 == id)?;
    let powerup = &game.content.powerups[index];
    let scene = game
        .content
        .presentations
        .iter()
        .find(|p| Some(&p.id) == powerup.presentation.as_ref())?
        .scene
        .as_ref()?;
    let entity = commands
        .spawn((
            SessionEntity,
            Pickup {
                definition: index,
                remaining: 26.5,
            },
            WorldAssetRoot(server.load(scene.clone())),
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .id();
    sound(commands, game, "zmb_spawn_powerup", position);
    if game.audio.aliases.contains_key("zmb_spawn_powerup_loop") {
        crate::audio::emit_loop(
            commands,
            "zmb_spawn_powerup_loop",
            crate::audio::SoundSource::attached(entity),
        );
    }
    Some(entity)
}
fn effect_audio(
    mut commands: Commands,
    definition: Res<GameDefinition>,
    effects: Res<PowerupEffects>,
    players: Query<(Entity, &Transform), With<Player>>,
    entities: Query<()>,
    mut loops: Local<[Option<Entity>; 3]>,
) {
    let Ok((player, pose)) = players.single() else {
        return;
    };
    for (index, (remaining, alias, end)) in [
        (
            effects.double_points,
            "zmb_double_point_loop",
            "zmb_points_loop_off",
        ),
        (effects.insta_kill, "zmb_insta_kill_loop", "zmb_insta_kill"),
        (effects.zombie_blood, "zmb_zombieblood_3rd_loop", ""),
    ]
    .into_iter()
    .enumerate()
    {
        if loops[index].is_some_and(|entity| !entities.contains(entity)) {
            loops[index] = None;
        }
        if remaining > 0. && loops[index].is_none() && definition.audio.aliases.contains_key(alias)
        {
            loops[index] = Some(crate::audio::emit_loop(
                &mut commands,
                alias,
                crate::audio::SoundSource::attached(player),
            ));
        } else if remaining <= 0.
            && let Some(entity) = loops[index].take()
        {
            commands.entity(entity).try_despawn();
            if entities.contains(entity) {
                sound(&mut commands, &definition, end, pose.translation);
            }
        }
    }
}
fn drop_powerups(
    mut commands: Commands,
    mut deaths: MessageReader<EnemyDeath>,
    definition: Res<GameDefinition>,
    mut drops: ResMut<Drops>,
    mut session: ResMut<Session>,
    enemies: Query<(&Transform, Option<&crate::rounds::PopulationKind>), With<Zombie>>,
    panzers: Query<(), With<crate::panzer::Panzer>>,
    pilots: Query<(), With<crate::quest_recovery::Pilot>>,
    players: Query<Entity, With<Player>>,
    spatial: SpatialQuery,
    server: Res<AssetServer>,
    zones: Query<&crate::access::ZoneSensor>,
    stations: Res<crate::stations::Stations>,
    effects: Res<PowerupEffects>,
) {
    if drops.round != session.round {
        drops.round = session.round;
        drops.count = 0;
    }
    if f64::from(session.score) > drops.threshold {
        drops.increment *= 1.14;
        drops.threshold = f64::from(session.score) + drops.increment;
        drops.earned = true;
    }
    for EnemyDeath(receipt) in deaths.read() {
        if enemies.get(receipt.impact.target).is_ok_and(|(_, kind)| {
            kind.is_some_and(|kind| *kind == crate::rounds::PopulationKind::GeneratorDefender)
        }) {
            continue;
        }
        if panzers.contains(receipt.impact.target)
            || pilots.contains(receipt.impact.target)
            || drops.count >= 4
            || !receipt.impact.source.player_attributed()
            || !players.contains(receipt.impact.attacker)
            || definition.content.weapons.iter().any(|w| {
                w.id.0 == receipt.impact.source.id()
                    && w.throwable
                        .as_ref()
                        .is_some_and(|t| t.slot == crate::equipment::ThrowableSlot::Tactical)
            })
        {
            continue;
        }
        if session.random() % 100 > 2 && !drops.earned {
            continue;
        }
        let Ok((transform, _)) = enemies.get(receipt.impact.target) else {
            continue;
        };
        let origin = transform.translation + Vec3::Y * 0.127;
        let Some(hit) = spatial.cast_ray(
            origin,
            Dir3::NEG_Y,
            7.62,
            true,
            &crate::environment::solids([receipt.impact.target, receipt.impact.attacker]),
        ) else {
            continue;
        };
        let position = origin - Vec3::Y * hit.distance + Vec3::Y * 1.016;
        if !spatial
            .point_intersections(
                position,
                &SpatialQueryFilter::from_mask(crate::access::ZONE_SENSOR_LAYER),
            )
            .iter()
            .any(|e| zones.contains(*e))
        {
            continue;
        }
        let Some(index) = drops.next_powerup(
            &definition,
            &mut session,
            stations.moves > 0 && effects.fire_sale <= 0.,
        ) else {
            continue;
        };
        let d = &definition.content.powerups[index];
        let Some(scene) = definition
            .content
            .presentations
            .iter()
            .find(|p| Some(&p.id) == d.presentation.as_ref())
            .and_then(|p| p.scene.as_ref())
        else {
            continue;
        };
        let entity = commands
            .spawn((
                SessionEntity,
                Pickup {
                    definition: index,
                    remaining: 26.5,
                },
                WorldAssetRoot(server.load(scene.clone())),
                Transform::from_translation(position),
                Visibility::Inherited,
            ))
            .id();
        sound(&mut commands, &definition, "zmb_spawn_powerup", position);
        if definition
            .audio
            .aliases
            .contains_key("zmb_spawn_powerup_loop")
        {
            crate::audio::emit_loop(
                &mut commands,
                "zmb_spawn_powerup_loop",
                crate::audio::SoundSource::attached(entity),
            );
        }
        drops.count += 1;
        drops.spawned += 1;
        drops.earned = false;
    }
}
pub(crate) fn collect(
    mut commands: Commands,
    mut remote: MessageReader<DronePickup>,
    definition: Res<GameDefinition>,
    mut effects: ResMut<PowerupEffects>,
    mut drops: ResMut<Drops>,
    mut session: ResMut<Session>,
    pickups: Query<
        (Entity, &Pickup, &Transform),
        Without<crate::environment::digging::RisingPickup>,
    >,
    mut players: Query<(
        Entity,
        &Player,
        &Transform,
        &Vitality,
        &mut WeaponInventory,
        &mut crate::equipment::GrenadeInventory,
        &mut crate::equipment::TacticalInventory,
        &mut crate::mines::MineInventory,
        &mut crate::solo_revive::PerkInventory,
        Option<&crate::perks::PerkDrinking>,
    )>,
) {
    let authorized: HashSet<_> = remote
        .read()
        .map(|event| (event.owner, event.pickup))
        .collect();
    for (entity, pickup, transform) in &pickups {
        if pickup.remaining <= 0. {
            continue;
        }
        let Some(d) = definition.content.powerups.get(pickup.definition) else {
            continue;
        };
        for (
            owner,
            player,
            pose,
            health,
            mut weapons,
            mut grenades,
            mut tactical,
            mut mines,
            mut perks,
            drinking,
        ) in &mut players
        {
            if !matches!(health.state(), LifeState::Standing | LifeState::Downed)
                || (d.id.0 == "powerup.zombie_blood" && health.state() != LifeState::Standing)
                || (!authorized.contains(&(owner, entity))
                    && (pose.translation - Vec3::Y * (player.height * 0.5))
                        .distance_squared(transform.translation)
                        > 1.6256_f32.powi(2))
            {
                continue;
            }
            match d.id.0.as_str() {
                "powerup.random_perk" => {
                    if health.state() != LifeState::Standing {
                        continue;
                    }
                    crate::perks::grant_random_perk(
                        &definition,
                        &mut perks,
                        drinking.map(|drink| &drink.perk),
                        &mut session,
                    );
                }
                "powerup.bonus_points_player" | "powerup.bonus_points_team" => {
                    let _default_points = (1 + session.random() % 24) * 100;
                    let points = (1 + session.random() % 5) * 50;
                    if health.state() == LifeState::Standing {
                        session.award(points * effects.multiplier());
                    }
                }
                "powerup.fire_sale" => effects.fire_sale += d.duration,
                "powerup.double_points" => effects.double_points = d.duration,
                "powerup.insta_kill" => effects.insta_kill = d.duration,
                "powerup.zombie_blood" => effects.zombie_blood = d.duration,
                "powerup.full_ammo" => {
                    if health.state() == LifeState::Standing {
                        weapons.refill(&definition.content.weapons);
                        grenades.count = grenades.capacity;
                        if let Some(tactical) = tactical.0.as_mut() {
                            tactical.count = tactical.capacity;
                        }
                        if let Some(profile) = mines
                            .definition
                            .as_ref()
                            .and_then(|id| definition.content.weapons.iter().find(|w| w.id == *id))
                            .and_then(|w| w.mine.as_ref())
                        {
                            mines.count = profile.stock;
                        }
                    }
                    sound(
                        &mut commands,
                        &definition,
                        "zmb_full_ammo",
                        pose.translation,
                    );
                }
                "powerup.nuke" => {
                    commands.spawn((
                        SessionEntity,
                        Nuke {
                            owner,
                            origin: transform.translation,
                            remaining: 0.5,
                            targets: None,
                        },
                    ));
                    sound(
                        &mut commands,
                        &definition,
                        "evt_nuke_flash",
                        pose.translation,
                    );
                }
                _ => continue,
            }
            for effect in if matches!(
                d.id.0.as_str(),
                "powerup.zombie_blood" | "powerup.bonus_points_player"
            ) {
                [
                    "misc/fx_zombie_powerup_solo_grab",
                    "misc/fx_zombie_powerup_solo_wave",
                ]
            } else {
                ["misc/fx_zombie_powerup_grab", "misc/fx_zombie_powerup_wave"]
            } {
                crate::native_fx::emit(&mut commands, effect, *transform);
            }
            commands.entity(entity).despawn();
            sound(
                &mut commands,
                &definition,
                "zmb_powerup_grabbed",
                transform.translation,
            );
            let voice = match d.id.0.as_str() {
                "powerup.insta_kill" => "powerup_insta",
                "powerup.full_ammo" => "powerup_ammo",
                "powerup.double_points" => "",
                "powerup.zombie_blood" => "powerup_zombie_blood",
                "powerup.fire_sale" => "powerup_firesale",
                "powerup.bonus_points_player"
                | "powerup.bonus_points_team"
                | "powerup.random_perk" => "",
                _ => "powerup_nuke",
            };
            if !voice.is_empty() {
                crate::audio::voice(&mut commands, voice, owner);
            }
            let announcer = match d.id.0.as_str() {
                "powerup.insta_kill" => "instakill",
                "powerup.full_ammo" => "maxammo",
                "powerup.double_points" => "doublepoints",
                "powerup.zombie_blood" => "zombie_blood",
                "powerup.fire_sale" => "firesale",
                "powerup.random_perk" => "perk",
                "powerup.bonus_points_player" | "powerup.bonus_points_team" => "blood_money",
                _ => "nuke",
            };
            sound(
                &mut commands,
                &definition,
                &format!("vox_zmba_powerup_{announcer}_0"),
                pose.translation,
            );
            drops.collected += 1;
            commands.write_message(PowerupCollected(d.id.clone()));
            break;
        }
    }
}
fn detonate(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    mut waves: Query<(Entity, &mut Nuke)>,
    enemies: Query<(Entity, &Transform, &Vitality), With<Zombie>>,
    panzers: Query<(), With<crate::panzer::Panzer>>,
    mut queue: ResMut<DamageQueue>,
    mut session: ResMut<Session>,
    effects: Res<PowerupEffects>,
    definition: Res<GameDefinition>,
) {
    let mut claimed: HashSet<_> = waves
        .iter()
        .filter_map(|(_, wave)| wave.targets.as_ref())
        .flat_map(|targets| targets.iter().copied())
        .collect();
    for (entity, mut wave) in &mut waves {
        wave.remaining -= time.delta_secs();
        if wave.remaining > 0. {
            continue;
        }
        if wave.targets.is_none() {
            for (target, _, health) in &enemies {
                if panzers.contains(target) && health.state() == LifeState::ActorAlive {
                    let delay = 0.1 + (session.random() as f32 / u32::MAX as f32) * 0.6;
                    crate::panzer::queue_nuke(&mut commands, target, wave.owner, delay);
                }
            }
            let mut targets: Vec<_> = enemies
                .iter()
                .filter(|(entity, _, v)| {
                    v.state() == LifeState::ActorAlive
                        && !panzers.contains(*entity)
                        && !claimed.contains(entity)
                })
                .collect();
            targets.sort_by(|(_, a, _), (_, b, _)| {
                a.translation
                    .distance_squared(wave.origin)
                    .total_cmp(&b.translation.distance_squared(wave.origin))
            });
            wave.targets = Some(
                targets
                    .into_iter()
                    .map(|(e, _, _)| {
                        claimed.insert(e);
                        commands.entity(e).insert(NukeTarget);
                        e
                    })
                    .collect(),
            );
            if wave
                .targets
                .as_ref()
                .is_some_and(|targets| !targets.is_empty())
            {
                wave.remaining = 0.1 + (session.random() as f32 / u32::MAX as f32) * 0.6;
                continue;
            }
        }
        if let Some(target) = wave.targets.as_mut().and_then(VecDeque::pop_front) {
            if let Ok((_, pose, health)) = enemies.get(target)
                && health.state() == LifeState::ActorAlive
            {
                queue.0.push(Impact {
                    point: None,
                    attacker: wave.owner,
                    target,
                    source: ImpactSource::Powerup,
                    category: DamageCategory::Explosive,
                    region: HitRegion::Torso,
                    amount: health.current().unwrap_or(0).saturating_add(666),
                });
                sound(&mut commands, &definition, "evt_nuked", pose.translation);
            }
            if wave
                .targets
                .as_ref()
                .is_some_and(|targets| !targets.is_empty())
            {
                wave.remaining = 0.1 + (session.random() as f32 / u32::MAX as f32) * 0.6;
                continue;
            }
        }
        session.award(400 * effects.multiplier());
        commands.entity(entity).despawn();
    }
}
