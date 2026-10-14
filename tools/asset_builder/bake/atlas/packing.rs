use super::super::*;

pub(in crate::bake) const GUTTER: usize = 8;

pub(in crate::bake) struct ChartTriangle {
    pub(in crate::bake) surface: usize,
    pub(in crate::bake) first: usize,
    pub(in crate::bake) points: [[f64; 2]; 3],
    pub(in crate::bake) metric: [f64; 3],
    pub(in crate::bake) copies: Vec<(usize, usize, [usize; 3])>,
}

pub(in crate::bake) struct Chart {
    pub(in crate::bake) triangles: Vec<ChartTriangle>,
    pub(in crate::bake) width: usize,
    pub(in crate::bake) height: usize,
    pub(in crate::bake) x: usize,
    pub(in crate::bake) y: usize,
}

pub(in crate::bake) struct AtlasPage {
    pub(in crate::bake) charts: Vec<Chart>,
    pub(in crate::bake) rows: Vec<(usize, usize, usize)>,
    pub(in crate::bake) width: usize,
    pub(in crate::bake) height: usize,
}

pub(in crate::bake) fn place_pages(mut charts: Vec<Chart>) -> Result<Vec<AtlasPage>> {
    for chart in &mut charts {
        let limit = (8192 - 2 * GUTTER) as f64;
        if chart.width as f64 <= limit && chart.height as f64 <= limit {
            continue;
        }
        let points: Vec<_> = chart
            .triangles
            .iter()
            .flat_map(|triangle| triangle.points)
            .collect();
        let quarter = std::f64::consts::FRAC_PI_2;
        let mut angles = vec![0., quarter];
        for (index, a) in points.iter().enumerate() {
            for b in &points[index + 1..] {
                let (x, y) = (b[0] - a[0], b[1] - a[1]);
                let length = x.hypot(y);
                if length <= limit {
                    continue;
                }
                let phase = y.atan2(x);
                let boundary = (limit / length).acos();
                for angle in [
                    boundary - phase,
                    -boundary - phase,
                    quarter + boundary - phase,
                    quarter - boundary - phase,
                ] {
                    angles.push(angle.rem_euclid(quarter));
                }
            }
        }
        angles.sort_by(f64::total_cmp);
        angles.dedup();
        let candidates: Vec<_> = angles
            .windows(2)
            .map(|pair| (pair[0] + pair[1]) * 0.5)
            .chain(angles.iter().copied())
            .collect();
        for angle in candidates {
            let (sin, cos) = angle.sin_cos();
            let rotate = |point: [f64; 2]| {
                [
                    point[0] * cos - point[1] * sin,
                    point[0] * sin + point[1] * cos,
                ]
            };
            let lower: [f64; 2] = std::array::from_fn(|axis| {
                points
                    .iter()
                    .map(|&point| rotate(point)[axis])
                    .fold(f64::INFINITY, f64::min)
            });
            let upper: [f64; 2] = std::array::from_fn(|axis| {
                points
                    .iter()
                    .map(|&point| rotate(point)[axis])
                    .fold(f64::NEG_INFINITY, f64::max)
            });
            let width = (upper[0] - lower[0]).ceil() as usize;
            let height = (upper[1] - lower[1]).ceil() as usize;
            if width + 2 * GUTTER > 8192 || height + 2 * GUTTER > 8192 {
                continue;
            }
            for triangle in &mut chart.triangles {
                for point in &mut triangle.points {
                    *point = std::array::from_fn(|axis| rotate(*point)[axis] - lower[axis]);
                }
            }
            println!(
                "Rotated oversized source chart {}x{} into {width}x{height} without changing source triangles or directional footprints",
                chart.width, chart.height
            );
            chart.width = width.max(2);
            chart.height = height.max(2);
            break;
        }
    }
    charts.sort_by_key(|chart| {
        (
            std::cmp::Reverse(chart.height),
            std::cmp::Reverse(chart.width),
        )
    });
    let mut pages = Vec::<AtlasPage>::new();
    for mut chart in charts {
        let (width, height) = (chart.width + 2 * GUTTER, chart.height + 2 * GUTTER);
        if width > 8192 || height > 8192 {
            let triangle = &chart.triangles[0];
            return Err(format!("Bake chart requires {width}x{height}, beyond an 8192 page: source surface {}, triangle {}; source-preserving subdivision is required",triangle.surface,triangle.first/3).into());
        }
        let surface = chart.triangles[0].surface;
        let mut best = None;
        for (page_index, page) in pages.iter().enumerate() {
            let shared = page.charts.iter().any(|chart| {
                chart
                    .triangles
                    .iter()
                    .any(|triangle| triangle.surface == surface)
            });
            for (row_index, &(_, row_height, x)) in page.rows.iter().enumerate() {
                if height <= row_height && x + width <= 8192 {
                    let score = (
                        (row_height - height) * width,
                        !shared,
                        8192 - x - width,
                        page_index,
                        row_index,
                    );
                    if best.is_none_or(|current| score < current) {
                        best = Some(score);
                    }
                }
            }
        }
        let (page_index, row_index) = if let Some((_, _, _, page, row)) = best {
            (page, row)
        } else {
            let available = pages
                .iter()
                .enumerate()
                .filter_map(|(index, page)| {
                    let y = page.rows.last().map_or(0, |&(y, height, _)| y + height);
                    let shared = page.charts.iter().any(|chart| {
                        chart
                            .triangles
                            .iter()
                            .any(|triangle| triangle.surface == surface)
                    });
                    (y + height <= 8192).then(|| (!shared, 8192 - y - height, index, y))
                })
                .min();
            let (page, y) = if let Some((_, _, page, y)) = available {
                (page, y)
            } else {
                pages.push(AtlasPage {
                    charts: Vec::new(),
                    rows: Vec::new(),
                    width: 0,
                    height: 0,
                });
                (pages.len() - 1, 0)
            };
            pages[page].rows.push((y, height, 0));
            (page, pages[page].rows.len() - 1)
        };
        let page = &mut pages[page_index];
        let (y, _, x) = &mut page.rows[row_index];
        chart.x = *x;
        chart.y = *y;
        *x += width;
        page.width = page.width.max(*x);
        page.height = page.height.max(*y + height);
        page.charts.push(chart);
    }
    for page in &mut pages {
        page.width = page.width.max(4).next_power_of_two();
        page.height = page.height.max(4).next_power_of_two();
        page.rows.clear();
        for chart in &page.charts {
            if chart.x + chart.width + 2 * GUTTER > page.width
                || chart.y + chart.height + 2 * GUTTER > page.height
            {
                return Err("Atlas chart placement exceeds its page".into());
            }
        }
    }
    Ok(pages)
}

pub(in crate::bake) fn overlaps(left: [[f64; 2]; 3], right: [[f64; 2]; 3]) -> bool {
    for triangle in [left, right] {
        for edge in 0..3 {
            let axis = [
                triangle[(edge + 1) % 3][1] - triangle[edge][1],
                triangle[edge][0] - triangle[(edge + 1) % 3][0],
            ];
            let range =
                |points: [[f64; 2]; 3]| points.map(|point| point[0] * axis[0] + point[1] * axis[1]);
            let a = range(left);
            let b = range(right);
            if a.into_iter().fold(f64::NEG_INFINITY, f64::max)
                <= b.into_iter().fold(f64::INFINITY, f64::min)
                || b.into_iter().fold(f64::NEG_INFINITY, f64::max)
                    <= a.into_iter().fold(f64::INFINITY, f64::min)
            {
                return false;
            }
        }
    }
    true
}

pub(in crate::bake) fn triangle_weights(
    vertices: [[f64; 2]; 3],
    point: [f64; 2],
) -> Result<([f64; 3], f64)> {
    let a = [
        vertices[1][0] - vertices[0][0],
        vertices[1][1] - vertices[0][1],
    ];
    let b = [
        vertices[2][0] - vertices[0][0],
        vertices[2][1] - vertices[0][1],
    ];
    let p = [point[0] - vertices[0][0], point[1] - vertices[0][1]];
    let determinant = a[0] * b[1] - a[1] * b[0];
    if determinant == 0. || !determinant.is_finite() {
        return Err("Degenerate bake triangle".into());
    }
    let s = (p[0] * b[1] - p[1] * b[0]) / determinant;
    let t = (a[0] * p[1] - a[1] * p[0]) / determinant;
    let weights = [1. - s - t, s, t];
    if weights.iter().all(|&value| value >= 0.) {
        return Ok((weights, 0.));
    }
    let mut best = ([0.; 3], f64::INFINITY);
    for edge in 0..3 {
        let next = (edge + 1) % 3;
        let d = [
            vertices[next][0] - vertices[edge][0],
            vertices[next][1] - vertices[edge][1],
        ];
        let q = [point[0] - vertices[edge][0], point[1] - vertices[edge][1]];
        let length = d[0] * d[0] + d[1] * d[1];
        let along = if length > 0. {
            ((q[0] * d[0] + q[1] * d[1]) / length).clamp(0., 1.)
        } else {
            0.
        };
        let distance = (q[0] - along * d[0]).powi(2) + (q[1] - along * d[1]).powi(2);
        if distance < best.1 {
            let mut weights = [0.; 3];
            weights[edge] = 1. - along;
            weights[next] = along;
            best = (weights, distance);
        }
    }
    Ok(best)
}

pub(in crate::bake) fn merge_islands(
    charts: Vec<Chart>,
    surfaces: &[Surface],
) -> Result<Vec<Chart>> {
    let source_triangles: usize = charts
        .iter()
        .flat_map(|chart| &chart.triangles)
        .map(|triangle| triangle.copies.len())
        .sum();
    let mut inputs = Vec::new();
    let mut edges = BTreeMap::<(Vec<u32>, Vec<u32>), Vec<usize>>::new();
    for (index, chart) in charts.iter().enumerate() {
        let triangle = &chart.triangles[0];
        let surface = &surfaces[triangle.surface];
        let uv: [[f64; 2]; 3] =
            std::array::from_fn(|i| surface.uv[triangle.first + i].map(f64::from));
        let a = [uv[1][0] - uv[0][0], uv[1][1] - uv[0][1]];
        let b = [uv[2][0] - uv[0][0], uv[2][1] - uv[0][1]];
        let determinant = a[0] * b[1] - a[1] * b[0];
        let size = a.into_iter().chain(b).map(f64::abs).fold(0., f64::max);
        if determinant.abs() <= size * size * 1e-12 {
            inputs.push(None);
            continue;
        }
        let x = [b[1] / determinant, -a[1] / determinant];
        let y = [-b[0] / determinant, a[0] / determinant];
        let metric = [
            quadratic(triangle.metric, x, x),
            quadratic(triangle.metric, x, y),
            quadratic(triangle.metric, y, y),
        ];
        let scale = (0.5
            * (metric[0] + metric[2] + (metric[0] - metric[2]).hypot(2. * metric[1]))
            * (1. + 1e-10))
            .sqrt();
        if !scale.is_finite() || !dominates([scale * scale, 0., scale * scale], metric) {
            return Err("Island transform reduces a source footprint".into());
        }
        inputs.push(Some((uv, scale)));
        let keys: [Vec<u32>; 3] = std::array::from_fn(|i| {
            let vertex = triangle.first + i;
            let mut key = vec![triangle.surface as u32];
            key.extend(surface.positions[vertex].map(f32::to_bits));
            key.extend(surface.uv[vertex].map(f32::to_bits));
            key.extend(surface.normals[vertex].map(f32::to_bits));
            key.extend(surface.tangents[vertex].map(f32::to_bits));
            key.extend(surface.colors[vertex].map(u32::from));
            key.extend(surface.auxiliary[vertex].iter().map(|&value| value as u32));
            key
        });
        for edge in 0..3 {
            let (a, b) = (keys[edge].clone(), keys[(edge + 1) % 3].clone());
            let key = if a < b { (a, b) } else { (b, a) };
            edges.entry(key).or_default().push(index);
        }
    }
    let pairs: Vec<_> = edges
        .values()
        .flat_map(|members| {
            members
                .iter()
                .enumerate()
                .flat_map(move |(i, &a)| members[i + 1..].iter().map(move |&b| (a, b)))
        })
        .collect();
    let mut parents: Vec<_> = (0..charts.len()).collect();
    let root = |mut index: usize, parents: &[usize]| {
        while parents[index] != index {
            index = parents[index];
        }
        index
    };
    let mut groups: Vec<_> = (0..charts.len()).map(|index| vec![index]).collect();
    let mut costs: Vec<_> = charts
        .iter()
        .map(|chart| (chart.width + 2 * GUTTER) * (chart.height + 2 * GUTTER))
        .collect();
    for &(a, b) in &pairs {
        let (mut a, mut b) = (root(a, &parents), root(b, &parents));
        if a == b {
            continue;
        }
        if groups[a].iter().any(|&left| {
            groups[b]
                .iter()
                .any(|&right| overlaps(inputs[left].unwrap().0, inputs[right].unwrap().0))
        }) {
            continue;
        }
        let members: Vec<_> = groups[a].iter().chain(&groups[b]).copied().collect();
        let scale = members
            .iter()
            .map(|&index| inputs[index].unwrap().1)
            .fold(0., f64::max);
        let lower: [f64; 2] = std::array::from_fn(|axis| {
            members
                .iter()
                .flat_map(|&index| inputs[index].unwrap().0)
                .map(|uv| uv[axis])
                .fold(f64::INFINITY, f64::min)
        });
        let upper: [f64; 2] = std::array::from_fn(|axis| {
            members
                .iter()
                .flat_map(|&index| inputs[index].unwrap().0)
                .map(|uv| uv[axis])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let width = ((upper[0] - lower[0]) * scale).ceil() as usize;
        let height = ((upper[1] - lower[1]) * scale).ceil() as usize;
        if width + 2 * GUTTER > 8192 || height + 2 * GUTTER > 8192 {
            continue;
        }
        let cost = (width.max(2) + 2 * GUTTER) * (height.max(2) + 2 * GUTTER);
        if cost > costs[a] + costs[b] {
            continue;
        }
        if groups[a].len() < groups[b].len() {
            std::mem::swap(&mut a, &mut b);
        }
        parents[b] = a;
        let moved = std::mem::take(&mut groups[b]);
        groups[a].extend(moved);
        costs[a] = cost;
    }
    let original_count = charts.len();
    let mut charts: Vec<_> = charts.into_iter().map(Some).collect();
    let mut result = Vec::new();
    for members in groups.into_iter().filter(|members| !members.is_empty()) {
        if members.len() == 1 {
            result.push(charts[members[0]].take().unwrap());
            continue;
        }
        let scale = members
            .iter()
            .map(|&index| inputs[index].unwrap().1)
            .fold(0., f64::max);
        let lower: [f64; 2] = std::array::from_fn(|axis| {
            members
                .iter()
                .flat_map(|&index| inputs[index].unwrap().0)
                .map(|uv| uv[axis])
                .fold(f64::INFINITY, f64::min)
        });
        let mut triangles = Vec::new();
        let mut extent = [0_f64; 2];
        for index in members {
            let mut chart = charts[index].take().unwrap();
            let triangle = &mut chart.triangles[0];
            triangle.points = inputs[index]
                .unwrap()
                .0
                .map(|uv| std::array::from_fn(|axis| (uv[axis] - lower[axis]) * scale));
            for point in triangle.points {
                for axis in 0..2 {
                    extent[axis] = extent[axis].max(point[axis]);
                }
            }
            triangles.extend(chart.triangles);
        }
        result.push(Chart {
            triangles,
            width: (extent[0].ceil() as usize).max(2),
            height: (extent[1].ceil() as usize).max(2),
            x: 0,
            y: 0,
        });
    }
    println!(
        "Merged {original_count} triangle charts into {} continuous nonoverlapping UV islands",
        result.len()
    );
    if result
        .iter()
        .flat_map(|chart| &chart.triangles)
        .map(|triangle| triangle.copies.len())
        .sum::<usize>()
        != source_triangles
    {
        return Err("Island assembly changed the source triangle count".into());
    }
    Ok(result)
}

pub(in crate::bake) fn quadratic(metric: [f64; 3], left: [f64; 2], right: [f64; 2]) -> f64 {
    metric[0] * left[0] * right[0]
        + metric[1] * (left[0] * right[1] + left[1] * right[0])
        + metric[2] * left[1] * right[1]
}

pub(in crate::bake) fn dominates(metric: [f64; 3], source: [f64; 3]) -> bool {
    let [a, b, c] = std::array::from_fn(|i| metric[i] - source[i]);
    let minimum = 0.5 * (a + c - (a - c).hypot(2. * b));
    let tolerance = metric
        .into_iter()
        .chain(source)
        .map(f64::abs)
        .fold(1., f64::max)
        * 1e-10;
    minimum >= -tolerance
}

pub(in crate::bake) fn footprint(coordinates: &[[[f32; 2]; 3]]) -> Result<[f64; 3]> {
    let mut metrics = Vec::new();
    for uv in coordinates {
        let left: [f64; 2] = std::array::from_fn(|axis| uv[1][axis] as f64 - uv[0][axis] as f64);
        let right: [f64; 2] = std::array::from_fn(|axis| uv[2][axis] as f64 - uv[0][axis] as f64);
        let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
        let metric = [dot(left, left), dot(left, right), dot(right, right)];
        if !metric.iter().all(|value| value.is_finite()) {
            return Err("Non-finite material UV footprint".into());
        }
        if !metrics.contains(&metric) {
            metrics.push(metric);
        }
    }
    let mut best: [f64; 3] = std::array::from_fn(|i| metrics.iter().map(|metric| metric[i]).sum());
    let determinant = |metric: [f64; 3]| (metric[0] * metric[2] - metric[1] * metric[1]).max(0.);
    for &base in &metrics {
        let scale = base[0].max(base[2]);
        if base[0] <= 0. || determinant(base) <= scale * scale * 1e-12 {
            continue;
        }
        let root = base[0].sqrt();
        let remainder = (base[2] - base[1] * base[1] / base[0]).sqrt();
        let (p, q, r) = (1. / root, -base[1] / (base[0] * remainder), 1. / remainder);
        let mut factor = 1_f64;
        for &source in &metrics {
            let a = p * p * source[0];
            let b = p * (q * source[0] + r * source[1]);
            let c = q * q * source[0] + 2. * q * r * source[1] + r * r * source[2];
            factor = factor.max(0.5 * (a + c + (a - c).hypot(2. * b)));
        }
        let candidate = base.map(|value| value * factor * (1. + 1e-10));
        if candidate.iter().all(|value| value.is_finite())
            && determinant(candidate) < determinant(best)
            && metrics.iter().all(|&source| dominates(candidate, source))
        {
            best = candidate;
        }
    }
    if !metrics.iter().all(|&source| dominates(best, source)) {
        return Err("Material UV metric reduces a source directional footprint".into());
    }
    Ok(best)
}
