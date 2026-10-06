//! Compiled map graph with Avian geometry admission and session access requirements.
use crate::{
    GameState,
    access::{DoorBlocker, ZoneAccess},
    assets::GameDefinition,
    gameplay::{GameStep, Session},
    origins::{ContentId, Traversal, ZoneId},
    player::{Player, capsule},
};
use avian3d::prelude::*;
use bevy::prelude::*;
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, VecDeque},
};
#[derive(Clone)]
struct Node {
    point: Vec3,
    zone: ZoneId,
    ready: bool,
}
#[derive(Clone)]
struct Edge {
    to: usize,
    traversal: Traversal,
    blockers: Vec<ContentId>,
    ready: bool,
    cost: f32,
}
#[derive(Clone)]
pub struct Admission {
    pub position: Vec3,
    pub zone: ZoneId,
    pub source: String,
    pub rising: bool,
    pub activation: ZoneId,
    node: usize,
}
#[derive(Resource, Default)]
pub struct Navigation {
    nodes: Vec<Node>,
    edges: Vec<Vec<Edge>>,
    incoming: Vec<Vec<(usize, usize)>>,
    spawns: Vec<Admission>,
    pub admissions: Vec<Admission>,
    cache: HashMap<(usize, usize), (Vec3, bool)>,
    order: VecDeque<(usize, usize)>,
    pub revision: u32,
    epoch: u32,
    player_node: Option<usize>,
    warmup: u8,
    pub requests: u64,
    pub cache_hits: u64,
    pub fallbacks: u64,
    pub graph_changes: u32,
    pub rejected_nodes: usize,
    pub rejected_links: usize,
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
    pub fn inspect_gate(&self, id: &ContentId, access: &ZoneAccess) -> Option<(Vec3, Vec3, bool)> {
        self.edges.iter().enumerate().find_map(|(from, edges)| {
            edges
                .iter()
                .find(|e| {
                    e.ready && matches!(e.traversal, Traversal::Walk) && e.blockers.contains(id)
                })
                .map(|e| {
                    (
                        self.nodes[from].point,
                        self.nodes[e.to].point,
                        self.allowed(from, e, access),
                    )
                })
        })
    }
    pub fn inspect_gate_nodes(&self, id: &ContentId) -> serde_json::Value {
        let rows: Vec<_> = self.edges.iter().enumerate().flat_map(|(from, edges)| {
            edges.iter().filter(|e| e.blockers.contains(id)).map(move |e| serde_json::json!({
                "from":from,"to":e.to,"from_ready":self.nodes[from].ready,"to_ready":self.nodes[e.to].ready,
                "edge_ready":e.ready,"a":self.nodes[from].point.to_array(),"b":self.nodes[e.to].point.to_array(),"traversal":format!("{:?}",e.traversal)
            }))
        }).collect();
        serde_json::json!(rows)
    }
    pub fn inspect_cache(&mut self, access: &ZoneAccess) -> bool {
        let mut pairs = Vec::new();
        'scan: for (from, edges) in self.edges.iter().enumerate() {
            for e in edges {
                if self.allowed(from, e, access) {
                    pairs.push((self.nodes[from].point, self.nodes[e.to].point));
                    if pairs.len() >= 1000 {
                        break 'scan;
                    }
                }
            }
        }
        for (from, to) in pairs {
            self.destination(from, to, access);
        }
        self.cache.len() <= 512 && self.cache.len() > 0
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn link_count(&self) -> usize {
        self.edges.iter().map(Vec::len).sum()
    }
    pub fn cache_len(&self) -> usize {
        self.cache.len()
    }
    fn nearest(&self, p: Vec3, access: &ZoneAccess, active: bool) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.ready && (!active || access.reachable.contains(&n.zone)))
            .min_by(|(_, a), (_, b)| {
                a.point
                    .distance_squared(p)
                    .total_cmp(&b.point.distance_squared(p))
            })
            .map(|(i, _)| i)
    }
    fn allowed(&self, from: usize, e: &Edge, access: &ZoneAccess) -> bool {
        e.ready
            && matches!(e.traversal, Traversal::Walk)
            && self.nodes[from].ready
            && self.nodes[e.to].ready
            && access.reachable.contains(&self.nodes[from].zone)
            && access.reachable.contains(&self.nodes[e.to].zone)
            && e.blockers.iter().all(|id| access.opened(id))
    }
    pub fn active_links(&self, access: &ZoneAccess) -> usize {
        self.edges
            .iter()
            .enumerate()
            .map(|(from, es)| es.iter().filter(|e| self.allowed(from, e, access)).count())
            .sum()
    }
    pub fn direct_allowed(&self, a: Vec3, b: Vec3, access: &ZoneAccess) -> bool {
        self.nearest(a, access, false)
            .zip(self.nearest(b, access, false))
            .is_some_and(|(a, b)| {
                access.reachable.contains(&self.nodes[a].zone)
                    && access.reachable.contains(&self.nodes[b].zone)
            })
    }
    pub fn destination(&mut self, position: Vec3, target: Vec3, access: &ZoneAccess) -> Vec3 {
        self.requests += 1;
        let Some((start, goal)) = self
            .nearest(position, access, true)
            .zip(self.nearest(target, access, false))
        else {
            self.fallbacks += 1;
            return position;
        };
        if start == goal {
            return if access.reachable.contains(&self.nodes[goal].zone) {
                target
            } else {
                self.nodes[start].point
            };
        }
        if let Some((point, fallback)) = self.cache.get(&(start, goal)).copied() {
            self.cache_hits += 1;
            self.fallbacks += u64::from(fallback);
            return point;
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
        self.fallbacks += u64::from(fallback);
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
        point
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
    pub fn choose_spawn(
        &self,
        session: &mut Session,
        position: Vec3,
        forward: Vec3,
    ) -> Option<&Admission> {
        let nearest_zone = self.player_node.map(|i| &self.nodes[i].zone);
        let mut candidates: Vec<_> = self
            .admissions
            .iter()
            .filter(|s| s.position.distance_squared(position) > 9.)
            .collect();
        if candidates.is_empty() {
            candidates = self
                .admissions
                .iter()
                .filter(|s| s.position.distance_squared(position) > 4.)
                .collect();
        }
        let weights: Vec<u32> = candidates
            .iter()
            .map(|s| {
                let delta = s.position - position;
                let offscreen = forward.dot(delta.normalize_or_zero()) < 0.35;
                let zone = if Some(&s.zone) == nearest_zone { 4 } else { 1 };
                zone * if offscreen { 3 } else { 1 }
            })
            .collect();
        let total: u32 = weights.iter().sum();
        if total == 0 {
            return None;
        }
        let mut choice = session.random() % total;
        session.spawn_cursor = session.spawn_cursor.wrapping_add(1);
        for (candidate, weight) in candidates.into_iter().zip(weights) {
            if choice < weight {
                return Some(candidate);
            }
            choice -= weight;
        }
        None
    }
}
pub struct NavigationPlugin;
impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Navigation>()
            .add_systems(OnEnter(GameState::Playing), reset)
            .add_systems(
                FixedUpdate,
                refresh
                    .in_set(GameStep::EnemyIntent)
                    .before(crate::enemies::lifecycle)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn reset(mut nav: ResMut<Navigation>, players: Query<(), With<Player>>) {
    if !players.is_empty() {
        return;
    }
    nav.cache.clear();
    nav.order.clear();
    nav.admissions.clear();
    nav.epoch = u32::MAX;
    nav.player_node = None;
    nav.revision = 0;
    nav.requests = 0;
    nav.cache_hits = 0;
    nav.fallbacks = 0;
    nav.graph_changes = 0;
}
fn refresh(
    mut nav: ResMut<Navigation>,
    definition: Res<GameDefinition>,
    access: Res<ZoneAccess>,
    query: SpatialQuery,
    players: Query<(Entity, &Transform), With<Player>>,
    blockers: Query<Entity, With<DoorBlocker>>,
    actors: Query<Entity, With<crate::enemies::Zombie>>,
    zone: Res<crate::access::CurrentZone>,
    slide: MoveAndSlide,
) {
    let Ok((owner, pose)) = players.single() else {
        return;
    };
    if nav.nodes.is_empty() {
        nav.warmup += 1;
        if nav.warmup < 3 {
            return;
        }
        let excluded: Vec<_> = std::iter::once(owner)
            .chain(blockers.iter())
            .chain(actors.iter())
            .collect();
        let filter = crate::environment::solids(excluded);
        let shape = capsule(0.3, 1.7);
        let mut ids = HashMap::new();
        for source in &definition.content.navigation_nodes {
            let point = Vec3::from_array(source.position);
            let floor = query.cast_ray(point + Vec3::Y * 0.5, Dir3::NEG_Y, 2., false, &filter);
            let feet = floor.filter(|f| f.normal.y > 0.65).and_then(|f| {
                let floor = point + Vec3::Y * (0.5 - f.distance + 0.03);
                // A center ray misses the capsule's uphill side on stairs/slopes.
                // Keep support within the normal step budget and require full clearance.
                (0..=6)
                    .map(|i| floor + Vec3::Y * (i as f32 * 0.05))
                    .find(|feet| {
                        query
                            .shape_intersections(
                                &shape,
                                *feet + Vec3::Y * 0.85,
                                Quat::IDENTITY,
                                &filter,
                            )
                            .is_empty()
                    })
            });
            let ready = feet.is_some();
            ids.insert(source.id.clone(), nav.nodes.len());
            nav.nodes.push(Node {
                point: feet.unwrap_or(point),
                zone: source.zone.clone(),
                ready,
            });
            nav.rejected_nodes += usize::from(!ready);
        }
        let count = nav.nodes.len();
        nav.edges = vec![Vec::new(); count];
        for source in &definition.content.navigation_links {
            let (Some(&from), Some(&to)) = (ids.get(&source.from), ids.get(&source.to)) else {
                continue;
            };
            let delta = nav.nodes[to].point - nav.nodes[from].point;
            let ready = nav.nodes[from].ready
                && nav.nodes[to].ready
                && (!matches!(source.traversal, Traversal::Walk)
                    || walking_edge(
                        &query,
                        &slide,
                        &shape,
                        nav.nodes[from].point,
                        nav.nodes[to].point,
                        definition.tuning.step_height,
                        &filter,
                    ));
            let mut requirements = source.blockers.clone();
            requirements.extend(source.connection.iter().cloned());
            nav.edges[from].push(Edge {
                to,
                traversal: source.traversal,
                blockers: requirements,
                ready,
                cost: delta.length().max(0.01),
            });
            nav.rejected_links += usize::from(!ready);
        }
        let mut incoming = vec![Vec::new(); count];
        for (from, edges) in nav.edges.iter().enumerate() {
            for (ordinal, edge) in edges.iter().enumerate() {
                incoming[edge.to].push((from, ordinal));
            }
        }
        nav.incoming = incoming;
        for spawn in &definition.access.spawns {
            nav.spawns.push(Admission {
                position: Vec3::from_array(spawn.position),
                zone: spawn.zone.clone(),
                source: spawn.id.clone(),
                rising: spawn.rising,
                activation: spawn.zone.clone(),
                node: usize::MAX,
            });
        }
        info!(
            "Native navigation: {} nodes, {} links, {} spawn projections",
            nav.node_count(),
            nav.link_count(),
            nav.spawns.len()
        );
    }
    let feet = pose.translation - Vec3::Y * 0.9;
    let node = if let Some(zone) = &zone.0 {
        nav.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.ready && n.zone == *zone && access.reachable.contains(zone))
            .min_by(|(_, a), (_, b)| {
                a.point
                    .distance_squared(feet)
                    .total_cmp(&b.point.distance_squared(feet))
            })
            .map(|(i, _)| i)
    } else {
        nav.nearest(feet, &access, true)
    };
    if nav.epoch != access.purchases {
        nav.cache.clear();
        nav.order.clear();
        nav.epoch = access.purchases;
        nav.revision = nav.revision.wrapping_add(1);
        nav.graph_changes += 1;
    } else if nav.player_node == node {
        return;
    }
    nav.player_node = node;
    let Some(start) = node else {
        nav.admissions.clear();
        return;
    };
    let mut reachable = vec![false; nav.nodes.len()];
    let mut work = vec![start];
    reachable[start] = true;
    while let Some(to) = work.pop() {
        // Admission requires an enemy route toward the player, including one-way links.
        for &(from, ordinal) in &nav.incoming[to] {
            if !reachable[from] && nav.allowed(from, &nav.edges[from][ordinal], &access) {
                reachable[from] = true;
                work.push(from);
            }
        }
    }
    nav.admissions = nav
        .spawns
        .iter()
        .filter_map(|s| {
            if !access.reachable.contains(&s.activation) {
                return None;
            }
            let (node, n) = nav
                .nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| reachable[*i])
                .min_by(|(_, a), (_, b)| {
                    a.point
                        .distance_squared(s.position)
                        .total_cmp(&b.point.distance_squared(s.position))
                })?;
            if n.point.distance_squared(s.position) > 100. {
                return None;
            }
            let mut admission = s.clone();
            admission.node = node;
            admission.zone = n.zone.clone();
            admission.position = n.point;
            Some(admission)
        })
        .collect();
}
fn walking_edge(
    query: &SpatialQuery,
    slide: &MoveAndSlide,
    shape: &Collider,
    a: Vec3,
    b: Vec3,
    step: f32,
    filter: &SpatialQueryFilter,
) -> bool {
    let delta = b - a;
    if delta.length_squared() < 0.0001 {
        return true;
    }
    let Ok(dir) = Dir3::new(delta) else {
        return false;
    };
    if query
        .cast_shape(
            shape,
            a + Vec3::Y * 0.85,
            Quat::IDENTITY,
            dir,
            &ShapeCastConfig {
                ignore_origin_penetration: true,
                ..ShapeCastConfig::from_max_distance(delta.length())
            },
            filter,
        )
        .is_none()
    {
        return true;
    }
    let count = (delta.length() / 0.6).ceil().clamp(1., 64.) as u32;
    let mut center = a + Vec3::Y * 0.85;
    for i in 1..=count {
        let goal = a + delta * (i as f32 / count as f32);
        let direction = goal.xz() - center.xz();
        let velocity = Vec3::new(direction.x / 0.1, -3., direction.y / 0.1);
        let result = crate::player::slide_character(
            slide,
            query,
            crate::player::MotionShape {
                collider: shape,
                radius: 0.3,
                step_height: step,
                grounded: true,
            },
            center,
            velocity,
            std::time::Duration::from_secs_f32(0.1),
            filter,
        );
        center = result.position;
    }
    center.distance_squared(b + Vec3::Y * 0.85) < 0.1225
}
