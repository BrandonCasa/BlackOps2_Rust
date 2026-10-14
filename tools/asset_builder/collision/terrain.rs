use super::*;

pub(super) fn terrain_partitions(
    streams: &Streams,
    root: Address,
    first: usize,
    count: usize,
) -> Result<BTreeSet<(usize, usize)>> {
    if count == 0 {
        return Ok(BTreeSet::new());
    }
    let tree_count = streams.u32(root.add(136)?)? as usize;
    if first.checked_add(count).is_none_or(|end| end > tree_count) {
        return Err("Native collision terrain root range exceeds AABB trees".into());
    }
    let trees = pointer(streams, root.add(140)?)?;
    let partition_count = streams.u32(root.add(128)?)? as usize;
    let mut result = BTreeSet::new();
    for index in first..first + count {
        let material = streams.u16(trees.add(index * 32 + 12)?)? as usize;
        let mut pending = vec![(index, false)];
        let mut active = BTreeSet::new();
        let mut complete = BTreeSet::new();
        while let Some((index, leaving)) = pending.pop() {
            if leaving {
                active.remove(&index);
                complete.insert(index);
                continue;
            }
            if complete.contains(&index) {
                continue;
            }
            if index >= tree_count || !active.insert(index) {
                return Err("Invalid or cyclic native collision AABB tree".into());
            }
            let tree = trees.add(index * 32)?;
            let children = streams.u16(tree.add(14)?)? as usize;
            let target = streams.u32(tree.add(28)?)? as usize;
            pending.push((index, true));
            if children == 0 {
                if target >= partition_count {
                    return Err("Native collision AABB leaf exceeds partitions".into());
                }
                result.insert((target, material));
            } else {
                let end = target
                    .checked_add(children)
                    .filter(|end| *end <= tree_count)
                    .ok_or("Native collision AABB child range exceeds trees")?;
                for child in (target..end).rev() {
                    pending.push((child, false));
                }
            }
        }
    }
    Ok(result)
}

pub(super) fn triangle_shape(
    streams: &Streams,
    root: Address,
    info: Address,
    partition: usize,
    material: usize,
) -> Result<(Value, BTreeSet<usize>)> {
    let material_count = streams.u32(info.add(8)?)? as usize;
    if material >= material_count {
        return Err("Native collision terrain material exceeds ClipInfo materials".into());
    }
    let material = pointer(streams, info.add(12)?)?.add(material * 12)?;
    let surface_flags = streams.u32(material.add(4)?)?;
    let contents = streams.u32(material.add(8)?)?;
    let entry = pointer(streams, root.add(132)?)?.add(partition * 16)?;
    let count = streams.u8(entry)? as usize;
    let first = streams.u32(entry.add(4)?)? as usize;
    let triangle_count = streams.u32(root.add(116)?)? as usize;
    let end = first
        .checked_add(count)
        .filter(|end| *end <= triangle_count)
        .ok_or("Native collision partition triangle range exceeds triangles")?;
    let vertex_count = streams.u32(root.add(108)?)? as usize;
    let source_vertices = pointer(streams, root.add(112)?)?;
    let source_triangles = pointer(streams, root.add(120)?)?;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut remap = BTreeMap::new();
    let mut owned = BTreeSet::new();
    for index in first..end {
        let mut triangle = [0; 3];
        for (corner, offset) in [4, 2, 0].into_iter().enumerate() {
            let vertex = streams.u16(source_triangles.add(index * 6 + offset)?)? as usize;
            if vertex >= vertex_count {
                return Err("Native collision partition triangle exceeds terrain vertices".into());
            }
            let next = u32::try_from(vertices.len())?;
            triangle[corner] = *remap.entry(vertex).or_insert_with(|| next);
            if triangle[corner] == next {
                let [x, y, z] = vector(streams, source_vertices.add(vertex * 12)?)?;
                vertices.push([x * 0.0254, z * 0.0254, -y * 0.0254]);
            }
        }
        indices.push(triangle);
        owned.insert(index);
    }
    Ok((
        json!({"native_partition":partition,"contents":contents,"surface_flags":surface_flags,"vertices":vertices,"indices":indices}),
        owned,
    ))
}

pub(super) fn inactive_terrain_triangles(
    streams: &Streams,
    root: Address,
) -> Result<BTreeSet<usize>> {
    let partition_count = streams.u32(root.add(128)?)? as usize;
    if partition_count == 0 {
        return Ok(BTreeSet::new());
    }
    let partitions = pointer(streams, root.add(132)?)?;
    let tree_count = streams.u32(root.add(136)?)? as usize;
    let mut referenced = BTreeSet::new();
    if tree_count != 0 {
        let trees = pointer(streams, root.add(140)?)?;
        for index in 0..tree_count {
            let tree = trees.add(index * 32)?;
            if streams.u16(tree.add(14)?)? == 0 {
                let partition = streams.u32(tree.add(28)?)? as usize;
                if partition >= partition_count {
                    return Err("Native AABB leaf references an invalid terrain partition".into());
                }
                referenced.insert(partition);
            }
        }
    }
    let triangle_count = streams.u32(root.add(116)?)? as usize;
    let mut inactive = BTreeSet::new();
    let mut active = BTreeSet::new();
    for index in 0..partition_count {
        let entry = partitions.add(index * 16)?;
        let first = streams.u32(entry.add(4)?)? as usize;
        let count = streams.u8(entry)? as usize;
        if count == 0 {
            continue;
        }
        let end = first
            .checked_add(count)
            .filter(|end| *end <= triangle_count)
            .ok_or("Native terrain partition storage exceeds triangle range")?;
        if referenced.contains(&index) {
            active.extend(first..end);
        } else {
            inactive.extend(first..end);
        }
    }
    inactive.retain(|triangle| !active.contains(triangle));
    Ok(inactive)
}
