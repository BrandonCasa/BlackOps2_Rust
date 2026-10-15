use crate::{
    GameState,
    access::ZoneAccess,
    assets::GameDefinition,
    gameplay::{GameStep, Session},
    origins::{ContentId, NativeNegotiation, NativePathNode, Traversal, ZoneId},
    player::Player,
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet, VecDeque},
};
#[derive(Clone)]
struct Node {
    point: Vec3,
    zone: ZoneId,
    native: Option<NativePathNode>,
}
impl Node {
    fn disconnectable(&self, origin: Vec3, radius: f32) -> bool {
        self.native.is_some_and(|native| {
            let extent = Vec3::splat(radius + 266. * 0.0254);
            native.spawn_flags & 0x4100 == 0
                && (matches!(native.r#type, 17 | 18)
                    || (self.point.cmpge(origin - extent).all()
                        && self.point.cmple(origin + extent).all()))
        })
    }
}
#[derive(Clone)]
struct Edge {
    to: usize,
    traversal: Traversal,
    animation: Option<String>,
    blockers: Vec<ContentId>,
    cost: f32,
    native: Option<NativeNegotiation>,
}
pub struct TraversalStep {
    pub start: Vec3,
    pub end: Vec3,
    pub alias: String,
    pub yaw: f32,
}
#[derive(Clone)]
pub struct Admission {
    pub position: Vec3,
    pub zone: ZoneId,
    pub source: String,
    pub rising: bool,
    pub yaw: f32,
    pub desired_origin: Option<Vec3>,
    pub activation: ZoneId,
}
#[derive(Resource, Default)]
pub struct Navigation {
    nodes: Vec<Node>,
    edges: Vec<Vec<Edge>>,
    spawns: Vec<Admission>,
    entrances: Vec<crate::access::SpawnEntranceDefinition>,
    entrance_nodes: HashMap<ContentId, usize>,
    pub admissions: Vec<Admission>,
    cache: HashMap<(usize, usize), (Vec3, bool)>,
    order: VecDeque<(usize, usize)>,
    pub revision: u32,
    epoch: u32,
    chamber_blocked: std::collections::HashSet<(usize, usize)>,
    perk_blocked: HashMap<Entity, HashSet<(usize, usize)>>,
}
#[derive(Clone, Copy)]
struct Work {
    cost: f32,
    node: usize,
}
impl PartialEq for Work {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.node == other.node
    }
}
impl Eq for Work {}
impl PartialOrd for Work {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Work {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .total_cmp(&self.cost)
            .then_with(|| other.node.cmp(&self.node))
    }
}
impl Navigation {
    fn invalidate(&mut self) {
        self.cache.clear();
        self.order.clear();
        self.revision = self.revision.wrapping_add(1);
    }

    fn replace_perk_paths(&mut self, owner: Entity, blocked: HashSet<(usize, usize)>) {
        if self.perk_blocked.get(&owner) == Some(&blocked) {
            return;
        }
        self.perk_blocked.insert(owner, blocked);
        self.invalidate();
    }

    pub(crate) fn disconnect_perk_paths(
        &mut self,
        owner: Entity,
        origin: Vec3,
        radius: f32,
        spatial: &SpatialQuery,
        clips: &HashSet<Entity>,
    ) -> bool {
        if self.nodes.is_empty()
            || clips.is_empty()
            || self.nodes.iter().any(|node| node.native.is_none())
        {
            return false;
        }
        let shape = Collider::cuboid(30. * 0.0254, 30. * 0.0254, 30. * 0.0254);
        let filter = SpatialQueryFilter::from_mask(crate::environment::MONSTER_CLIP_LAYER);
        let mut blocked = HashSet::new();
        for (from, node) in self.nodes.iter().enumerate() {
            if !node.disconnectable(origin, radius) {
                continue;
            }
            let start = node.point + Vec3::Y * (33. * 0.0254);
            for edge in &self.edges[from] {
                let delta = self.nodes[edge.to].point - node.point;
                let hit = if let Ok(direction) = Dir3::new(delta) {
                    spatial
                        .cast_shape_predicate(
                            &shape,
                            start,
                            Quat::IDENTITY,
                            direction,
                            &ShapeCastConfig::from_max_distance(delta.length()),
                            &filter,
                            &|entity| clips.contains(&entity),
                        )
                        .is_some()
                } else {
                    spatial
                        .shape_intersections(&shape, start, Quat::IDENTITY, &filter)
                        .iter()
                        .any(|entity| clips.contains(entity))
                };
                if hit {
                    blocked.insert((from, edge.to));
                }
            }
        }
        self.replace_perk_paths(owner, blocked);
        true
    }

    pub(crate) fn connect_perk_paths(&mut self, owner: Entity) {
        if self.perk_blocked.remove(&owner).is_some() {
            self.invalidate();
        }
    }

    pub fn refresh_chamber(
        &mut self,
        spatial: &SpatialQuery,
        walls: &std::collections::HashSet<Entity>,
    ) {
        self.chamber_blocked.clear();
        let center = Vec3::new(262.636, -7.9756, 200.8124);
        for (from, edges) in self.edges.iter().enumerate() {
            let a = self.nodes[from].point + Vec3::Y * 0.8;
            if (a - center).with_y(0.).length_squared() > 45.72_f32.powi(2) {
                continue;
            }
            for edge in edges {
                let b = self.nodes[edge.to].point + Vec3::Y * 0.8;
                let delta = b - a;
                if let Ok(direction) = Dir3::new(delta)
                    && spatial
                        .cast_ray_predicate(
                            a,
                            direction,
                            delta.length(),
                            true,
                            &crate::environment::solids([]),
                            &|entity| walls.contains(&entity),
                        )
                        .is_some()
                {
                    self.chamber_blocked.insert((from, edge.to));
                }
            }
        }
        self.cache.clear();
        self.order.clear();
        self.revision = self.revision.wrapping_add(1);
    }

    fn nearest(&self, p: Vec3, access: &ZoneAccess, active: bool) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| !active || access.reachable.contains(&n.zone))
            .min_by(|(_, a), (_, b)| {
                a.point
                    .distance_squared(p)
                    .total_cmp(&b.point.distance_squared(p))
            })
            .map(|(i, _)| i)
    }
    fn allowed(&self, from: usize, e: &Edge, access: &ZoneAccess) -> bool {
        !self.chamber_blocked.contains(&(from, e.to))
            && !self
                .perk_blocked
                .values()
                .any(|paths| paths.contains(&(from, e.to)))
            && e.native.as_ref().is_none_or(|n| n.disconnect_count == 0)
            && (matches!(e.traversal, Traversal::Walk) || e.animation.is_some())
            && access.reachable.contains(&self.nodes[from].zone)
            && access.reachable.contains(&self.nodes[e.to].zone)
            && e.blockers.iter().all(|id| access.opened(id))
    }
    pub fn direct_allowed(&self, a: Vec3, b: Vec3, access: &ZoneAccess) -> bool {
        self.nearest(a, access, false)
            .zip(self.nearest(b, access, false))
            .is_some_and(|(a, b)| {
                access.reachable.contains(&self.nodes[a].zone)
                    && access.reachable.contains(&self.nodes[b].zone)
            })
    }
    pub fn project_emergence(&self, source: Vec3, access: &ZoneAccess) -> Option<Vec3> {
        let node = self.nearest(source, access, true)?;
        let p = self.nodes[node].point;
        (p.distance_squared(source) <= 100.).then_some(p)
    }
    pub fn destination(&mut self, position: Vec3, target: Vec3, access: &ZoneAccess) -> Vec3 {
        let Some((start, goal)) = self
            .nearest(position, access, true)
            .zip(self.nearest(target, access, false))
        else {
            return position;
        };
        self.route(start, goal, target, access).0
    }
    pub(crate) fn entrance_destination(
        &mut self,
        position: Vec3,
        entrance: &crate::access::SpawnEntranceDefinition,
        access: &ZoneAccess,
    ) -> Vec3 {
        let Some((start, goal)) = self
            .nearest(position, access, true)
            .zip(self.entrance_nodes.get(&entrance.node).copied())
        else {
            return position;
        };
        let (point, fallback) = self.route(start, goal, self.nodes[goal].point, access);
        if fallback { position } else { point }
    }
    pub(crate) fn entrance_traversal(
        &self,
        position: Vec3,
        entrance: &crate::access::SpawnEntranceDefinition,
        access: &ZoneAccess,
    ) -> Option<TraversalStep> {
        let from = *self.entrance_nodes.get(&entrance.node)?;
        if position.distance_squared(Vec3::from_array(entrance.position))
            > (128. * 0.0254_f32).powi(2)
        {
            return None;
        }
        self.edges[from].iter().find_map(|edge| {
            if matches!(edge.traversal, Traversal::Walk) || !self.allowed(from, edge, access) {
                return None;
            }
            Some(TraversalStep {
                start: self.nodes[from].point,
                end: self.nodes[edge.to].point,
                alias: edge.animation.clone()?,
                yaw: edge.native.as_ref()?.start.yaw,
            })
        })
    }
    fn route(
        &mut self,
        start: usize,
        goal: usize,
        target: Vec3,
        access: &ZoneAccess,
    ) -> (Vec3, bool) {
        if start == goal {
            return if access.reachable.contains(&self.nodes[goal].zone) {
                (target, false)
            } else {
                (self.nodes[start].point, true)
            };
        }
        if let Some((point, fallback)) = self.cache.get(&(start, goal)).copied() {
            return (point, fallback);
        }
        let mut costs = vec![f32::INFINITY; self.nodes.len()];
        let mut previous = vec![usize::MAX; self.nodes.len()];
        let mut open = BinaryHeap::new();
        let mut best = start;
        costs[start] = 0.;
        open.push(Work {
            cost: 0.,
            node: start,
        });
        while let Some(Work { cost, node }) = open.pop() {
            if cost > costs[node] {
                continue;
            }
            if self.nodes[node].point.distance_squared(target)
                < self.nodes[best].point.distance_squared(target)
            {
                best = node;
            }
            if node == goal {
                best = node;
                break;
            }
            for edge in &self.edges[node] {
                if !self.allowed(node, edge, access) {
                    continue;
                }
                let next = cost + edge.cost;
                if next < costs[edge.to] {
                    costs[edge.to] = next;
                    previous[edge.to] = node;
                    open.push(Work {
                        cost: next,
                        node: edge.to,
                    });
                }
            }
        }
        let fallback = best != goal;
        let mut node = best;
        while previous[node] != start && previous[node] != usize::MAX {
            node = previous[node];
        }
        let point = self.nodes[node].point;
        if self.cache.len() >= 512 {
            if let Some(old) = self.order.pop_front() {
                self.cache.remove(&old);
            }
        }
        self.cache.insert((start, goal), (point, fallback));
        self.order.push_back((start, goal));
        (point, fallback)
    }
    pub fn traversal(
        &self,
        position: Vec3,
        destination: Vec3,
        access: &ZoneAccess,
    ) -> Option<TraversalStep> {
        let from = self.nearest(position, access, true)?;
        if self.nodes[from].point.distance_squared(position) > 0.64 {
            return None;
        }
        self.edges[from].iter().find_map(|edge| {
            (!matches!(edge.traversal, Traversal::Walk)
                && self.allowed(from, edge, access)
                && self.nodes[edge.to].point.distance_squared(destination) < 0.25)
                .then(|| {
                    edge.animation.clone().map(|alias| {
                        let start = self.nodes[from].point;
                        let end = self.nodes[edge.to].point;
                        let delta = end - start;
                        TraversalStep {
                            start,
                            end,
                            alias,
                            yaw: edge
                                .native
                                .as_ref()
                                .map_or_else(|| (-delta.z).atan2(delta.x), |n| n.start.yaw),
                        }
                    })
                })
                .flatten()
        })
    }
    pub fn alternate(&self, position: Vec3, target: Vec3, old: Vec3, access: &ZoneAccess) -> Vec3 {
        self.nearest(position, access, true)
            .and_then(|start| {
                self.edges[start]
                    .iter()
                    .filter(|e| {
                        self.allowed(start, e, access)
                            && self.nodes[e.to].point.distance_squared(old) > 0.25
                    })
                    .min_by(|a, b| {
                        self.nodes[a.to]
                            .point
                            .distance_squared(target)
                            .total_cmp(&self.nodes[b.to].point.distance_squared(target))
                    })
                    .map(|e| self.nodes[e.to].point)
            })
            .unwrap_or(position)
    }
    pub fn choose_spawn(&self, session: &mut Session) -> Option<&Admission> {
        self.admissions
            .get(session.spawn_index(self.admissions.len())?)
    }
    pub(crate) fn choose_entrance(
        &self,
        spawn: &Admission,
        spawner: &crate::access::ActorSpawnerDefinition,
        session: &mut Session,
    ) -> Option<crate::access::SpawnEntranceDefinition> {
        let desired = spawn
            .desired_origin
            .or(spawner.desired_origin.map(Vec3::from_array));
        let origin = desired.unwrap_or(spawn.position);
        let mut goals = self.entrances.iter().collect::<Vec<_>>();
        goals.sort_by(|a, b| {
            Vec3::from_array(a.position)
                .distance_squared(origin)
                .total_cmp(&Vec3::from_array(b.position).distance_squared(origin))
        });
        if desired.is_some() {
            return goals.first().map(|goal| (*goal).clone());
        }
        goals.truncate(3);
        let mut previous = Vec3::from_array(goals.first()?.position).distance(spawn.position);
        let mut count = 1;
        for goal in goals.iter().skip(1) {
            let current = Vec3::from_array(goal.position).distance(spawn.position);
            if current - previous > 500. * 0.0254 {
                break;
            }
            previous = current;
            count += 1;
        }
        goals
            .get(session.random() as usize % count)
            .map(|goal| (*goal).clone())
    }
}
pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .add_systems(
                OnEnter(GameState::StartingSession),
                reset.in_set(crate::session::SessionSetup::World),
            )
            .add_systems(
                FixedUpdate,
                refresh
                    .in_set(GameStep::EnemyIntent)
                    .before(crate::enemies::lifecycle)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(
    mut nav: ResMut<Navigation>,
    definition: Res<GameDefinition>,
    players: Query<(), With<Player>>,
) {
    if !players.is_empty() {
        return;
    }
    nav.cache.clear();
    nav.order.clear();
    nav.admissions.clear();
    nav.epoch = u32::MAX;
    nav.revision = 0;
    nav.chamber_blocked.clear();
    nav.perk_blocked.clear();
    if !nav.nodes.is_empty() {
        return;
    }
    nav.nodes = definition
        .content
        .navigation_nodes
        .iter()
        .map(|source| Node {
            point: Vec3::from_array(source.position),
            zone: source.zone.clone(),
            native: source.native,
        })
        .collect();
    let ids: HashMap<_, _> = definition
        .content
        .navigation_nodes
        .iter()
        .enumerate()
        .map(|(index, source)| (&source.id, index))
        .collect();
    nav.edges = vec![Vec::new(); nav.nodes.len()];
    for source in &definition.content.navigation_links {
        let &from = ids.get(&source.from).unwrap_or_else(|| {
            panic!(
                "Navigation link {:?} has missing start {:?}",
                source.id, source.from
            )
        });
        let &to = ids.get(&source.to).unwrap_or_else(|| {
            panic!(
                "Navigation link {:?} has missing end {:?}",
                source.id, source.to
            )
        });
        let mut blockers = source.blockers.clone();
        blockers.extend(source.connection.iter().cloned());
        let cost = source.cost.unwrap_or_else(|| {
            source.negotiation.as_ref().map_or_else(
                || {
                    nav.nodes[from]
                        .point
                        .distance(nav.nodes[to].point)
                        .max(0.01)
                },
                |native| native.cost,
            )
        });
        nav.edges[from].push(Edge {
            to,
            traversal: source.traversal,
            animation: source.source_animation.clone(),
            blockers,
            cost,
            native: source.negotiation.clone(),
        });
    }
    nav.spawns = definition
        .access
        .spawns
        .iter()
        .map(|spawn| Admission {
            position: Vec3::from_array(spawn.position),
            zone: spawn.zone.clone(),
            source: spawn.id.clone(),
            rising: spawn.rising,
            yaw: spawn.yaw,
            desired_origin: spawn.desired_origin.map(Vec3::from_array),
            activation: spawn.zone.clone(),
        })
        .collect();
    nav.entrances = definition.access.entrances.clone();
    nav.entrance_nodes = nav
        .entrances
        .iter()
        .map(|entrance| {
            let &index = ids.get(&entrance.node).unwrap_or_else(|| {
                panic!(
                    "Navigation entrance references missing node {:?}",
                    entrance.node
                )
            });
            (entrance.node.clone(), index)
        })
        .collect();
    info!(
        "Native navigation: {} authored nodes, {} directed links, {} spawns",
        nav.nodes.len(),
        nav.edges.iter().map(Vec::len).sum::<usize>(),
        nav.spawns.len()
    );
}
fn refresh(mut nav: ResMut<Navigation>, access: Res<ZoneAccess>) {
    if nav.epoch == access.purchases && !access.is_changed() {
        return;
    }
    nav.invalidate();
    nav.epoch = access.purchases;
    nav.admissions = nav
        .spawns
        .iter()
        .filter(|spawn| {
            access.active.contains(&spawn.activation)
                && access.reachable.contains(&spawn.activation)
        })
        .cloned()
        .collect();
}
#[cfg(test)]
mod tests {
    use super::*;

    fn node(point: Vec3, kind: u8, flags: i32) -> Node {
        Node {
            point,
            zone: ZoneId("zone.start".into()),
            native: Some(NativePathNode {
                r#type: kind,
                spawn_flags: flags,
            }),
        }
    }

    #[test]
    fn native_perk_disconnect_preserves_node_flags_and_negotiation_scan_exceptions() {
        let origin = Vec3::new(4., -3., 7.);
        let radius = 2.;
        let extent = radius + 266. * 0.0254;
        assert!(node(origin + Vec3::X * extent, 1, 0).disconnectable(origin, radius));
        assert!(!node(origin + Vec3::Y * (extent + 0.001), 1, 0).disconnectable(origin, radius));
        assert!(node(origin + Vec3::Z * 100., 17, 0).disconnectable(origin, radius));
        assert!(node(origin + Vec3::Y * 100., 18, 0).disconnectable(origin, radius));
        assert!(!node(origin, 1, 0x100).disconnectable(origin, radius));
        assert!(!node(origin, 17, 0x4000).disconnectable(origin, radius));
        assert!(node(origin, 1, 0x8000).disconnectable(origin, radius));
    }

    #[test]
    fn native_perk_reconnect_retains_other_owners_and_invalidates_cached_routes() {
        let mut world = World::new();
        let first = world.spawn_empty().id();
        let second = world.spawn_empty().id();
        let mut access = ZoneAccess::default();
        access.reachable.insert(ZoneId("zone.start".into()));
        let mut navigation = Navigation::default();
        navigation.nodes = vec![node(Vec3::ZERO, 1, 0), node(Vec3::X, 1, 0)];
        navigation.edges = vec![
            vec![Edge {
                to: 1,
                traversal: Traversal::Walk,
                animation: None,
                blockers: Vec::new(),
                cost: 2.,
                native: None,
            }],
            Vec::new(),
        ];
        assert_eq!(
            navigation.destination(Vec3::ZERO, Vec3::X, &access),
            Vec3::X
        );
        assert_eq!(navigation.cache.len(), 1);
        navigation.replace_perk_paths(first, HashSet::from([(0, 1)]));
        assert_eq!(navigation.cache.len(), 0);
        assert_eq!(
            navigation.destination(Vec3::ZERO, Vec3::X, &access),
            Vec3::ZERO
        );
        navigation.replace_perk_paths(second, HashSet::from([(0, 1)]));
        navigation.connect_perk_paths(first);
        assert!(!navigation.allowed(0, &navigation.edges[0][0], &access));
        navigation.connect_perk_paths(second);
        assert!(navigation.allowed(0, &navigation.edges[0][0], &access));
        assert_eq!(
            navigation.destination(Vec3::ZERO, Vec3::X, &access),
            Vec3::X
        );
    }

    #[test]
    fn exterior_entrance_routes_to_authored_begin_without_reverse_or_closed_paths() {
        let mut access = ZoneAccess::default();
        access.reachable.insert(ZoneId("zone.start".into()));
        let entrance = crate::access::SpawnEntranceDefinition {
            node: ContentId("node.entrance".into()),
            position: Vec3::X.to_array(),
        };
        let mut navigation = Navigation::default();
        navigation.nodes = vec![
            node(Vec3::ZERO, 1, 0),
            node(Vec3::X, 1, 0),
            node(Vec3::X * 2., 17, 0),
        ];
        navigation.entrance_nodes.insert(entrance.node.clone(), 2);
        navigation.edges = vec![
            vec![Edge {
                to: 2,
                traversal: Traversal::Walk,
                animation: None,
                blockers: Vec::new(),
                cost: 2.,
                native: None,
            }],
            Vec::new(),
            Vec::new(),
        ];
        assert_eq!(
            navigation.entrance_destination(Vec3::ZERO, &entrance, &access),
            Vec3::X * 2.
        );
        navigation.edges.swap(0, 2);
        navigation.edges[2][0].to = 0;
        navigation.invalidate();
        assert_eq!(
            navigation.entrance_destination(Vec3::ZERO, &entrance, &access),
            Vec3::ZERO
        );
        navigation.edges.swap(0, 2);
        navigation.edges[0][0].to = 2;
        let gate = ContentId("connection.closed".into());
        navigation.edges[0][0].blockers.push(gate.clone());
        navigation.invalidate();
        assert_eq!(
            navigation.entrance_destination(Vec3::ZERO, &entrance, &access),
            Vec3::ZERO
        );
        access.open.insert(gate);
        navigation.invalidate();
        assert_eq!(
            navigation.entrance_destination(Vec3::ZERO, &entrance, &access),
            Vec3::X * 2.
        );
    }
}
