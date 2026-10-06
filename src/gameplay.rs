use crate::{
    GameState,
    assets::GameDefinition,
    life::{LifeState, ResolvedDamage, Vitality},
    player::{Player, TickActions, capsule},
    weapons::{WeaponEmission, WeaponIntent, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::prelude::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameStep {
    Movement,
    Interactions,
    Combat,
    Zombies,
    Rounds,
}
#[derive(Component)]
pub struct SessionEntity;
#[derive(Component)]
#[require(RigidBody::Kinematic, CustomPositionIntegration)]
pub struct Zombie {
    velocity: Vec3,
    attack: f32,
    navigation: f32,
    destination: Vec3,
}
impl Zombie {
    pub fn new(position: Vec3) -> Self {
        Self {
            velocity: Vec3::ZERO,
            attack: 0.,
            navigation: 0.,
            destination: position,
        }
    }
}

#[derive(Resource, Clone)]
pub struct Session {
    pub round: u32,
    pub pending: u32,
    pub points: u32,
    pub spawn_timer: Timer,
    pub intermission: Option<Timer>,
    pub spawn_cursor: usize,
    pub seed: u64,
    pub kills: u32,
    pub shots: u32,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            round: 1,
            pending: 6,
            points: 500,
            spawn_timer: Timer::from_seconds(1., TimerMode::Once),
            intermission: None,
            spawn_cursor: 0,
            seed: 0xB02,
            kills: 0,
            shots: 0,
        }
    }
}

impl Session {
    pub fn admitted_spawn(&mut self) -> bool {
        if self.pending == 0 {
            return false;
        }
        self.pending -= 1;
        true
    }
    pub fn round_complete(&self, living: usize) -> bool {
        self.pending == 0 && living == 0
    }
    pub fn spawn_index(&mut self, count: usize) -> Option<usize> {
        if count == 0 {
            return None;
        }
        self.seed = self.seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.spawn_cursor = self.spawn_cursor.wrapping_add(1);
        Some(((self.seed >> 32) as usize) % count)
    }
}

pub fn round_budget(round: u32) -> u32 {
    let round = round.clamp(1, 255);
    let multiplier = (round / 5).max(1) as f64 * if round >= 10 { round as f64 * 0.15 } else { 1. };
    let base = 24 + (3. * multiplier) as u32;
    let early = [0.25, 0.3, 0.5, 0.7, 0.9];
    if round < 6 {
        (base as f64 * early[(round - 1) as usize]) as u32
    } else {
        base
    }
}

#[derive(Resource, Default)]
struct Navigation {
    points: Vec<Vec3>,
    edges: Vec<Vec<usize>>,
    admissions: Vec<Vec3>,
}

impl Navigation {
    fn destination(&self, position: Vec3, target: Vec3) -> Vec3 {
        if self.points.is_empty() {
            return target;
        }
        let nearest = |p: Vec3| {
            self.points
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.distance_squared(p).total_cmp(&b.distance_squared(p)))
                .map(|(i, _)| i)
                .unwrap_or(0)
        };
        let start = nearest(position);
        let goal = nearest(target);
        if start == goal {
            return target;
        }
        let mut cost = vec![f32::INFINITY; self.points.len()];
        let mut previous = vec![usize::MAX; self.points.len()];
        let mut open = vec![start];
        cost[start] = 0.;
        while !open.is_empty() {
            let best = (0..open.len())
                .min_by(|a, b| {
                    (cost[open[*a]] + self.points[open[*a]].distance(self.points[goal])).total_cmp(
                        &(cost[open[*b]] + self.points[open[*b]].distance(self.points[goal])),
                    )
                })
                .unwrap_or(0);
            let node = open.swap_remove(best);
            if node == goal {
                break;
            }
            for &other in &self.edges[node] {
                let next_cost = cost[node] + self.points[node].distance(self.points[other]);
                if next_cost < cost[other] {
                    cost[other] = next_cost;
                    previous[other] = node;
                    if !open.contains(&other) {
                        open.push(other);
                    }
                }
            }
        }
        if previous[goal] == usize::MAX {
            return self.points[start];
        }
        let mut node = goal;
        while previous[node] != start && previous[node] != usize::MAX {
            node = previous[node];
        }
        self.points[node]
    }
}

pub struct GameplayPlugin;
#[derive(Resource)]
struct ZombieVisuals {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

fn zombie_visuals(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(ZombieVisuals {
        mesh: meshes.add(Capsule3d::new(0.3, 1.1)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.32, 0.4, 0.28),
            perceptual_roughness: 0.9,
            ..default()
        }),
    });
}

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Session>()
            .add_systems(Startup, zombie_visuals)
            .init_resource::<Navigation>()
            .configure_sets(
                FixedUpdate,
                (
                    GameStep::Movement,
                    GameStep::Interactions,
                    GameStep::Combat,
                    GameStep::Zombies,
                    GameStep::Rounds,
                )
                    .chain(),
            )
            .add_systems(OnEnter(GameState::Playing), begin_session)
            .add_systems(
                FixedUpdate,
                (
                    combat.in_set(GameStep::Combat),
                    move_zombies.in_set(GameStep::Zombies),
                    (cleanup_dead, rounds).chain().in_set(GameStep::Rounds),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn begin_session(
    mut session: ResMut<Session>,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
    mut navigation: ResMut<Navigation>,
    query: SpatialQuery,
) {
    if !players.is_empty() {
        return;
    }
    *session = Session::default();
    if !navigation.points.is_empty() {
        return;
    }
    let shape = capsule(0.3, 1.7);
    navigation.points = definition
        .waypoints
        .iter()
        .filter_map(|point| {
            let point = Vec3::from_array(*point);
            let floor = query.cast_ray(
                point + Vec3::Y * 0.1,
                Dir3::NEG_Y,
                3.,
                false,
                &SpatialQueryFilter::default(),
            )?;
            if floor.normal.y < 0.65 {
                return None;
            }
            let feet = point + Vec3::Y * (0.1 - floor.distance + 0.02);
            query
                .shape_intersections(
                    &shape,
                    feet + Vec3::Y * 0.85,
                    Quat::IDENTITY,
                    &SpatialQueryFilter::default(),
                )
                .is_empty()
                .then_some(feet)
        })
        .collect();
    navigation.edges = vec![Vec::new(); navigation.points.len()];
    for i in 0..navigation.points.len() {
        let a = navigation.points[i];
        let mut candidates: Vec<_> = navigation
            .points
            .iter()
            .enumerate()
            .filter(|(j, b)| i != *j && (a.y - b.y).abs() < 1.2 && a.distance_squared(**b) < 16.)
            .map(|(j, b)| (j, *b))
            .collect();
        candidates.sort_by(|(_, a), (_, b)| {
            a.distance_squared(navigation.points[i])
                .total_cmp(&b.distance_squared(navigation.points[i]))
        });
        for (j, b) in candidates.into_iter().take(8) {
            let delta = b - a;
            if let Ok(direction) = Dir3::new(delta)
                && query
                    .cast_shape(
                        &shape,
                        a + Vec3::Y * 0.85,
                        Quat::IDENTITY,
                        direction,
                        &ShapeCastConfig {
                            ignore_origin_penetration: true,
                            ..ShapeCastConfig::from_max_distance(delta.length())
                        },
                        &SpatialQueryFilter::default(),
                    )
                    .is_none()
            {
                navigation.edges[i].push(j);
            }
        }
    }
    // Admit enemies on the reachable side of map entries until barrier/climb
    // behavior exists. Never spawn an enemy into an inaccessible closed zone.
    let spawn = Vec3::from_array(definition.spawn);
    if let Some(start) = navigation
        .points
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            a.distance_squared(spawn)
                .total_cmp(&b.distance_squared(spawn))
        })
        .map(|(i, _)| i)
    {
        let mut reachable = vec![false; navigation.points.len()];
        reachable[start] = true;
        let mut work = vec![start];
        while let Some(node) = work.pop() {
            for &other in &navigation.edges[node] {
                if !reachable[other] {
                    reachable[other] = true;
                    work.push(other);
                }
            }
        }
        let eligible: Vec<_> = navigation
            .points
            .iter()
            .enumerate()
            .filter(|(i, p)| reachable[*i] && p.distance_squared(spawn) > 4.)
            .map(|(_, p)| *p)
            .collect();
        for point in &definition.spawn_points {
            let entry = Vec3::from_array(*point);
            if let Some(nearest) = eligible.iter().min_by(|a, b| {
                a.distance_squared(entry)
                    .total_cmp(&b.distance_squared(entry))
            }) && !navigation
                .admissions
                .iter()
                .any(|p| p.distance_squared(*nearest) < 0.5)
            {
                navigation.admissions.push(*nearest);
            }
        }
    }
    info!(
        "Navigation: {} nodes, {} reachable entry projections",
        navigation.points.len(),
        navigation.admissions.len()
    );
}

fn combat(
    time: Res<Time<Fixed>>,
    actions: Res<TickActions>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    mut players: Query<(Entity, &Player, &Transform, &mut WeaponInventory)>,
    mut enemies: Query<(&mut Vitality, &Transform), With<Zombie>>,
    query: SpatialQuery,
) {
    let dt = time.delta_secs();
    let Ok((entity, player, transform, mut inventory)) = players.single_mut() else {
        return;
    };
    let emissions = inventory.tick(
        dt,
        WeaponIntent {
            pressed: actions.fire,
            held: actions.fire_held,
            reload: actions.reload,
            melee: actions.melee,
            cycle: actions.cycle_weapon,
            slot: actions.weapon_slot,
            ads: actions.ads,
        },
        &definition.content.weapons,
    );
    let eye = if player.crouched {
        definition.tuning.crouch_height * 0.9
    } else {
        definition.tuning.eye_height
    };
    let origin = transform.translation + Vec3::Y * (eye - player.height * 0.5);
    let rotation = Quat::from_rotation_y(player.yaw + inventory.recoil.x)
        * Quat::from_rotation_x((player.pitch + inventory.recoil.y).clamp(-1.5, 1.5));
    let direction = Dir3::new(rotation * Vec3::NEG_Z).unwrap_or(Dir3::NEG_Z);
    let filter = SpatialQueryFilter::from_excluded_entities([entity]);
    for emission in emissions {
        let weapon = inventory.definition(&definition.content.weapons);
        let pellets = if matches!(emission, WeaponEmission::Melee) {
            1
        } else {
            weapon.map_or(1, |w| w.mechanics.pellets)
        };
        let (melee, damage) = match emission {
            WeaponEmission::Shot(id) => {
                session.shots += 1;
                let Some(weapon) = definition.content.weapons.iter().find(|w| w.id == id) else {
                    continue;
                };
                (false, weapon.damage)
            }
            WeaponEmission::Melee => (
                true,
                definition
                    .content
                    .weapons
                    .iter()
                    .find(|w| w.id.0 == "weapon.knife_zm")
                    .map_or(definition.tuning.melee_damage, |w| w.damage),
            ),
        };
        let spread = weapon.map_or(0., |w| {
            let hip = w.mechanics.spread[if player.crouched { 1 } else { 0 }]
                * (1. + player.velocity.xz().length().min(6.) / 6.);
            hip + (w.mechanics.spread[3] - hip) * inventory.ads
        });
        if !melee && let Some(weapon) = weapon {
            inventory.recoil_impulse(weapon, session.shots.wrapping_mul(31) as u64);
        }
        for pellet in 0..pellets {
            let direction = if melee {
                direction
            } else {
                crate::weapons::spread_direction(
                    rotation,
                    spread,
                    session.shots as u64 * 137 + pellet as u64 * 19,
                )
            };
            if let Some(hit) = query.cast_ray(
                origin,
                direction,
                if melee { 2.2 } else { 100. },
                true,
                &filter,
            ) && let Ok((mut vitality, target)) = enemies.get_mut(hit.entity)
                && vitality.state() == LifeState::ActorAlive
            {
                // Capsule regions are an explicit approximation until animated hit volumes are bound.
                let local_height =
                    (origin + direction.as_vec3() * hit.distance).y - target.translation.y;
                let region = if local_height > 0.55 {
                    crate::weapons::HitRegion::Head
                } else if local_height > 0.1 {
                    crate::weapons::HitRegion::UpperTorso
                } else if local_height < -0.35 {
                    crate::weapons::HitRegion::Leg
                } else {
                    crate::weapons::HitRegion::Torso
                };
                let damage = if melee {
                    damage
                } else {
                    inventory
                        .definition(&definition.content.weapons)
                        .map_or(damage, |weapon| {
                            crate::weapons::damage_at(weapon, hit.distance, region)
                        })
                };
                let Ok(damage) = ResolvedDamage::new(damage) else {
                    continue;
                };
                if vitality.apply(damage).is_ok() {
                    session.points = session.points.saturating_add(10);
                }
                if vitality.state() == LifeState::ActorDead {
                    session.points = session.points.saturating_add(if melee { 130 } else { 60 });
                }
            }
        }
    }
}

fn move_zombies(
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    navigation: Res<Navigation>,
    mut players: Query<(Entity, &Transform, &mut Vitality), (With<Player>, Without<Zombie>)>,
    mut enemies: Query<
        (Entity, &mut Zombie, &mut Transform, &Collider, &Vitality),
        Without<Player>,
    >,
    slide: MoveAndSlide,
    query: SpatialQuery,
    mut next: ResMut<NextState<GameState>>,
) {
    let Ok((player_entity, player_transform, mut health)) = players.single_mut() else {
        return;
    };
    for (entity, mut zombie, mut transform, shape, vitality) in &mut enemies {
        if vitality.state() != LifeState::ActorAlive {
            continue;
        }
        let dt = time.delta_secs();
        zombie.attack = (zombie.attack - dt).max(0.);
        zombie.navigation -= dt;
        let target = player_transform.translation;
        let feet = transform.translation - Vec3::Y * 0.85;
        if zombie.navigation <= 0. {
            zombie.navigation = 0.5;
            let delta = target - transform.translation;
            let filter = SpatialQueryFilter::from_excluded_entities([entity, player_entity]);
            let direct = Dir3::new(delta).ok().is_some_and(|dir| {
                query
                    .cast_ray(transform.translation, dir, delta.length(), false, &filter)
                    .is_none()
            });
            zombie.destination = if direct {
                target
            } else {
                navigation.destination(feet, target - Vec3::Y * 0.9) + Vec3::Y * 0.85
            };
        }
        let delta = zombie.destination - transform.translation;
        let direction = Vec3::new(delta.x, 0., delta.z).normalize_or_zero();
        zombie.velocity.x = direction.x * definition.tuning.zombie_speed;
        zombie.velocity.z = direction.z * definition.tuning.zombie_speed;
        zombie.velocity.y = (zombie.velocity.y - definition.tuning.gravity * dt).max(-30.);
        let filter = SpatialQueryFilter::from_excluded_entities([entity]);
        let support = query.cast_ray(feet + Vec3::Y * 0.1, Dir3::NEG_Y, 0.25, false, &filter);
        let grounded = support.is_some_and(|hit| hit.normal.y > 0.65) && zombie.velocity.y <= 0.;
        let result = crate::player::slide_character(
            &slide,
            &query,
            crate::player::MotionShape {
                collider: shape,
                radius: 0.3,
                step_height: definition.tuning.step_height,
                grounded,
            },
            transform.translation,
            zombie.velocity,
            time.delta(),
            &filter,
        );
        transform.translation = result.position;
        zombie.velocity.y = result.projected_velocity.y;
        if transform.translation.distance(target) < 1.35 && zombie.attack == 0. {
            zombie.attack = 1.2;
            if let Ok(damage) = ResolvedDamage::new(definition.tuning.zombie_damage) {
                let _ = health.apply(damage);
            }
            if health.state() == LifeState::HumanTerminal {
                next.set(GameState::GameOver);
            }
        }
    }
}

fn cleanup_dead(
    mut commands: Commands,
    enemies: Query<(Entity, &Vitality), With<Zombie>>,
    mut session: ResMut<Session>,
) {
    for (entity, health) in &enemies {
        if health.state() == LifeState::ActorDead {
            commands.entity(entity).despawn();
            session.kills += 1;
        }
    }
}

fn rounds(
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut session: ResMut<Session>,
    enemies: Query<&Vitality, With<Zombie>>,
    mut commands: Commands,
    query: SpatialQuery,
    visuals: Res<ZombieVisuals>,
    navigation: Res<Navigation>,
) {
    let living = enemies
        .iter()
        .filter(|v| v.state() == LifeState::ActorAlive)
        .count();
    if session.round_complete(living) {
        let timer = session
            .intermission
            .get_or_insert_with(|| Timer::from_seconds(5., TimerMode::Once));
        timer.tick(time.delta());
        if timer.is_finished() {
            session.round = (session.round + 1).min(255);
            session.pending = round_budget(session.round);
            session.intermission = None;
            session.spawn_timer = Timer::from_seconds(1., TimerMode::Once);
            info!("Round {} begins, budget {}", session.round, session.pending);
        }
        return;
    }
    session.spawn_timer.tick(time.delta());
    if session.pending == 0 || living >= 24 || !session.spawn_timer.is_finished() {
        return;
    }
    session.spawn_timer = Timer::from_seconds(1.2, TimerMode::Once);
    let count = if navigation.admissions.is_empty() {
        definition.spawn_points.len()
    } else {
        navigation.admissions.len()
    };
    let Some(index) = session.spawn_index(count) else {
        return;
    };
    let base = navigation
        .admissions
        .get(index)
        .copied()
        .unwrap_or_else(|| Vec3::from_array(definition.spawn_points[index]));
    let Some(floor) = query.cast_ray(
        base + Vec3::Y * 0.1,
        Dir3::NEG_Y,
        4.,
        false,
        &SpatialQueryFilter::default(),
    ) else {
        return;
    };
    if floor.normal.y < 0.5 {
        return;
    }
    let position = base + Vec3::Y * (0.1 - floor.distance + 0.87);
    let shape = capsule(0.3, 1.7);
    let blocked = query.shape_intersections(
        &shape,
        position,
        Quat::IDENTITY,
        &SpatialQueryFilter::default(),
    );
    if !blocked.is_empty() {
        return;
    }
    let health_amount = 150 + (session.round - 1).min(100) * 100;
    let Ok(health) = Vitality::ordinary_actor(health_amount, health_amount) else {
        return;
    };
    if !session.admitted_spawn() {
        return;
    }
    commands.spawn((
        Zombie::new(position),
        shape,
        health,
        SessionEntity,
        Mesh3d(visuals.mesh.clone()),
        MeshMaterial3d(visuals.material.clone()),
        Transform::from_translation(position),
        Name::new("Temporary zombie"),
    ));
}
