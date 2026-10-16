//! T6 DPVS: 0040f7c0 outside-world handling, 0040d4e0 portal walk,
//! 00409010 clipping, 00408770 portal cones and 004589a0 cell AABB trees.
//! Camera admission is independent of gameplay visibility and shadow views.
use std::{any::TypeId, collections::VecDeque, path::Path};

use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{NoFrustumCulling, VisibleEntities},
    },
    prelude::*,
};
use serde::Deserialize;

type Bounds = [[f32; 3]; 2];

/// zm_tomb.gsc sets 5500 for the ordinary game and 2500 only for more than
/// two split-screen players. 0050edd0 -> 0040fd20 supplies the DPVS far plane.
pub(crate) const CULL_DISTANCE: f32 = 5500.;

#[derive(Resource, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VisibilityWorld {
    surface_count: usize,
    model_count: usize,
    sorted_surfaces: Vec<u16>,
    surface_bounds: Vec<Bounds>,
    model_bounds: Vec<Bounds>,
    cells: Vec<Cell>,
    occluders: Vec<Occluder>,
}

#[derive(Deserialize)]
struct Cell {
    bounds: Bounds,
    trees: Vec<Tree>,
    portals: Vec<Portal>,
}

#[derive(Deserialize)]
struct Tree {
    bounds: Bounds,
    children: [usize; 2],
    surfaces: [usize; 2],
    models: Vec<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Portal {
    cell: usize,
    plane: Vec4,
    vertices: Vec<Vec3>,
    hull_axes: [Vec3; 2],
    bounds: Bounds,
}

#[derive(Deserialize)]
struct Occluder {
    flags: u32,
    points: [Vec3; 4],
}

/// Only immutable BSP surfaces have an index here. Moving brush surfaces use
/// their current bounds; their authored position must not determine admission.
#[derive(Component)]
pub(crate) struct BspSurface(pub usize);

impl VisibilityWorld {
    pub(crate) fn load(root: &Path, surfaces: usize, models: usize, cells: usize) -> Self {
        let path = root.join("world_visibility.json");
        let bytes = std::fs::read(&path).unwrap_or_else(|error| {
            panic!("Read original DPVS {}: {error}. Run tools/native_visibility.py or the asset export.", path.display())
        });
        let world: Self = serde_json::from_slice(&bytes).expect("Original DPVS definitions");
        assert_eq!(world.surface_count, surfaces, "Original DPVS surface count");
        assert_eq!(world.model_count, models, "Original DPVS model count");
        assert_eq!(world.cells.len(), cells, "Original DPVS BSP cell count");
        world.validate();
        world
    }

    fn validate(&self) {
        assert!(!self.cells.is_empty() && self.cells.len() <= 64);
        assert_eq!(self.surface_bounds.len(), self.surface_count);
        assert_eq!(self.model_bounds.len(), self.model_count);
        let valid_bounds = |b: &Bounds| {
            b.iter().flatten().all(|v| v.is_finite()) && (0..3).all(|axis| b[0][axis] <= b[1][axis])
        };
        assert!(
            self.surface_bounds
                .iter()
                .chain(&self.model_bounds)
                .all(valid_bounds)
        );
        assert!(
            self.sorted_surfaces
                .iter()
                .all(|&i| usize::from(i) < self.surface_count)
        );
        let mut surfaces = vec![false; self.surface_count];
        let mut models = vec![false; self.model_count];
        for cell in &self.cells {
            assert!(valid_bounds(&cell.bounds) && !cell.trees.is_empty());
            let mut pending = vec![0];
            let mut seen = vec![false; cell.trees.len()];
            while let Some(index) = pending.pop() {
                assert!(
                    !seen[index],
                    "Original DPVS tree must not cycle or share children"
                );
                seen[index] = true;
                let node = &cell.trees[index];
                assert!(valid_bounds(&node.bounds));
                let [first, count] = node.children;
                assert!(count == 0 || first > index && first + count <= cell.trees.len());
                pending.extend(first..first + count);
                let [first, count] = node.surfaces;
                assert!(first + count <= self.sorted_surfaces.len());
                for &i in &self.sorted_surfaces[first..first + count] {
                    surfaces[usize::from(i)] = true;
                }
                for &i in &node.models {
                    assert!(usize::from(i) < self.model_count);
                    models[usize::from(i)] = true;
                }
            }
            assert!(
                seen.iter().all(|v| *v),
                "Unreachable original DPVS tree nodes"
            );
            for portal in &cell.portals {
                assert!(portal.cell < self.cells.len());
                assert!((3..=128).contains(&portal.vertices.len()));
                assert!(portal.plane.is_finite() && portal.vertices.iter().all(|v| v.is_finite()));
                assert!(portal.hull_axes.iter().all(|v| v.is_finite()));
                assert!(valid_bounds(&portal.bounds));
            }
        }
        assert!(
            self.sorted_surfaces
                .iter()
                .all(|&i| surfaces[usize::from(i)])
        );
        assert!(
            models.iter().all(|v| *v),
            "Original static models missing from DPVS trees"
        );
        assert!(
            self.occluders
                .iter()
                .flat_map(|o| &o.points)
                .all(|p| p.is_finite())
        );
    }
}

#[derive(Clone, Copy, Default)]
struct Volume {
    planes: [Vec4; 16],
    count: usize,
}

impl Volume {
    fn push(&mut self, plane: Vec4) {
        assert!(self.count < self.planes.len(), "Original DPVS plane budget");
        self.planes[self.count] = plane;
        self.count += 1;
    }

    fn planes(&self) -> &[Vec4] {
        &self.planes[..self.count]
    }

    fn intersects(&self, bounds: &Bounds) -> bool {
        self.planes().iter().all(|p| support(*p, bounds) > 0.)
    }
}

fn support(plane: Vec4, bounds: &Bounds) -> f32 {
    let point = Vec3::new(
        bounds[usize::from(plane.x >= 0.)][0],
        bounds[usize::from(plane.y >= 0.)][1],
        bounds[usize::from(plane.z >= 0.)][2],
    );
    plane.truncate().dot(point) + plane.w
}

fn normalized(plane: Vec4) -> Vec4 {
    plane / plane.truncate().length()
}

/// 00409010 classifies at distance 0 and 0.002, intersecting edges at 0.001.
/// Keep source inches here; a metric epsilon changes doorway admission.
fn clip_polygon(points: &[Vec3], plane: Vec4, output: &mut Vec<Vec3>) -> bool {
    output.clear();
    if !points
        .iter()
        .any(|p| plane.truncate().dot(*p) + plane.w > 0.002)
    {
        return false;
    }
    let classify = |p: Vec3| {
        let distance = plane.truncate().dot(p) + plane.w - 0.001;
        (
            distance,
            if distance < -0.001 {
                -1
            } else if distance > 0.001 {
                1
            } else {
                0
            },
        )
    };
    let mut a = points[0];
    let (mut da, mut ca) = classify(a);
    for i in 0..points.len() {
        let b = points[(i + 1) % points.len()];
        let (db, cb) = classify(b);
        if ca >= 0 {
            output.push(a);
        }
        if ca != 0 && cb != 0 && ca != cb {
            output.push(a + (b - a) * (da / (da - db)));
        }
        a = b;
        da = db;
        ca = cb;
    }
    !output.is_empty()
}

fn cross(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b - a).perp_dot(c - a)
}

fn hull(points: &mut Vec<Vec2>, output: &mut Vec<Vec2>) {
    points.sort_unstable_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    output.clear();
    for &p in points.iter() {
        while output.len() >= 2
            && cross(output[output.len() - 2], output[output.len() - 1], p) <= 0.
        {
            output.pop();
        }
        output.push(p);
    }
    let lower = output.len();
    for &p in points.iter().rev().skip(1) {
        while output.len() > lower
            && cross(output[output.len() - 2], output[output.len() - 1], p) <= 0.
        {
            output.pop();
        }
        output.push(p);
    }
    if output.len() > 1 {
        output.pop();
    }
}

#[derive(Default)]
struct PortalHull {
    points: Vec<Vec2>,
    ancestors: u64,
    depth: usize,
    queued: bool,
}

#[derive(Resource, Default)]
pub(crate) struct SourceVisibility {
    pub(crate) surfaces: Vec<bool>,
    pub(crate) models: Vec<bool>,
    cells: Vec<bool>,
    eye: Option<Vec3>,
    clip: Mat4,
    near: Vec4,
    far: Vec4,
    base: Volume,
    occluders: Vec<[Vec4; 5]>,
    portals: Vec<PortalHull>,
    portal_offsets: Vec<usize>,
    queue: VecDeque<(usize, usize)>,
    polygon: Vec<Vec3>,
    clipped: Vec<Vec3>,
    merged: Vec<Vec2>,
    convex: Vec<Vec2>,
    pub(crate) tree_visits: usize,
    pub(crate) portal_visits: usize,
}

impl SourceVisibility {
    fn occluded(&self, bounds: &Bounds) -> bool {
        // 004582d0: every positive-support corner must be strictly behind all
        // five planes of one authored occluder. Straddling boxes remain visible.
        self.occluders
            .iter()
            .any(|planes| planes.iter().all(|p| support(*p, bounds) < 0.))
    }

    fn update(
        &mut self,
        world: &VisibilityWorld,
        cell: Option<usize>,
        eye: Vec3,
        clip: Mat4,
        forward: Vec3,
    ) {
        self.eye = Some(eye);
        self.clip = clip;
        self.surfaces.resize(world.surface_count, false);
        self.models.resize(world.model_count, false);
        self.cells.resize(world.cells.len(), false);
        self.surfaces.fill(false);
        self.models.fill(false);
        self.cells.fill(false);
        self.tree_visits = 0;
        self.portal_visits = 0;
        self.queue.clear();
        self.portal_offsets.clear();
        let mut count = 0;
        for cell in &world.cells {
            self.portal_offsets.push(count);
            count += cell.portals.len();
        }
        self.portals.resize_with(count, PortalHull::default);
        for portal in &mut self.portals {
            portal.points.clear();
            portal.queued = false;
        }
        self.base = Volume::default();
        for plane in [
            clip.row(3) + clip.row(0),
            clip.row(3) - clip.row(0),
            clip.row(3) + clip.row(1),
            clip.row(3) - clip.row(1),
        ] {
            self.base.push(normalized(plane));
        }
        // 0040e7a0 permits one tenth of an inch behind the eye for DPVS,
        // independently of r_znear's four-inch raster clipping plane.
        self.near = forward.extend(0.1 - forward.dot(eye));
        self.far = (-forward).extend(CULL_DISTANCE + forward.dot(eye));
        self.base.push(self.near);
        self.base.push(self.far);
        self.occluders.clear();
        for occluder in &world.occluders {
            if occluder.flags & 1 == 0 {
                continue;
            }
            let p = occluder.points;
            let center = p.iter().sum::<Vec3>() * 0.25;
            let mut face = (p[3] - p[0]).cross(p[1] - p[0]).normalize_or_zero();
            if face.dot(eye - p[0]) < 0. {
                face = -face;
            }
            let mut planes = [face.extend(-face.dot(p[0])); 5];
            for i in 0..4 {
                let mut normal = (p[(i + 1) % 4] - p[i])
                    .cross(eye - p[i])
                    .normalize_or_zero();
                if normal.dot(center - p[i]) > 0. {
                    normal = -normal;
                }
                planes[i + 1] = normal.extend(-normal.dot(p[i]));
            }
            self.occluders.push(planes);
        }
        if let Some(cell) = cell {
            self.walk(world, cell, self.base, 0, 0, false);
            while let Some((owner, index)) = self.queue.pop_front() {
                let slot = self.portal_offsets[owner] + index;
                self.portals[slot].queued = false;
                let portal = &world.cells[owner].portals[index];
                self.polygon.clear();
                self.polygon
                    .extend(self.portals[slot].points.iter().map(|p| {
                        portal.hull_axes[0] * p.x + portal.hull_axes[1] * p.y
                            - portal.plane.truncate() * portal.plane.w
                    }));
                let (volume, large) = self.portal_volume();
                let state = &self.portals[slot];
                let (depth, ancestors) = (state.depth, state.ancestors);
                // 00407e60/0040d7c0: small portals stop tightening the cone
                // after two parents. They still visit connected cells.
                self.walk(
                    world,
                    portal.cell,
                    volume,
                    ancestors,
                    depth + 1,
                    !large && depth >= 2,
                );
            }
        } else {
            // 0040f7c0: an eye outside the BSP submits every cell, never an
            // empty PVS. The ordinary view frustum and occluders still apply.
            for cell in 0..world.cells.len() {
                self.cells[cell] = true;
                self.tree(world, cell, 0, self.base);
            }
        }
    }

    fn tree(&mut self, world: &VisibilityWorld, cell: usize, index: usize, volume: Volume) {
        self.tree_visits += 1;
        let tree = &world.cells[cell].trees[index];
        if !volume.intersects(&tree.bounds) || self.occluded(&tree.bounds) {
            return;
        }
        let [first, count] = tree.children;
        if count > 0 {
            for child in first..first + count {
                self.tree(world, cell, child, volume);
            }
            return;
        }
        let [first, count] = tree.surfaces;
        for &index in &world.sorted_surfaces[first..first + count] {
            let i = usize::from(index);
            if !self.surfaces[i]
                && volume.intersects(&world.surface_bounds[i])
                && !self.occluded(&world.surface_bounds[i])
            {
                self.surfaces[i] = true;
            }
        }
        for &index in &tree.models {
            let i = usize::from(index);
            if !self.models[i]
                && volume.intersects(&world.model_bounds[i])
                && !self.occluded(&world.model_bounds[i])
            {
                self.models[i] = true;
            }
        }
    }

    fn walk(
        &mut self,
        world: &VisibilityWorld,
        cell: usize,
        volume: Volume,
        ancestors: u64,
        depth: usize,
        flood: bool,
    ) {
        let bit = 1u64 << cell;
        if ancestors & bit != 0 {
            return;
        }
        let ancestors = ancestors | bit;
        self.cells[cell] = true;
        self.tree(world, cell, 0, volume);
        for (index, portal) in world.cells[cell].portals.iter().enumerate() {
            if ancestors & (1 << portal.cell) != 0 || self.occluded(&portal.bounds) {
                continue;
            }
            self.portal_visits += 1;
            let distance = portal.plane.truncate().dot(self.eye.unwrap()) + portal.plane.w;
            if distance > 0.125 || !volume.intersects(&portal.bounds) {
                continue;
            }
            if !flood && distance > -0.125 {
                // 0040d4e0 handles near-coplanar portals before polygon
                // clipping: at the eye plane the clipped cone has zero area.
                let point = self.eye.unwrap() - portal.plane.truncate() * distance;
                let center = portal.vertices.iter().sum::<Vec3>() / portal.vertices.len() as f32;
                let inside = (0..portal.vertices.len()).all(|i| {
                    let a = portal.vertices[i];
                    let edge = portal.vertices[(i + 1) % portal.vertices.len()] - a;
                    let normal = edge.cross(portal.plane.truncate());
                    normal.dot(point - a) * normal.dot(center - a) >= 0.
                });
                if inside {
                    self.walk(world, portal.cell, volume, ancestors, depth + 1, false);
                }
                continue;
            }
            self.polygon.clone_from(&portal.vertices);
            for &plane in volume.planes() {
                if !clip_polygon(&self.polygon, plane, &mut self.clipped) {
                    self.polygon.clear();
                    break;
                }
                std::mem::swap(&mut self.polygon, &mut self.clipped);
            }
            if self.polygon.is_empty() {
                continue;
            }
            if flood {
                self.flood(world, portal.cell, volume, ancestors);
            } else {
                let slot = self.portal_offsets[cell] + index;
                let state = &mut self.portals[slot];
                let first = state.points.is_empty();
                let new_path = !first && state.ancestors & ancestors != state.ancestors;
                let contained = !first
                    && self.polygon.iter().all(|p| {
                        let p = Vec2::new(portal.hull_axes[0].dot(*p), portal.hull_axes[1].dot(*p));
                        (0..state.points.len()).all(|i| {
                            cross(
                                state.points[i],
                                state.points[(i + 1) % state.points.len()],
                                p,
                            ) >= -0.0001
                        })
                    });
                if contained && !new_path {
                    continue;
                }
                self.merged.clone_from(&state.points);
                self.merged.extend(
                    self.polygon.iter().map(|p| {
                        Vec2::new(portal.hull_axes[0].dot(*p), portal.hull_axes[1].dot(*p))
                    }),
                );
                hull(&mut self.merged, &mut self.convex);
                state.points.clone_from(&self.convex);
                state.ancestors = if first {
                    ancestors
                } else {
                    state.ancestors & ancestors
                };
                state.depth = if first { depth } else { state.depth.min(depth) };
                if !state.queued {
                    state.queued = true;
                    self.queue.push_back((cell, index));
                }
            }
        }
    }

    fn flood(&mut self, world: &VisibilityWorld, cell: usize, volume: Volume, mut visited: u64) {
        let mut pending = [0usize; 64];
        let mut count = 1;
        pending[0] = cell;
        visited |= 1 << cell;
        while count > 0 {
            count -= 1;
            let cell = pending[count];
            self.cells[cell] = true;
            self.tree(world, cell, 0, volume);
            for portal in &world.cells[cell].portals {
                let bit = 1 << portal.cell;
                if visited & bit != 0
                    || portal.plane.truncate().dot(self.eye.unwrap()) + portal.plane.w > 0.125
                {
                    continue;
                }
                self.polygon.clone_from(&portal.vertices);
                for &plane in volume.planes() {
                    if !clip_polygon(&self.polygon, plane, &mut self.clipped) {
                        self.polygon.clear();
                        break;
                    }
                    std::mem::swap(&mut self.polygon, &mut self.clipped);
                }
                if !self.polygon.is_empty() {
                    visited |= bit;
                    pending[count] = portal.cell;
                    count += 1;
                }
            }
        }
    }

    fn portal_volume(&self) -> (Volume, bool) {
        let mut min = Vec2::ONE;
        let mut max = -Vec2::ONE;
        let mut large = false;
        let mut projected = [Vec2::ZERO; 128];
        assert!(
            self.polygon.len() <= projected.len(),
            "Original clipped portal hull capacity"
        );
        for (point, output) in self.polygon.iter().zip(&mut projected) {
            let clip = self.clip * point.extend(1.);
            // Source clip W is in inches, including its 1/8-inch guard.
            if clip.w < 0.125 {
                min = -Vec2::ONE;
                max = Vec2::ONE;
                large = true;
                break;
            }
            *output = clip.truncate().truncate() / clip.w;
            min = min.min(*output);
            max = max.max(*output);
        }
        if !large && (max - min).element_product() * 0.25 >= 0.02 {
            let points = &projected[..self.polygon.len()];
            let area: f32 = (0..points.len())
                .map(|i| points[i].perp_dot(points[(i + 1) % points.len()]))
                .sum();
            large = area.abs() * 0.125 >= 0.02;
        }
        let mut volume = Volume::default();
        for plane in [
            self.clip.row(0) - self.clip.row(3) * min.x,
            self.clip.row(3) * max.x - self.clip.row(0),
            self.clip.row(1) - self.clip.row(3) * min.y,
            self.clip.row(3) * max.y - self.clip.row(1),
        ] {
            volume.push(normalized(plane));
        }
        if self.polygon.len() <= 10 {
            let eye = self.eye.unwrap();
            let center = self.polygon.iter().sum::<Vec3>() / self.polygon.len() as f32;
            for i in 0..self.polygon.len() {
                let a = self.polygon[i];
                let b = self.polygon[(i + 1) % self.polygon.len()];
                let mut normal = (b - eye).cross(a - eye).normalize_or_zero();
                if normal.dot(center - a) < 0. {
                    normal = -normal;
                }
                if normal != Vec3::ZERO {
                    volume.push(normal.extend(0.001 - normal.dot(a)));
                }
            }
        }
        volume.push(self.near);
        volume.push(self.far);
        (volume, large)
    }

    pub(crate) fn dynamic_visible(&self, world: &VisibilityWorld, bounds: &Bounds) -> bool {
        if !self.base.intersects(bounds) || self.occluded(bounds) {
            return false;
        }
        let mut in_world = false;
        for (cell, &visible) in world.cells.iter().zip(&self.cells) {
            if (0..3)
                .all(|a| bounds[1][a] >= cell.bounds[0][a] && bounds[0][a] <= cell.bounds[1][a])
            {
                in_world = true;
                if visible {
                    return true;
                }
            }
        }
        // Large robots and effects can extend outside the BSP's cells.
        !in_world
    }
}

#[cfg(test)]
mod tests;

pub(crate) fn update_visibility(
    world: Option<Res<VisibilityWorld>>,
    bsp: Option<Res<super::reflection::ReflectionWorld>>,
    cameras: Query<(&Camera, &GlobalTransform), With<super::GameCamera>>,
    mut visible: ResMut<SourceVisibility>,
) {
    let (Some(world), Some(bsp), Ok((camera, transform))) = (world, bsp, cameras.single()) else {
        return;
    };
    let frame = super::world::source_frame();
    let eye = frame.inverse().transform_point3(transform.translation());
    let clip = camera.clip_from_view()
        * transform.to_matrix().inverse()
        * frame
        * crate::assets::SOURCE_UNIT_METERS.recip();
    if visible.eye == Some(eye) && visible.clip == clip && !world.is_changed() {
        return;
    }
    let forward = frame
        .inverse()
        .transform_vector3(*transform.forward())
        .normalize();
    visible.update(&world, bsp.cell(eye.to_array()), eye, clip, forward);
}

pub(crate) fn filter_visibility(
    world: Option<Res<VisibilityWorld>>,
    source: Res<SourceVisibility>,
    mut cameras: Query<&mut VisibleEntities, With<super::GameCamera>>,
    surfaces: Query<(
        Option<&BspSurface>,
        Option<&super::native_model::NativeModelSurface>,
        Option<&Aabb>,
        &GlobalTransform,
        Has<NoFrustumCulling>,
    )>,
    instances: Query<&super::models::StaticModelInstance>,
) {
    let Some(world) = world else {
        return;
    };
    if source.eye.is_none() {
        return;
    }
    let frame_inverse = super::world::source_frame().inverse();
    for mut camera in &mut cameras {
        for class in [
            TypeId::of::<super::native::NativeWorldSurface>(),
            TypeId::of::<super::native_model::NativeModelSurface>(),
            TypeId::of::<super::native_water::NativeWaterSurface>(),
        ] {
            camera.get_mut(class).retain(|&entity| {
                let Ok((bsp, model, bounds, transform, no_culling)) = surfaces.get(entity) else {
                    return true;
                };
                if no_culling {
                    return true;
                }
                if let Some(bsp) = bsp {
                    return source.surfaces[bsp.0];
                }
                if let Some(model) = model
                    && let Ok(instance) = instances.get(model.placement)
                {
                    return source.models[instance.source_index as usize];
                }
                let Some(bounds) = bounds else {
                    return true;
                };
                let matrix = frame_inverse * transform.to_matrix();
                let center = matrix.transform_point3(bounds.center.into());
                let extents = Mat3::from_mat4(matrix).abs() * Vec3::from(bounds.half_extents);
                let bounds = [(center - extents).to_array(), (center + extents).to_array()];
                source.dynamic_visible(&world, &bounds)
            });
        }
    }
}
