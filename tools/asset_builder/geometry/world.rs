use super::*;

pub fn world(catalog: &Catalog<'_>, zone: usize, root: Address) -> Result<Vec<Surface>> {
    let streams = &catalog.zones[zone].1.streams;
    let draw = root.add(396)?;
    let size = streams.u32(draw.add(32)?)? as usize;
    let vertices = streams.read(pointer(streams, draw.add(36)?)?, size)?;
    let indices = streams.read(
        pointer(streams, draw.add(60)?)?,
        streams.u32(draw.add(56)?)? as usize * 2,
    )?;
    let surfaces = pointer(streams, root.add(872)?)?;
    let count = streams.u32(root.add(16)?)? as usize;
    let secondary_size = streams.u32(draw.add(44)?)? as usize;
    let secondary = if secondary_size == 0 {
        &[]
    } else {
        streams.read(pointer(streams, draw.add(48)?)?, secondary_size)?
    };
    let mut groups = std::collections::BTreeMap::<usize, (usize, usize)>::new();
    let mut covered = vec![false; indices.len() / 2];
    for index in 0..count {
        let surface = surfaces.add(index * 80)?;
        let offset = streams.u32(surface.add(12)?)? as usize;
        let secondary_offset = streams.u32(surface.add(28)?)? as usize;
        let first = streams.u32(surface.add(44)?)? as usize;
        let index_count = streams.u16(surface.add(42)?)? as usize * 3;
        let bytes = indices
            .get(first * 2..(first + index_count) * 2)
            .ok_or("World index span exceeds buffer")?;
        for visited in covered
            .get_mut(first..first + index_count)
            .ok_or("World index span exceeds buffer")?
        {
            if std::mem::replace(visited, true) {
                return Err("Overlapping world surface index spans".into());
            }
        }
        let span = bytes
            .chunks_exact(2)
            .map(|v| u16_at(v, 0) as usize + 1)
            .max()
            .unwrap_or(0);
        let group = groups.entry(offset).or_insert((secondary_offset, 0));
        if group.0 != secondary_offset {
            return Err("World primary group has inconsistent secondary offset".into());
        }
        group.1 = group.1.max(span);
    }
    if covered.iter().any(|visited| !visited) {
        return Err("World has unreferenced index-buffer entries".into());
    }
    let offsets: Vec<_> = groups.keys().copied().collect();
    let mut auxiliary_stride = std::collections::BTreeMap::new();
    let mut group_vertices = 0;
    for (index, &offset) in offsets.iter().enumerate() {
        let (secondary_offset, span) = groups[&offset];
        if span == 0 {
            return Err("Empty native world vertex group".into());
        }
        group_vertices += span;
        let next_primary = offsets.get(index + 1).copied().unwrap_or(vertices.len());
        if (offset + span * 36).next_multiple_of(16) != next_primary {
            return Err("Native world vertex groups overlap or have unexpected padding".into());
        }
        let next_secondary = offsets
            .get(index + 1)
            .map(|next| groups[next].0)
            .unwrap_or(secondary.len());
        let bytes = next_secondary
            .checked_sub(secondary_offset)
            .ok_or("World secondary offsets regress")?;
        if bytes % span != 0 || !matches!(bytes / span, 0 | 4 | 8 | 12 | 16) {
            return Err("Unknown native world auxiliary vertex stride".into());
        }
        auxiliary_stride.insert(offset, bytes / span);
    }
    if group_vertices != streams.u32(draw.add(28)?)? as usize {
        return Err("World shared groups do not cover native vertex count".into());
    }
    println!(
        "Validated {} shared world vertex groups: {group_vertices} primary vertices, {} auxiliary bytes",
        groups.len(),
        secondary.len()
    );
    let mut meshes = Vec::new();
    let mut total_indices = 0;
    for index in 0..count {
        let surface = surfaces.add(index * 80)?;
        let offset = streams.u32(surface.add(12)?)? as usize;
        let native_count = streams.u16(surface.add(40)?)? as usize;
        let triangles = streams.u16(surface.add(42)?)? as usize;
        let first = streams.u32(surface.add(44)?)? as usize;
        total_indices += triangles * 3;
        if triangles == 0 {
            continue;
        }
        let triangle_bytes = indices
            .get(first * 2..first * 2 + triangles * 6)
            .ok_or("World surface exceeds native index buffer")?;
        let referenced = triangle_bytes
            .chunks_exact(2)
            .map(|bytes| u16_at(bytes, 0) as usize + 1)
            .max()
            .ok_or("Empty world triangle span")?;
        let vertex_count = if native_count == 0 {
            referenced
        } else {
            native_count
        };
        if vertex_count < referenced {
            return Err("World indices exceed declared vertex group".into());
        }
        let material = catalog.pointer(zone, surface.add(48)?, 6)?.0;
        let mut mesh = Surface::read(
            format!("world_surface{index}"),
            index,
            vertices
                .get(offset..offset + vertex_count * 36)
                .ok_or("World surface exceeds native vertex buffer")?,
            36,
            triangle_bytes,
            material,
        )?;
        let stride = auxiliary_stride[&offset];
        if stride != 0 {
            let base = groups[&offset].0;
            for &vertex in &mesh.source_vertices {
                let start = base + vertex as usize * stride;
                mesh.auxiliary.push(
                    secondary
                        .get(start..start + stride)
                        .ok_or("World auxiliary vertex exceeds buffer")?
                        .to_vec(),
                );
            }
        }
        meshes.push(mesh);
    }
    if total_indices != indices.len() / 2 {
        return Err("World surface index spans do not cover the native index buffer".into());
    }
    Ok(meshes)
}
