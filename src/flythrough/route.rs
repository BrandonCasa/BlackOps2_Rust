use crate::assets::GameDefinition;
use bevy::prelude::*;
use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
};

pub(super) struct Shot {
    pub label: String,
    pub path: Vec<Vec3>,
    pub target: Option<Vec3>,
    pub seconds: f64,
    pub cut: bool,
    length: f32,
}

impl Shot {
    fn new(label: String, path: Vec<Vec3>, target: Option<Vec3>, seconds: f64, cut: bool) -> Self {
        let length = path.windows(2).map(|p| p[0].distance(p[1])).sum();
        Self {
            label,
            path,
            target,
            seconds,
            cut,
            length,
        }
    }

    pub fn pose(&self, fraction: f32) -> Transform {
        let u = fraction.clamp(0., 1.);
        let u = u * u * (3. - 2. * u);
        let point = self.point(u * self.length);
        let target = self.target.unwrap_or_else(|| {
            let ahead = self.point((u * self.length + 2.).min(self.length));
            if ahead.distance_squared(point) > 0.01 {
                ahead
            } else {
                point + (point - self.point((self.length - 2.).max(0.))).normalize_or(Vec3::NEG_Z)
            }
        });
        Transform::from_translation(point).looking_at(target, Vec3::Y)
    }

    fn point(&self, mut distance: f32) -> Vec3 {
        for pair in self.path.windows(2) {
            let length = pair[0].distance(pair[1]);
            if distance <= length && length > 0.0001 {
                return pair[0].lerp(pair[1], distance / length);
            }
            distance -= length;
        }
        *self.path.last().unwrap_or(&Vec3::ZERO)
    }
}

struct Graph {
    nodes: Vec<Vec3>,
    edges: Vec<Vec<usize>>,
}

impl Graph {
    fn new(game: &GameDefinition) -> Self {
        let nodes: Vec<_> = game
            .content
            .navigation_nodes
            .iter()
            .map(|n| Vec3::from_array(n.position) + Vec3::Y * 1.8)
            .collect();
        let indices: HashMap<_, _> = game
            .content
            .navigation_nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (&n.id, i))
            .collect();
        let mut edges = vec![Vec::new(); nodes.len()];
        for edge in &game.content.navigation_links {
            if matches!(
                edge.traversal,
                crate::origins::Traversal::Walk
                    | crate::origins::Traversal::Climb
                    | crate::origins::Traversal::Jump
            ) && let (Some(&a), Some(&b)) = (indices.get(&edge.from), indices.get(&edge.to))
            {
                edges[a].push(b);
                edges[b].push(a);
            }
        }
        Self { nodes, edges }
    }

    fn nearest(&self, position: Vec3) -> Option<usize> {
        self.nodes
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(position)
                    .total_cmp(&b.distance_squared(position))
            })
            .map(|(i, _)| i)
    }

    fn path(&self, start: Vec3, end: Vec3) -> Option<Vec<Vec3>> {
        let (a, b) = (self.nearest(start)?, self.nearest(end)?);
        let mut costs = vec![u64::MAX; self.nodes.len()];
        let mut parents = vec![usize::MAX; self.nodes.len()];
        let mut queue = BinaryHeap::from([Reverse((0u64, a))]);
        costs[a] = 0;
        while let Some(Reverse((cost, index))) = queue.pop() {
            if index == b {
                break;
            }
            if cost != costs[index] {
                continue;
            }
            for &next in &self.edges[index] {
                let candidate =
                    cost + (self.nodes[index].distance(self.nodes[next]) * 1000.) as u64;
                if candidate < costs[next] {
                    costs[next] = candidate;
                    parents[next] = index;
                    queue.push(Reverse((candidate, next)));
                }
            }
        }
        if costs[b] == u64::MAX {
            return None;
        }
        let mut path = vec![end];
        let mut index = b;
        loop {
            path.push(self.nodes[index]);
            if index == a {
                break;
            }
            index = parents[index];
        }
        path.push(start);
        path.reverse();
        Some(path)
    }
}

pub(super) fn build(game: &GameDefinition, lamps: &[Vec3]) -> Vec<Shot> {
    let graph = Graph::new(game);
    let spawn = Vec3::from_array(game.spawn) + Vec3::Y * game.tuning.eye_height;
    let mut shots = vec![Shot::new(
        "Spawn bunker".into(),
        vec![spawn, spawn + Vec3::new(-1., 0.2, 0.)],
        Some(spawn + Quat::from_rotation_y(game.yaw) * Vec3::NEG_Z * 5.),
        8.,
        false,
    )];
    let mut last = *shots[0].path.last().unwrap();
    let mut used_lamps = Vec::<usize>::new();
    let itinerary = [
        ("placement.quick_revive", "Quick Revive"),
        ("placement.generator.generator_start_bunker", "Generator 1"),
        ("placement.speed_cola", "Speed Cola / trenches"),
        ("placement.generator.generator_tank_trench", "Tank station"),
        (
            "placement.generator.generator_mid_trench",
            "Generator 3 / workshop",
        ),
        ("placement.juggernog", "Juggernog / No Man's Land"),
        ("excavation", "Excavation site"),
        ("staff_holders", "Staff chamber / ancient pedestals"),
        ("placement.mule_kick", "Mule Kick / underground"),
        ("placement.stamin_up", "Stamin-Up"),
        ("placement.generator.generator_nml_left", "Generator 5"),
        ("church", "Church ruins"),
        (
            "placement.generator.generator_church",
            "Generator 6 / churchyard",
        ),
        ("crazy_place", "The Crazy Place"),
    ];
    for (id, label) in itinerary {
        let (target, yaw, radius) =
            if let Some(p) = game.content.placements.iter().find(|p| p.id.0 == id) {
                (
                    Vec3::from_array(p.position) + Vec3::Y * 1.15,
                    p.yaw,
                    if p.definition.0.starts_with("perk.") {
                        3.4
                    } else {
                        4.5
                    },
                )
            } else {
                match id {
                    "excavation" => (Vec3::new(0., 8.8, 0.), 0.5, 16.),
                    "church" => (Vec3::new(12., 12., 70.), 3.1, 9.),
                    "staff_holders" => (Vec3::new(-1.58, -18.2, -6.618), 0.1745, 2.8),
                    "crazy_place" => (Vec3::new(262.6, -10., 200.8), 0.7, 6.),
                    _ => continue,
                }
            };
        let rotation = Quat::from_rotation_y(yaw);
        let mut arc: Vec<_> = (0..=24)
            .map(|i| {
                let angle = -0.28 + i as f32 / 24. * 0.56;
                target
                    + rotation
                        * Vec3::new(
                            angle.sin() * radius,
                            0.15 + (i as f32 / 24.) * 0.2,
                            angle.cos() * radius,
                        )
            })
            .collect();
        let survey = match id {
            "excavation" => Some(2),
            "church" => Some(3),
            "crazy_place" => Some(5),
            _ => None,
        };
        if let Some(view) = survey.and_then(|i| game.survey_views.get(i)) {
            let eye = Vec3::from_array(*view) + Vec3::Y * 2.;
            arc = vec![eye, eye + Vec3::new(1.5, 0.3, 0.)];
        }
        travel(&graph, &mut shots, last, arc[0], label);
        last = *arc.last().unwrap();
        shots.push(Shot::new(
            label.into(),
            arc,
            Some(target),
            if radius < 5. { 10. } else { 12. },
            false,
        ));
        if let Some((index, &lamp)) = lamps
            .iter()
            .enumerate()
            .filter(|(i, p)| !used_lamps.contains(i) && p.distance(target) < 20.)
            .min_by(|(_, a), (_, b)| {
                a.distance_squared(target)
                    .total_cmp(&b.distance_squared(target))
            })
        {
            used_lamps.push(index);
            let toward = (last - lamp).with_y(0.).normalize_or(Vec3::Z);
            let side = Vec3::Y.cross(toward);
            let path = vec![
                lamp + toward * 1.4 - side * 0.35,
                lamp + toward * 1.2 + side * 0.35,
            ];
            travel(&graph, &mut shots, last, path[0], "Lantern detail");
            last = path[1];
            shots.push(Shot::new(
                format!("Lantern detail / {label}"),
                path,
                Some(lamp),
                7.,
                false,
            ));
        }
    }
    travel(&graph, &mut shots, last, spawn, "Return to spawn");
    shots
}

fn travel(graph: &Graph, shots: &mut Vec<Shot>, start: Vec3, end: Vec3, label: &str) {
    if let Some(path) = graph.path(start, end) {
        let length: f32 = path.windows(2).map(|p| p[0].distance(p[1])).sum();
        if length < 400. {
            shots.push(Shot::new(
                format!("Flying to {label}"),
                path,
                None,
                (length as f64 / 9.).max(2.),
                false,
            ));
            return;
        }
    }
    shots.push(Shot::new(
        format!("Transition to {label}"),
        vec![end, end + Vec3::Y * 0.1],
        Some(end + Vec3::NEG_Z),
        2.,
        true,
    ));
}
