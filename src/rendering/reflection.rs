//! Dynamic probe selection: 00409490 volumes, 00408ef0 BSP, 00409230
//! cell-local nearest probe, and 00409410 outside-world fallback.
use super::models::ReflectionProbe;
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReflectionWorld {
    #[serde(rename = "reflectionProbes")]
    pub probes: Vec<ReflectionProbe>,
    reflection_cells: Vec<Vec<u8>>,
    reflection_planes: Vec<[f32; 4]>,
    reflection_nodes: Vec<u16>,
}

impl ReflectionWorld {
    pub(crate) fn validate(&self) {
        assert!(!self.probes.is_empty() && self.probes.len() <= 256);
        assert!(!self.reflection_nodes.is_empty());
        assert!(
            self.reflection_planes
                .iter()
                .flatten()
                .all(|x| x.is_finite())
        );
        assert!(
            self.reflection_cells
                .iter()
                .flatten()
                .all(|&i| usize::from(i) < self.probes.len())
        );
        for probe in &self.probes {
            assert!(
                probe
                    .origin
                    .iter()
                    .chain(&probe.lighting_sh)
                    .all(|x| x.is_finite())
            );
            assert!(
                probe
                    .probe_volumes
                    .iter()
                    .flatten()
                    .flatten()
                    .all(|x| x.is_finite())
            );
        }
        let mut pending = vec![0];
        let mut visited = vec![false; self.reflection_nodes.len()];
        while let Some(at) = pending.pop() {
            assert!(at < visited.len(), "Original reflection BSP child range");
            if visited[at] {
                continue;
            }
            visited[at] = true;
            let code = usize::from(self.reflection_nodes[at]);
            if code > self.reflection_cells.len() {
                assert!(code - self.reflection_cells.len() - 1 < self.reflection_planes.len());
                let skip = usize::from(self.reflection_nodes[at + 1]);
                assert!(skip >= 2, "Original reflection BSP must advance");
                pending.push(at + 2);
                pending.push(at + skip);
            }
        }
    }

    pub(crate) fn cell_count(&self) -> usize {
        self.reflection_cells.len()
    }

    pub(crate) fn cell(&self, origin: [f32; 3]) -> Option<usize> {
        let mut at = 0;
        loop {
            let code = usize::from(self.reflection_nodes[at]);
            if code <= self.reflection_cells.len() {
                return code.checked_sub(1);
            }
            let plane = self.reflection_planes[code - self.reflection_cells.len() - 1];
            // Preserve the original y, x, z evaluation order and <= split.
            let distance =
                plane[1] * origin[1] + plane[0] * origin[0] + plane[2] * origin[2] - plane[3];
            at += if distance <= 0. {
                usize::from(self.reflection_nodes[at + 1])
            } else {
                2
            };
        }
    }

    pub(crate) fn select(&self, origin: Vec3) -> usize {
        assert!(origin.is_finite());
        let origin = origin.to_array();
        for (index, probe) in self.probes.iter().enumerate().skip(1) {
            if probe.probe_volumes.iter().any(|volume| {
                volume
                    .iter()
                    .all(|p| p[0] * origin[0] + p[1] * origin[1] + p[2] * origin[2] + p[3] >= 1.)
            }) {
                return index;
            }
        }
        let mut selected = 0;
        let mut best = f32::MAX;
        let mut consider = |index: usize| {
            let probe = &self.probes[index];
            let x = origin[0] - probe.origin[0];
            let y = origin[1] - probe.origin[1];
            let z = origin[2] - probe.origin[2];
            let distance = y * y + x * x + z * z;
            if distance < best {
                best = distance;
                selected = index;
            }
        };
        if let Some(cell) = self.cell(origin) {
            for &probe in &self.reflection_cells[cell] {
                consider(usize::from(probe));
            }
        } else {
            for (index, probe) in self.probes.iter().enumerate().skip(1) {
                if probe.probe_volumes.is_empty() {
                    consider(index);
                }
            }
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn world() -> ReflectionWorld {
        let probe = |x| ReflectionProbe {
            origin: [x, 0., 0.],
            lighting_sh: [0.; 12],
            reflection_image: String::new(),
            probe_volumes: vec![],
        };
        ReflectionWorld {
            probes: vec![probe(0.), probe(-100.), probe(100.), probe(10.)],
            reflection_cells: vec![vec![1], vec![2]],
            reflection_planes: vec![[1., 0., 0., 0.]],
            reflection_nodes: vec![3, 3, 1, 2],
        }
    }
    #[test]
    fn cell_candidates_override_global_distance_and_plane_zero_uses_back() {
        let w = world();
        w.validate();
        assert_eq!(w.select(Vec3::new(1., 0., 0.)), 1);
        assert_eq!(w.select(Vec3::ZERO), 2);
    }
    #[test]
    fn explicit_volume_precedes_cell_candidates() {
        let mut w = world();
        w.probes[3].probe_volumes.push([
            [1., 0., 0., -8.],
            [-1., 0., 0., 12.],
            [0., 1., 0., 2.],
            [0., -1., 0., 2.],
            [0., 0., 1., 2.],
            [0., 0., -1., 2.],
        ]);
        w.validate();
        assert_eq!(w.select(Vec3::new(9., 0., 0.)), 3);
        assert_eq!(w.select(Vec3::new(8.9, 0., 0.)), 1);
    }
    #[test]
    fn outside_world_excludes_volume_probes_and_preserves_nearest_ties() {
        let mut w = world();
        w.reflection_nodes = vec![0];
        w.probes[3].probe_volumes.push([[0.; 4]; 6]);
        w.validate();
        assert_eq!(w.select(Vec3::ZERO), 1);
    }
    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_probe_bsp_is_valid_and_all_probe_origins_resolve() {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets/world.json"),
        )
        .unwrap();
        let world: ReflectionWorld = serde_json::from_slice(&bytes).unwrap();
        world.validate();
        for probe in &world.probes {
            assert!(world.select(Vec3::from_array(probe.origin)) < world.probes.len());
        }
        println!(
            "Validated {} probe definitions, {} cells, {} BSP words",
            world.probes.len(),
            world.reflection_cells.len(),
            world.reflection_nodes.len()
        );
    }
}
