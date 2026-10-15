//! Sparse T6 light-grid sampling (0044c900, 0044ccb0, 0044cfd0).
//! Coordinates are original source inches; callers own the model sample origin.
use bevy::prelude::*;
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GridDefinition {
    mins: [u16; 3],
    maxs: [u16; 3],
    row_axis: usize,
    col_axis: usize,
    pub offset: f32,
    sun_primary_light_index: u8,
    raw_row_data_size: usize,
    entry_count: usize,
    pub coeff_count: usize,
    pub coefficients_file: String,
    color_count: usize,
    sky_grid_volume_count: usize,
    row_data_start_file: String,
    raw_row_data_file: String,
    entries_file: String,
    sky_grid_volumes_file: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Entry {
    coefficient: u16,
    light: u8,
    visibility: u8,
}

struct SkyVolume {
    min: Vec3,
    max: Vec3,
    entry: Entry,
}

pub(crate) struct LightGrid {
    definition: GridDefinition,
    rows: Vec<u16>,
    raw: Vec<u8>,
    entries: Vec<Entry>,
    sky: Vec<SkyVolume>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GridSample {
    /// Duplicate corners are merged in their original visitation order.
    pub coefficients: [(u16, f32); 8],
    pub count: usize,
    pub visibility: f32,
    pub primary_light: u8,
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}
fn u32_at(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

impl GridDefinition {
    pub(crate) fn load(self, root: &Path) -> LightGrid {
        assert!(self.row_axis < 2 && self.col_axis < 2 && self.row_axis != self.col_axis);
        assert!(self.offset.is_finite() && self.mins.iter().zip(self.maxs).all(|(&a, b)| a <= b));
        // Origins uses SH coefficients. Legacy color-volume evaluation is a
        // different native path and must not silently reinterpret RGB records.
        assert!(self.coeff_count > 0 && self.color_count == 0);
        let read = |path: &str, size: usize| {
            let bytes = std::fs::read(root.join(path))
                .unwrap_or_else(|e| panic!("Read original light grid {path}: {e}"));
            assert_eq!(bytes.len(), size, "Original light-grid stream {path}");
            bytes
        };
        let rows = read(
            &self.row_data_start_file,
            (usize::from(self.maxs[self.row_axis] - self.mins[self.row_axis]) + 1) * 2,
        )
        .chunks_exact(2)
        .map(|b| u16_at(b, 0))
        .collect();
        let raw = read(&self.raw_row_data_file, self.raw_row_data_size);
        let entry = |b: &[u8]| {
            let e = Entry {
                coefficient: u16_at(b, 0),
                light: b[2],
                visibility: b[3],
            };
            assert!(usize::from(e.coefficient) < self.coeff_count);
            e
        };
        let entries = read(&self.entries_file, self.entry_count * 4)
            .chunks_exact(4)
            .map(entry)
            .collect();
        let sky = read(&self.sky_grid_volumes_file, self.sky_grid_volume_count * 40)
            .chunks_exact(40)
            .map(|b| {
                let vector = |at| {
                    Vec3::from_array(std::array::from_fn(|i| {
                        f32::from_le_bytes(b[at + i * 4..at + i * 4 + 4].try_into().unwrap())
                    }))
                };
                let min = vector(0);
                let max = vector(12);
                assert!(min.is_finite() && max.is_finite() && min.cmple(max).all());
                SkyVolume {
                    min,
                    max,
                    entry: entry(&b[36..40]),
                }
            })
            .collect();
        let grid = LightGrid {
            definition: self,
            rows,
            raw,
            entries,
            sky,
        };
        grid.validate_rows();
        grid
    }
}

impl LightGrid {
    pub(crate) fn origin_offset(&self) -> f32 {
        self.definition.offset
    }

    fn validate_rows(&self) {
        for &row in &self.rows {
            if row == u16::MAX {
                continue;
            }
            let mut at = usize::from(row) * 4;
            let width = u32::from(u16_at(&self.raw, at + 2));
            let height = u16_at(&self.raw, at + 6);
            let mut first = u32_at(&self.raw, at + 8) as usize;
            at += 12;
            let mut covered = 0;
            while covered < width {
                let count = u32::from(self.raw[at]);
                let samples = usize::from(self.raw[at + 1]);
                assert!(
                    count > 0 && covered + count <= width,
                    "Original light-grid run width"
                );
                if samples > 0 {
                    let start = self.height_offset(at, height);
                    assert!(start + samples as u32 <= u32::from(height));
                    assert!(first + count as usize * samples <= self.entries.len());
                }
                covered += count;
                first += count as usize * samples;
                at += self.run_stride(at, height);
            }
        }
    }

    fn height_offset(&self, at: usize, height: u16) -> u32 {
        if height > 255 {
            u32::from(u16_at(&self.raw, at + 2))
        } else {
            u32::from(self.raw[at + 2])
        }
    }
    fn run_stride(&self, at: usize, height: u16) -> usize {
        if self.raw[at + 1] == 0 {
            2
        } else if height > 255 {
            4
        } else {
            3
        }
    }
    fn pair(&self, at: usize, height: u16, first: usize, z: u32) -> [Option<Entry>; 2] {
        let count = u32::from(self.raw[at + 1]);
        if count == 0 {
            return [None; 2];
        }
        let delta = z.wrapping_sub(self.height_offset(at, height));
        std::array::from_fn(|i| {
            let index = delta.wrapping_add(i as u32);
            (index < count).then(|| self.entries[first + index as usize])
        })
    }

    /// Two adjacent columns, with lower/upper Z samples per column. Preserve
    /// unsigned wrap at the lower boundary: -1 admits only the upper corner.
    fn row_corners(&self, cell: [u32; 3], fallback: &mut u16) -> [Option<Entry>; 4] {
        let d = &self.definition;
        let row = cell[d.row_axis].wrapping_sub(u32::from(d.mins[d.row_axis]));
        let Some(&offset) = self.rows.get(row as usize).filter(|&&v| v != u16::MAX) else {
            return [None; 4];
        };
        let header = usize::from(offset) * 4;
        let width = u32::from(u16_at(&self.raw, header + 2));
        let zmin = u32::from(u16_at(&self.raw, header + 4));
        let height = u16_at(&self.raw, header + 6);
        let mut col = cell[d.col_axis].wrapping_sub(u32::from(u16_at(&self.raw, header)));
        let z = cell[2].wrapping_sub(zmin);
        if width < col.wrapping_add(1) {
            return [None; 4];
        }
        if u32::from(height) < z.wrapping_add(1) {
            if cell[2] < zmin {
                *fallback = 0;
            }
            return [None; 4];
        }
        let mut first = u32_at(&self.raw, header + 8) as usize;
        let mut at = header + 12;
        if col == u32::MAX {
            let right = self.pair(at, height, first, z);
            if z < self.height_offset(at, height) {
                *fallback = 0;
            }
            return [None, None, right[0], right[1]];
        }
        while u32::from(self.raw[at]) <= col {
            let columns = u32::from(self.raw[at]);
            first += columns as usize * usize::from(self.raw[at + 1]);
            col -= columns;
            at += self.run_stride(at, height);
        }
        let columns = u32::from(self.raw[at]);
        let samples = usize::from(self.raw[at + 1]);
        let next = at + self.run_stride(at, height);
        if samples == 0 {
            // The native empty-run case inspects the next span when deciding
            // whether a missing sample uses sky entry 1 or indoor entry 0.
            if self.raw[next + 1] != 0
                && z < u32::from(self.raw[next + 1]) + self.height_offset(next, height)
            {
                *fallback = 0;
            }
        } else if z < self.height_offset(at, height) {
            *fallback = 0;
        }
        let left = self.pair(at, height, first + col as usize * samples, z);
        let right = if col + 1 < columns {
            self.pair(at, height, first + (col as usize + 1) * samples, z)
        } else if cell[d.col_axis] + 1 == u32::from(u16_at(&self.raw, header)) + width {
            [None; 2]
        } else {
            self.pair(next, height, first + columns as usize * samples, z)
        };
        [left[0], left[1], right[0], right[1]]
    }

    pub(crate) fn sample(&self, origin: Vec3) -> GridSample {
        assert!(origin.is_finite());
        let source = origin.to_array();
        let mut cell: [u32; 3] = std::array::from_fn(|i| {
            ((source[i].floor() as i32).wrapping_add(131072) >> if i == 2 { 6 } else { 5 }) as u32
        });
        let row = self.definition.row_axis;
        let col = self.definition.col_axis;
        let r = (source[row] - -131072.) * 0.03125 - cell[row] as f32;
        let c = (source[col] - -131072.) * 0.03125 - cell[col] as f32;
        let z = (source[2] - -131072.) * 0.015625 - cell[2] as f32;
        let a = (1. - c) * (1. - z);
        let b = (1. - c) * z;
        let e = (1. - z) * c;
        let weights = [
            (1. - r) * a,
            (1. - r) * b,
            (1. - r) * e,
            (1. - r) * z * c,
            a * r,
            b * r,
            e * r,
            z * c * r,
        ];
        let mut fallback = 1;
        let low = self.row_corners(cell, &mut fallback);
        cell[row] = cell[row].wrapping_add(1);
        let high = self.row_corners(cell, &mut fallback);
        let corners = [
            low[0], low[1], low[2], low[3], high[0], high[1], high[2], high[3],
        ];
        let mut primary = 0;
        let mut best = 0.;
        let mut found = false;
        for (corner, weight) in corners.iter().zip(weights) {
            let Some(entry) = corner.filter(|_| weight >= 0.001) else {
                continue;
            };
            if !found
                || primary == 0
                || (entry.light != 0 && (primary == 255 || (entry.light != 255 && best < weight)))
            {
                primary = entry.light;
                best = weight;
            }
            found = true;
        }
        let sun = self.definition.sun_primary_light_index;
        if primary == 255 {
            primary = sun;
        }
        let mut output = GridSample {
            coefficients: [(0, 0.); 8],
            count: 0,
            visibility: 0.,
            primary_light: primary,
        };
        if !found {
            let mut entry = Entry {
                coefficient: fallback,
                light: sun,
                visibility: 255,
            };
            if let Some(volume) = self
                .sky
                .iter()
                .find(|v| origin.cmpge(v.min).all() && origin.cmple(v.max).all())
            {
                entry = volume.entry;
            }
            output.coefficients[0] = (entry.coefficient, 1.);
            output.count = 1;
            output.primary_light = entry.light;
            output.visibility = f32::from(entry.visibility) * 0.003921569;
            return output;
        }
        let mut total = 0.;
        let mut visible_weight = 0.;
        let mut visible = 0.;
        for (corner, weight) in corners.iter().zip(weights) {
            let Some(entry) = corner.filter(|_| weight >= 0.001) else {
                continue;
            };
            total += weight;
            if entry.light == primary || (primary == sun && entry.light == 255) {
                visible_weight += weight;
                visible += f32::from(entry.visibility) * weight;
            }
            if let Some(existing) = output.coefficients[..output.count]
                .iter_mut()
                .find(|e| e.0 == entry.coefficient)
            {
                existing.1 += weight;
            } else {
                output.coefficients[output.count] = (entry.coefficient, weight);
                output.count += 1;
            }
        }
        output.visibility = visible / (visible_weight * 255.);
        if output.count == 1 {
            output.coefficients[0].1 = 1.;
            return output;
        }
        let scale = 1. / total;
        for coefficient in &mut output.coefficients[..output.count] {
            coefficient.1 *= scale;
        }
        output
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> LightGrid {
        let definition: GridDefinition = serde_json::from_value(serde_json::json!({
            "mins":[4096,4096,2048],"maxs":[4097,4097,2049],"rowAxis":0,"colAxis":1,
            "offset":12.,"sunPrimaryLightIndex":1,"rawRowDataSize":32,"entryCount":8,
            "coeffCount":16,"colorCount":0,"skyGridVolumeCount":0,"rowDataStartFile":"",
            "rawRowDataFile":"","entriesFile":"","skyGridVolumesFile":"","coefficientsFile":""
        }))
        .unwrap();
        let mut raw = Vec::new();
        for first in [0u32, 4] {
            for n in [4096u16, 2, 2048, 2] {
                raw.extend(n.to_le_bytes());
            }
            raw.extend(first.to_le_bytes());
            raw.extend([2, 2, 0, 0]);
        }
        LightGrid {
            definition,
            rows: vec![0, 4],
            raw,
            entries: (0..8)
                .map(|n| Entry {
                    coefficient: n / 2 + 1,
                    light: 1,
                    visibility: 255,
                })
                .collect(),
            sky: vec![],
        }
    }

    #[test]
    fn interpolation_merges_equal_samples_and_keeps_corner_order() {
        let grid = fixture();
        let sample = grid.sample(Vec3::new(16., 16., 32.));
        assert_eq!(sample.count, 4);
        assert_eq!(
            &sample.coefficients[..4],
            &[(1, 0.25), (2, 0.25), (3, 0.25), (4, 0.25)]
        );
        assert_eq!(sample.visibility, 1.);
    }

    #[test]
    fn primary_selection_prefers_authored_light_over_sun_sentinel() {
        let mut grid = fixture();
        grid.entries[0].light = 255;
        grid.entries[4].light = 2;
        grid.entries[4].visibility = 64;
        let sample = grid.sample(Vec3::new(3.2, 0., 0.));
        assert_eq!(sample.primary_light, 2);
        assert!((sample.visibility - 64. / 255.).abs() < 1e-6);
    }

    #[test]
    fn low_weight_corners_are_discarded_before_primary_selection() {
        let mut grid = fixture();
        grid.entries[4].light = 2;
        let sample = grid.sample(Vec3::new(0.016, 0., 0.));
        assert_eq!(sample.count, 1);
        assert_eq!(sample.coefficients[0], (1, 1.));
        assert_eq!(sample.primary_light, 1);
    }

    #[test]
    fn lower_column_and_height_boundaries_keep_upper_corner() {
        let grid = fixture();
        for origin in [Vec3::new(0., -16., 0.), Vec3::new(0., 0., -32.)] {
            let sample = grid.sample(origin);
            assert_eq!(sample.count, 1);
            assert_eq!(sample.coefficients[0], (1, 1.));
        }
    }

    #[test]
    fn sky_volume_fallback_preserves_authored_entry_and_visibility() {
        let mut grid = fixture();
        grid.sky.push(SkyVolume {
            min: Vec3::splat(100.),
            max: Vec3::splat(200.),
            entry: Entry {
                coefficient: 9,
                light: 3,
                visibility: 32,
            },
        });
        let sample = grid.sample(Vec3::splat(150.));
        assert_eq!(sample.coefficients[0], (9, 1.));
        assert_eq!(sample.primary_light, 3);
        assert!((sample.visibility - 32. / 255.).abs() < 1e-6);
    }

    #[test]
    #[ignore = "requires the authorized local Origins export"]
    fn exported_grid_samples_every_authored_entry() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/assets");
        let world: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("world.json")).unwrap()).unwrap();
        let definition: GridDefinition =
            serde_json::from_value(world["lightGrid"].clone()).unwrap();
        let grid = definition.load(&root);
        let mut visited = vec![false; grid.entries.len()];
        for (row_index, &offset) in grid.rows.iter().enumerate() {
            if offset == u16::MAX {
                continue;
            }
            let mut at = usize::from(offset) * 4;
            let mut col = u32::from(u16_at(&grid.raw, at));
            let width = u32::from(u16_at(&grid.raw, at + 2));
            let end = col + width;
            let zmin = u32::from(u16_at(&grid.raw, at + 4));
            let height = u16_at(&grid.raw, at + 6);
            let mut index = u32_at(&grid.raw, at + 8) as usize;
            at += 12;
            while col < end {
                let columns = u32::from(grid.raw[at]);
                let samples = u32::from(grid.raw[at + 1]);
                if samples > 0 {
                    let start = grid.height_offset(at, height);
                    for c in col..col + columns {
                        for z in start..start + samples {
                            let mut point = [0.; 3];
                            point[grid.definition.row_axis] = (row_index as i32
                                + i32::from(grid.definition.mins[grid.definition.row_axis]))
                                as f32
                                * 32.
                                - 131072.;
                            point[grid.definition.col_axis] = c as f32 * 32. - 131072.;
                            point[2] = (zmin + z) as f32 * 64. - 131072.;
                            let actual = grid.sample(Vec3::from_array(point));
                            let between =
                                grid.sample(Vec3::from_array(point) + Vec3::new(8., 8., 16.));
                            assert!(
                                between.visibility.is_finite(),
                                "interpolated visibility at {index}"
                            );
                            let sum: f32 = between.coefficients[..between.count]
                                .iter()
                                .map(|e| e.1)
                                .sum();
                            assert!((sum - 1.).abs() < 1e-5, "interpolated weight at {index}");
                            let expected = grid.entries[index];
                            assert_eq!(actual.count, 1, "entry {index}");
                            assert_eq!(
                                actual.coefficients[0],
                                (expected.coefficient, 1.),
                                "entry {index}"
                            );
                            assert_eq!(
                                actual.primary_light,
                                if expected.light == 255 {
                                    grid.definition.sun_primary_light_index
                                } else {
                                    expected.light
                                }
                            );
                            assert!(
                                (actual.visibility - f32::from(expected.visibility) / 255.).abs()
                                    < 1e-6
                            );
                            assert!(!visited[index]);
                            visited[index] = true;
                            index += 1;
                        }
                    }
                }
                col += columns;
                at += grid.run_stride(at, height);
            }
        }
        assert!(visited.iter().all(|&seen| seen));
        println!("Sampled all {} authored light-grid entries", visited.len());
    }
}
