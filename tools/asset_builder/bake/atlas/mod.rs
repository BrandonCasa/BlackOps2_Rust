use super::*;

mod normals;
mod packing;
pub(super) use normals::*;
pub(super) use packing::*;

pub(super) fn periodic_key(
    surface: &Surface,
    first: usize,
    color_only_layer: bool,
) -> Result<Option<Vec<u32>>> {
    if !constant_frame(surface, first) {
        return Ok(None);
    }
    let auxiliary = (0..3)
        .map(|i| auxiliary(surface, first + i, false, color_only_layer))
        .collect::<Result<Vec<_>>>()?;
    if !(1..3).all(|i| {
        surface.colors[first + i][1] == surface.colors[first][1] && auxiliary[i].2 == auxiliary[0].2
    }) {
        return Ok(None);
    }
    let offsets: [[f32; 2]; 3] = std::array::from_fn(|i| {
        std::array::from_fn(|axis| auxiliary[i].0[axis] - surface.uv[first + i][axis])
    });
    if offsets[1] != offsets[0] || offsets[2] != offsets[0] {
        return Ok(None);
    }
    let mut key = vec![surface.colors[first][1] as u32];
    key.extend(auxiliary[0].2.map(f32::to_bits));
    key.extend(offsets[0].map(|value| value.rem_euclid(1.).to_bits()));
    Ok(Some(key))
}

pub(super) fn periodic(
    native: &Value,
    recipe: &Recipe,
    baker: &gpu::Baker,
    key: &[u32],
    uri: &str,
    pack: &mut Pack,
) -> Result<Value> {
    let width = recipe
        .inputs
        .iter()
        .map(|image| image.width)
        .max()
        .ok_or("Periodic material has no inputs")?;
    let height = recipe
        .inputs
        .iter()
        .map(|image| image.height)
        .max()
        .unwrap();
    let mut outputs: [Vec<[f32; 4]>; 4] =
        std::array::from_fn(|_| Vec::with_capacity(width * height));
    let green = key[0] as f32 / 255.;
    let transform: [f32; 4] = std::array::from_fn(|axis| 2. * f32::from_bits(key[axis + 1]) - 1.);
    let offset: [f32; 2] = std::array::from_fn(|axis| f32::from_bits(key[axis + 5]));
    for start in (0..width * height).step_by(262_144) {
        let mut samples = vec![[0.; 20]; (width * height - start).min(262_144)];
        parallel_chunks_mut(&mut samples, 4096, |base, samples| {
            for (index, sample) in samples.iter_mut().enumerate() {
                let index = start + base + index;
                let uv0 = [
                    ((index % width) as f32 + 0.5) / width as f32,
                    ((index / width) as f32 + 0.5) / height as f32,
                ];
                let uv1: [f32; 2] = std::array::from_fn(|axis| uv0[axis] + offset[axis]);
                *sample = [
                    uv0[0],
                    uv0[1],
                    uv1[0],
                    uv1[1],
                    uv0[0],
                    uv0[1],
                    green,
                    0.,
                    transform[0],
                    transform[1],
                    transform[2],
                    transform[3],
                    transform[0],
                    transform[1],
                    transform[2],
                    transform[3],
                    0.,
                    0.,
                    if recipe.alpha_reveal { green } else { 0. },
                    0.,
                ];
            }
            Ok(())
        })?;
        for pixel in baker.sample(&samples)? {
            for (output, pixel) in outputs.iter_mut().zip(pixel) {
                output.push(pixel);
            }
        }
    }
    let material = store(native, uri, width, height, outputs, 20, 0, pack)?;
    println!(
        "Baked shared periodic recipe {uri}: {width}x{height}, source green {}",
        key[0]
    );
    Ok(material)
}

pub(super) fn layered(
    native: &Value,
    surfaces: &[Surface],
    recipe: &Recipe,
    baker: &gpu::Baker,
    uri: &str,
    converted: &mut BTreeMap<String, Value>,
    pack: &mut Pack,
) -> Result<Vec<Surface>> {
    let source_surface = if recipe.alpha_reveal {
        converted
            .get(
                native["name"]
                    .as_str()
                    .ok_or("Alpha reveal material name is missing")?,
            )
            .map(|value| value["extras"]["surface"].clone())
    } else {
        None
    };
    let (decal, layer_specular, inputs) = (recipe.decal, recipe.has_specular(), &recipe.inputs);
    let mut charts = Vec::<Chart>::new();
    let mut shared_charts = BTreeMap::<Vec<u32>, (usize, [usize; 3])>::new();
    let mut uv_area = 0_f64;
    for (surface_index, surface) in surfaces.iter().enumerate() {
        for first in (0..surface.positions.len()).step_by(3) {
            let varying_frame = !constant_frame(surface, first);
            let auxiliary = (0..3)
                .map(|i| recipe.auxiliary(surface, first + i))
                .collect::<Result<Vec<_>>>()?;
            let layer_uv: Vec<_> = auxiliary.iter().map(|value| value.0).collect();
            let decal_uv: Vec<_> = auxiliary.iter().map(|value| value.1).collect();
            let third_uv: Vec<_> = auxiliary.iter().map(|value| value.4).collect();
            let uv_streams = if recipe.third_multiply {
                vec![
                    &surface.uv[first..first + 3],
                    layer_uv.as_slice(),
                    decal_uv.as_slice(),
                    third_uv.as_slice(),
                ]
            } else if decal {
                vec![
                    &surface.uv[first..first + 3],
                    layer_uv.as_slice(),
                    decal_uv.as_slice(),
                ]
            } else {
                vec![&surface.uv[first..first + 3], layer_uv.as_slice()]
            };
            let offsets: Vec<[f32; 2]> = uv_streams
                .iter()
                .enumerate()
                .map(|(stream, uv)| {
                    std::array::from_fn(|axis| {
                        let clamped = inputs.iter().enumerate().any(|(input, image)| {
                            let source = recipe.input_uv(input);
                            source == stream && image.sampler & (32 << axis) != 0
                        });
                        if clamped {
                            0.
                        } else {
                            uv.iter()
                                .map(|value| value[axis])
                                .fold(f32::INFINITY, f32::min)
                                .floor()
                        }
                    })
                })
                .collect();
            let keys: [Vec<u32>; 3] = std::array::from_fn(|i| {
                let mut key = Vec::new();
                for (uv, offset) in uv_streams.iter().zip(&offsets) {
                    for axis in 0..2 {
                        key.push((uv[i][axis] - offset[axis]).to_bits());
                    }
                }
                key.push(surface.colors[first + i][1] as u32);
                if decal {
                    key.push(surface.colors[first + i][2] as u32);
                }
                if recipe.third_multiply {
                    key.push(surface.colors[first + i][3] as u32);
                }
                if recipe.alpha_reveal {
                    key.push(surface.colors[first + i][3] as u32);
                }
                key.extend(auxiliary[i].2.map(f32::to_bits));
                key.extend(auxiliary[i].3.map(f32::to_bits));
                key.push(varying_frame as u32);
                if varying_frame {
                    key.extend(surface.normals[first + i].map(f32::to_bits));
                    key.extend(surface.tangents[first + i].map(f32::to_bits));
                    key.push(surface.tangents[first][3].to_bits());
                }
                key
            });
            let mut canonical = [0, 1, 2];
            canonical.sort_by(|&a, &b| keys[a].cmp(&keys[b]));
            let key: Vec<_> = canonical
                .iter()
                .flat_map(|&i| keys[i].iter().copied())
                .collect();
            if let Some(&(chart, original)) = shared_charts.get(&key) {
                let target: &mut ChartTriangle = &mut charts[chart].triangles[0];
                let order = std::array::from_fn(|vertex| {
                    canonical[original.iter().position(|&value| value == vertex).unwrap()]
                });
                target.copies.push((surface_index, first, order));
                continue;
            }
            let coordinates: Vec<[[f32; 2]; 3]> = inputs
                .iter()
                .enumerate()
                .map(|(input, image)| {
                    let uv = match recipe.input_uv(input) {
                        0 => &surface.uv[first..first + 3],
                        2 => &decal_uv,
                        3 => &third_uv,
                        _ => &layer_uv,
                    };
                    std::array::from_fn(|i| {
                        [
                            uv[i][0] * image.width as f32,
                            uv[i][1] * image.height as f32,
                        ]
                    })
                })
                .collect();
            let metric = footprint(&coordinates)?;
            let basis = [[0., 0.], [1., 0.], [0., 1.]];
            let edge =
                |a: usize, b: usize| std::array::from_fn(|axis| basis[b][axis] - basis[a][axis]);
            let distance = |a: usize, b: usize| quadratic(metric, edge(a, b), edge(a, b)).max(0.);
            let order = [[0, 1, 2], [1, 2, 0], [2, 0, 1]]
                .into_iter()
                .max_by(|a, b| distance(a[0], a[1]).total_cmp(&distance(b[0], b[1])))
                .unwrap();
            let length = distance(order[0], order[1]).sqrt();
            let along = if length > 0. {
                quadratic(metric, edge(order[0], order[1]), edge(order[0], order[2])) / length
            } else {
                0.
            };
            let altitude = (distance(order[0], order[2]) - along * along)
                .max(0.)
                .sqrt();
            for uv in &coordinates {
                uv_area += (((uv[1][0] - uv[0][0]) as f64 * (uv[2][1] - uv[0][1]) as f64
                    - (uv[1][1] - uv[0][1]) as f64 * (uv[2][0] - uv[0][0]) as f64)
                    * 0.5)
                    .abs();
            }
            let width = (length.ceil() as usize).max(2);
            let height = (altitude.ceil() as usize).max(2);
            let apex = if length > 0. {
                along / length * width as f64
            } else {
                0.
            };
            let mut points = [[0.; 2]; 3];
            for (index, point) in [[0., 0.], [width as f64, 0.], [apex, height as f64]]
                .into_iter()
                .enumerate()
            {
                points[order[index]] = point;
            }
            let a: [f64; 2] = std::array::from_fn(|axis| points[1][axis] - points[0][axis]);
            let b: [f64; 2] = std::array::from_fn(|axis| points[2][axis] - points[0][axis]);
            let destination = [
                a[0] * a[0] + a[1] * a[1],
                a[0] * b[0] + a[1] * b[1],
                b[0] * b[0] + b[1] * b[1],
            ];
            if !dominates(destination, metric) {
                return Err("Rounded chart reduces a source footprint".into());
            }
            shared_charts.insert(key, (charts.len(), canonical));
            charts.push(Chart {
                triangles: vec![ChartTriangle {
                    surface: surface_index,
                    first,
                    points,
                    metric: destination,
                    copies: vec![(surface_index, first, [0, 1, 2])],
                }],
                width,
                height,
                x: 0,
                y: 0,
            });
        }
    }
    let charts = merge_islands(charts, surfaces)?;
    let area = charts
        .iter()
        .try_fold(0_usize, |area, chart| {
            area.checked_add((chart.width + 2 * GUTTER).checked_mul(chart.height + 2 * GUTTER)?)
        })
        .ok_or("Layered atlas size overflow")?;
    let largest = charts
        .iter()
        .map(|chart| chart.width + 2 * GUTTER)
        .max()
        .ok_or("Layered material has no triangles")?;
    let core_area: usize = charts.iter().map(|chart| chart.width * chart.height).sum();
    println!(
        "Layered chart allocation {}: {} unique charts for {} triangles, {area} padded texels, {core_area} core texels, {uv_area:.0} input UV triangle texels, longest chart {largest}",
        native["name"],
        charts.len(),
        charts
            .iter()
            .flat_map(|chart| &chart.triangles)
            .map(|triangle| triangle.copies.len())
            .sum::<usize>()
    );
    println!(
        "Verified all source directional footprints; rectangle area needs at least {} 8192-square pages before packing slack",
        area.div_ceil(8192 * 8192)
    );
    let pages = place_pages(charts)?;
    let mut assigned: Vec<_> = surfaces
        .iter()
        .map(|surface| vec![false; surface.indices.len() / 3])
        .collect();
    let mut fragments = Vec::new();
    for page in &pages {
        let mut groups = BTreeMap::<usize, Vec<usize>>::new();
        for triangle in page.charts.iter().flat_map(|chart| &chart.triangles) {
            for &(surface, first, _) in &triangle.copies {
                if first % 3 != 0
                    || std::mem::replace(
                        assigned
                            .get_mut(surface)
                            .and_then(|triangles| triangles.get_mut(first / 3))
                            .ok_or("Page triangle exceeds source surface")?,
                        true,
                    )
                {
                    return Err(
                        "Atlas page assignment duplicates or misaligns a source triangle".into(),
                    );
                }
                groups.entry(surface).or_default().push(first / 3);
            }
        }
        fragments.push(groups);
    }
    if assigned.iter().flatten().any(|&assigned| !assigned) {
        return Err("Atlas page assignment omits source triangles".into());
    }
    let texels: usize = pages.iter().map(|page| page.width * page.height).sum();
    println!(
        "Planned {} bounded atlas pages for {}: {texels} allocated texels, {} source-surface/page mesh parts",
        pages.len(),
        native["name"],
        fragments.iter().map(BTreeMap::len).sum::<usize>()
    );
    let mut remapped = surfaces.to_vec();
    let mut result = Vec::new();
    for (page_index, page) in pages.iter().enumerate() {
        let (width, height) = (page.width, page.height);
        println!(
            "Baking atlas page {}/{}: {width}x{height}, {} islands",
            page_index + 1,
            pages.len(),
            page.charts.len()
        );
        let mut albedo =
            vec![[0., 0., 0., if recipe.alpha_reveal { 0. } else { 1. }]; width * height];
        let mut normals = vec![[0., 0., 1., 1.]; width * height];
        let mut response = if layer_specular || recipe.base_alpha_response {
            vec![[1., 1., 0., 1.]; width * height]
        } else {
            Vec::new()
        };
        let mut specular = if layer_specular {
            vec![[0.04, 0.04, 0.04, 0.]; width * height]
        } else {
            Vec::new()
        };
        let mut basis_errors = [0_f64; 2];
        let mut basis_texels = 0_usize;
        let mut basis_triangles = 0_usize;
        let mut raster = Vec::new();
        for chart in &page.charts {
            for triangle in &chart.triangles {
                let surface = &surfaces[triangle.surface];
                let frame = if constant_frame(surface, triangle.first) {
                    None
                } else {
                    Some(NormalFrame::read(surface, triangle.first)?)
                };
                if frame.is_some() {
                    basis_triangles += triangle.copies.len();
                }
                let auxiliary = (0..3)
                    .map(|i| recipe.auxiliary(surface, triangle.first + i))
                    .collect::<Result<Vec<_>>>()?;
                let lower: [usize; 2] = std::array::from_fn(|axis| {
                    triangle
                        .points
                        .iter()
                        .map(|point| point[axis].floor().max(0.) as usize)
                        .min()
                        .unwrap()
                });
                let upper: [usize; 2] = std::array::from_fn(|axis| {
                    triangle
                        .points
                        .iter()
                        .map(|point| point[axis].ceil().max(0.) as usize)
                        .max()
                        .unwrap()
                        + 2 * GUTTER
                });
                let end = [
                    upper[0].min(chart.width + 2 * GUTTER),
                    upper[1].min(chart.height + 2 * GUTTER),
                ];
                raster.push((chart, triangle, surface, frame, auxiliary, lower, end));
            }
        }
        for band in (0..height).step_by(128) {
            let end_y = (band + 128).min(height);
            let mut rows: Vec<Vec<(usize, usize, [f64; 3], gpu::Sample)>> =
                (band..end_y).step_by(8).map(|_| Vec::new()).collect();
            parallel_chunks_mut(&mut rows, 1, |offset, rows| {
                for (row, jobs) in rows.iter_mut().enumerate() {
                    let y0 = band + (offset + row) * 8;
                    let y1 = (y0 + 8).min(end_y);
                    let mut coverage = vec![f64::INFINITY; (y1 - y0) * width];
                    for (triangle_index, (chart, triangle, surface, _, auxiliary, lower, end)) in
                        raster.iter().enumerate()
                    {
                        let begin = (chart.y + lower[1]).max(y0);
                        let finish = (chart.y + end[1]).min(y1);
                        for y in begin..finish {
                            for x in lower[0]..end[0] {
                                let point = [
                                    x as f64 + 0.5 - GUTTER as f64,
                                    (y - chart.y) as f64 + 0.5 - GUTTER as f64,
                                ];
                                let (weights, distance) = triangle_weights(triangle.points, point)?;
                                if distance > (GUTTER * GUTTER) as f64 {
                                    continue;
                                }
                                let local = (y - y0) * width + chart.x + x;
                                if coverage[local] <= distance {
                                    continue;
                                }
                                coverage[local] = distance;
                                let interpolate = |values: [f32; 3]| {
                                    values
                                        .into_iter()
                                        .zip(weights)
                                        .map(|(value, weight)| value as f64 * weight)
                                        .sum::<f64>() as f32
                                };
                                let uv0: [f32; 2] = std::array::from_fn(|axis| {
                                    interpolate(std::array::from_fn(|i| {
                                        surface.uv[triangle.first + i][axis]
                                    }))
                                });
                                let uv1: [f32; 2] = std::array::from_fn(|axis| {
                                    interpolate(std::array::from_fn(|i| auxiliary[i].0[axis]))
                                });
                                let uv2: [f32; 2] = std::array::from_fn(|axis| {
                                    interpolate(std::array::from_fn(|i| auxiliary[i].1[axis]))
                                });
                                let transform: [f32; 4] = std::array::from_fn(|axis| {
                                    2. * interpolate(std::array::from_fn(|i| auxiliary[i].2[axis]))
                                        - 1.
                                });
                                let second_transform: [f32; 4] = std::array::from_fn(|axis| {
                                    2. * interpolate(std::array::from_fn(|i| auxiliary[i].3[axis]))
                                        - 1.
                                });
                                let third_uv: [f32; 2] = std::array::from_fn(|axis| {
                                    interpolate(std::array::from_fn(|i| auxiliary[i].4[axis]))
                                });
                                let alpha = interpolate(std::array::from_fn(|i| {
                                    surface.colors[triangle.first + i][3] as f32 / 255.
                                }));
                                let green = interpolate(std::array::from_fn(|i| {
                                    surface.colors[triangle.first + i][1] as f32 / 255.
                                }));
                                let blue = interpolate(std::array::from_fn(|i| {
                                    surface.colors[triangle.first + i][2] as f32 / 255.
                                }));
                                jobs.push((
                                    y * width + chart.x + x,
                                    triangle_index,
                                    weights,
                                    [
                                        uv0[0],
                                        uv0[1],
                                        uv1[0],
                                        uv1[1],
                                        uv2[0],
                                        uv2[1],
                                        green,
                                        blue,
                                        transform[0],
                                        transform[1],
                                        transform[2],
                                        transform[3],
                                        second_transform[0],
                                        second_transform[1],
                                        second_transform[2],
                                        second_transform[3],
                                        third_uv[0],
                                        third_uv[1],
                                        alpha,
                                        0.,
                                    ],
                                ));
                            }
                        }
                    }
                }
                Ok(())
            })?;
            let jobs: Vec<_> = rows.into_iter().flatten().collect();
            let samples: Vec<_> = jobs.iter().map(|job| job.3).collect();
            let mut pixels = baker.sample(&samples)?;
            let statistics = std::sync::Mutex::new(([0_f64; 2], 0_usize));
            parallel_chunks_mut(&mut pixels, 4096, |offset, pixels| {
                let (mut maximum, mut count) = ([0_f64; 2], 0_usize);
                for (pixel, job) in pixels.iter_mut().zip(&jobs[offset..]) {
                    if let Some(frame) = &raster[job.1].3 {
                        let (normal, errors) = frame.convert(job.2, pixel[1])?;
                        pixel[1] = normal;
                        for axis in 0..2 {
                            maximum[axis] = maximum[axis].max(errors[axis]);
                        }
                        count += 1;
                    }
                }
                let mut statistics = statistics
                    .lock()
                    .map_err(|_| "Normal conversion worker panicked")?;
                for axis in 0..2 {
                    statistics.0[axis] = statistics.0[axis].max(maximum[axis]);
                }
                statistics.1 += count;
                Ok(())
            })?;
            let (maximum, count) = statistics
                .into_inner()
                .map_err(|_| "Normal conversion worker panicked")?;
            for axis in 0..2 {
                basis_errors[axis] = basis_errors[axis].max(maximum[axis]);
            }
            basis_texels += count;
            for (job, pixel) in jobs.iter().zip(pixels) {
                let index = job.0;
                if layer_specular || recipe.base_alpha_response {
                    response[index] = pixel[2];
                }
                if layer_specular {
                    specular[index] = pixel[3];
                }
                albedo[index] = pixel[0];
                normals[index] = pixel[1];
            }
        }
        for chart in &page.charts {
            for triangle in &chart.triangles {
                for &(surface, first, order) in &triangle.copies {
                    let surface = &mut remapped[surface];
                    for (index, point) in triangle.points.into_iter().enumerate() {
                        if point[0] < -1e-6
                            || point[1] < -1e-6
                            || point[0] > chart.width as f64 + 1e-6
                            || point[1] > chart.height as f64 + 1e-6
                        {
                            return Err("Remapped source vertex exceeds its atlas chart".into());
                        }
                        surface.uv[first + order[index]] = [
                            ((chart.x + GUTTER) as f64 + point[0]) as f32 / width as f32,
                            ((chart.y + GUTTER) as f64 + point[1]) as f32 / height as f32,
                        ];
                    }
                    let sign = surface.tangents[first][3];
                    for index in first..first + 3 {
                        if recipe.alpha_reveal {
                            surface.colors[index][3] = 255;
                        } else {
                            surface.colors[index] = [255; 4];
                        }
                        surface.tangents[index][3] = sign;
                    }
                }
            }
        }
        let identity = format!(
            "{}#atlas{page_index}",
            native["name"]
                .as_str()
                .ok_or("Missing native material identity")?
                .to_ascii_lowercase()
        );
        let mut material = store(
            native,
            &format!("{uri}/{page_index}"),
            width,
            height,
            [albedo, normals, response, specular],
            116,
            page.charts.len(),
            pack,
        )?;
        if recipe.alpha_reveal {
            reveal::finish(source_surface.as_ref(), &mut material)?;
        }
        material["extras"]["surface"]["bake"]["normal_basis"] = json!({"destination":"Solari interpolated normalized N/T; signed cross(N,T)",
        "varying_frame_triangles":basis_triangles,"converted_texel_samples":basis_texels,
        "maximum_precompression_solari_error_degrees":basis_errors[0],"maximum_precompression_raster_error_degrees":basis_errors[1]});
        if basis_triangles > 0 {
            material["extras"]["surface"]["approximations"].as_array_mut().unwrap().push(json!("Native per-vertex bitangents converted into Solari's reconstructed frame per texel; output handedness is constant per triangle. Raster frame differs on varying-frame triangles, with precompression sampled angular error recorded in bake.normal_basis"));
            println!(
                "Converted {basis_triangles} varying-frame triangles ({basis_texels} texel samples); maximum precompression normal error Solari {:.9} degrees, raster {:.6} degrees",
                basis_errors[0], basis_errors[1]
            );
        }
        material["name"] = json!(identity);
        material["extras"]["surface"]["identity"] = json!(identity);
        if page_index == 0 {
            let name = native["name"].as_str().unwrap().to_ascii_lowercase();
            let mut alias = material.clone();
            alias["name"] = json!(name);
            alias["extras"]["surface"]["identity"] = json!(name);
            converted.insert(name, alias);
        }
        converted.insert(identity.clone(), material);
        for (&surface, triangles) in &fragments[page_index] {
            let mut part = remapped[surface].select_triangles(triangles)?;
            part.material = identity.clone();
            result.push(part);
        }
        println!(
            "Baked atlas page {}/{}: {width}x{height}, {} source-surface parts",
            page_index + 1,
            pages.len(),
            fragments[page_index].len()
        );
    }
    for surface in surfaces {
        let count: usize = result
            .iter()
            .filter(|part| part.source_index == surface.source_index)
            .map(|part| part.indices.len())
            .sum();
        if count != surface.indices.len() {
            return Err("Paged bake changed a source surface triangle count".into());
        }
    }
    Ok(result)
}
