use super::*;

pub(super) fn brushes_in_node(
    streams: &Streams,
    info: Address,
    first: usize,
) -> Result<BTreeSet<usize>> {
    let count = streams.u32(info.add(24)?)? as usize;
    let nodes = pointer(streams, info.add(28)?)?;
    let mut pending = vec![first];
    let mut seen = BTreeSet::new();
    let mut brushes = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if index >= count || !seen.insert(index) {
            return Err("Invalid or cyclic native collision brush tree".into());
        }
        let node = nodes.add(index * 20)?;
        let leaves = streams.u16(node.add(2)?)? as i16;
        if leaves > 0 {
            let array = pointer(streams, node.add(8)?)?;
            for i in 0..leaves as usize {
                brushes.insert(streams.u16(array.add(i * 2)?)? as usize);
            }
        } else {
            for offset in [16, 18] {
                let child = streams.u16(node.add(offset)?)? as usize;
                if child != 0 {
                    pending.push(index + child);
                }
            }
        }
    }
    Ok(brushes)
}

pub(super) fn brush_geometry(
    streams: &Streams,
    root: Address,
) -> Result<(Vec<Vector>, Vec<(Vector, f64, u32)>)> {
    let minimum = vector(streams, root)?;
    let maximum = vector(streams, root.add(16)?)?;
    let mut planes = Vec::new();
    for side in 0..2 {
        for axis in 0..3 {
            let sign = if side == 0 { -1. } else { 1. };
            let mut normal = [0.; 3];
            normal[axis] = sign;
            let distance = if side == 0 {
                -minimum[axis]
            } else {
                maximum[axis]
            };
            planes.push((
                normal,
                distance,
                streams.u32(root.add(60 + (side * 3 + axis) * 4)?)?,
            ));
        }
    }
    let count = streams.u32(root.add(28)?)? as usize;
    if count != 0 {
        let sides = pointer(streams, root.add(32)?)?;
        for index in 0..count {
            let side = sides.add(index * 12)?;
            let plane = pointer(streams, side)?;
            planes.push((
                vector(streams, plane)?,
                scalar(streams, plane.add(12)?)?,
                streams.u32(side.add(8)?)?,
            ));
        }
    }
    let mut vertices = Vec::new();
    let count = streams.u32(root.add(84)?)? as usize;
    if count != 0 {
        let array = pointer(streams, root.add(88)?)?;
        for index in 0..count {
            vertices.push(vector(streams, array.add(index * 12)?)?);
        }
    } else {
        for a in 0..planes.len() {
            for b in a + 1..planes.len() {
                for c in b + 1..planes.len() {
                    let Some(point) = solve(
                        [planes[a].0, planes[b].0, planes[c].0],
                        [planes[a].1, planes[b].1, planes[c].1],
                    ) else {
                        continue;
                    };
                    if planes
                        .iter()
                        .all(|(normal, distance, _)| dot(point, *normal) <= distance + 0.002)
                        && vertices.iter().all(|other| {
                            dot(subtract(point, *other), subtract(point, *other)) > 1e-10
                        })
                    {
                        vertices.push(point);
                    }
                }
            }
        }
    }
    if vertices.len() < 4 {
        return Err("Native solid collision brush has fewer than four vertices".into());
    }
    Ok((vertices, planes))
}

pub(super) fn brush(streams: &Streams, root: Address, batches: &mut Batches) -> Result<()> {
    let (vertices, planes) = brush_geometry(streams, root)?;
    for (normal, distance, flag) in planes {
        let mut face: Vec<_> = vertices
            .iter()
            .copied()
            .filter(|point| (dot(*point, normal) - distance).abs() < 0.01)
            .collect();
        if face.len() < 3 {
            continue;
        }
        let center: Vector = std::array::from_fn(|axis| {
            face.iter().map(|point| point[axis]).sum::<f64>() / face.len() as f64
        });
        let first = subtract(face[0], center);
        let second = cross(normal, first);
        face.sort_by(|a, b| {
            dot(subtract(*a, center), second)
                .atan2(dot(subtract(*a, center), first))
                .total_cmp(
                    &dot(subtract(*b, center), second).atan2(dot(subtract(*b, center), first)),
                )
        });
        for index in 1..face.len() - 1 {
            batches
                .entry(flag)
                .or_default()
                .push([face[0], face[index + 1], face[index]]);
        }
    }
    Ok(())
}

pub(super) fn model(catalog: &Catalog<'_>, zone: usize, root: Address) -> Result<Batches> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u32(root.add(156)?)? as usize;
    let mut batches = Batches::new();
    if count == 0 {
        return Ok(batches);
    }
    let surfaces = pointer(streams, root.add(152)?)?;
    for index in 0..count {
        let surface = surfaces.add(index * 44)?;
        if streams.u32(surface.add(36)?)? & 0x10001 == 0 {
            continue;
        }
        let flags = streams.u32(surface.add(40)?)?;
        let count = streams.u32(surface.add(4)?)? as usize;
        if count == 0 {
            continue;
        }
        let triangles = pointer(streams, surface)?;
        for index in 0..count {
            let triangle = triangles.add(index * 48)?;
            let planes = [triangle, triangle.add(16)?, triangle.add(32)?];
            let matrix = [
                vector(streams, planes[0])?,
                vector(streams, planes[1])?,
                vector(streams, planes[2])?,
            ];
            let distances = [
                scalar(streams, planes[0].add(12)?)?,
                scalar(streams, planes[1].add(12)?)?,
                scalar(streams, planes[2].add(12)?)?,
            ];
            let a = solve(matrix, distances).ok_or("Degenerate native model collision triangle")?;
            let b = solve(matrix, [distances[0], distances[1] + 1., distances[2]])
                .ok_or("Degenerate native model collision triangle")?;
            let c = solve(matrix, [distances[0], distances[1], distances[2] + 1.])
                .ok_or("Degenerate native model collision triangle")?;
            batches.entry(flags).or_default().push([a, c, b]);
        }
    }
    Ok(batches)
}
