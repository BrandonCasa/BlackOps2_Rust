use crate::{convert::Catalog, core::*, fastfile::{Address, Streams}, gltf::Scene, pack::Pack};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Vector = [f64; 3];
type Triangle = [Vector; 3];
type Batches = BTreeMap<u32, Vec<Triangle>>;

fn pointer(streams: &Streams, field: Address) -> Result<Address> { Ok(Address::decode(streams.u32(field)?)?) }
fn scalar(streams: &Streams, field: Address) -> Result<f64> {
    let value = f32::from_bits(streams.u32(field)?);
    if !value.is_finite() { return Err("Non-finite collision component".into()); }
    Ok(value as f64)
}
fn vector(streams: &Streams, field: Address) -> Result<Vector> {
    Ok([scalar(streams, field)?, scalar(streams, field.add(4)?)?, scalar(streams, field.add(8)?)?])
}
fn dot(a: Vector, b: Vector) -> f64 { (0..3).map(|i| a[i] * b[i]).sum() }
fn cross(a: Vector, b: Vector) -> Vector { [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]] }
fn subtract(a: Vector, b: Vector) -> Vector { std::array::from_fn(|i| a[i] - b[i]) }
fn solve(matrix: [Vector; 3], rhs: Vector) -> Option<Vector> {
    let determinant = dot(matrix[0], cross(matrix[1], matrix[2]));
    if determinant.abs() < 1e-12 { return None; }
    let a = cross(matrix[1], matrix[2]); let b = cross(matrix[2], matrix[0]); let c = cross(matrix[0], matrix[1]);
    Some(std::array::from_fn(|i| (a[i]*rhs[0] + b[i]*rhs[1] + c[i]*rhs[2]) / determinant))
}

fn brushes_in_node(streams: &Streams, info: Address, first: usize) -> Result<BTreeSet<usize>> {
    let count = streams.u32(info.add(24)?)? as usize;
    let nodes = pointer(streams, info.add(28)?)?;
    let mut pending = vec![first]; let mut seen = BTreeSet::new(); let mut brushes = BTreeSet::new();
    while let Some(index) = pending.pop() {
        if index >= count || !seen.insert(index) { return Err("Invalid or cyclic native collision brush tree".into()); }
        let node = nodes.add(index * 20)?;
        let leaves = streams.u16(node.add(2)?)? as i16;
        if leaves > 0 {
            let array = pointer(streams, node.add(8)?)?;
            for i in 0..leaves as usize { brushes.insert(streams.u16(array.add(i * 2)?)? as usize); }
        } else {
            for offset in [16, 18] {
                let child = streams.u16(node.add(offset)?)? as usize;
                if child != 0 { pending.push(index + child); }
            }
        }
    }
    Ok(brushes)
}

fn brush_geometry(streams: &Streams, root: Address) -> Result<(Vec<Vector>, Vec<(Vector, f64, u32)>)> {
    let minimum = vector(streams, root)?; let maximum = vector(streams, root.add(16)?)?;
    let mut planes = Vec::new();
    for side in 0..2 { for axis in 0..3 {
        let sign = if side == 0 { -1. } else { 1. };
        let mut normal = [0.; 3]; normal[axis] = sign;
        let distance = if side == 0 { -minimum[axis] } else { maximum[axis] };
        planes.push((normal, distance, streams.u32(root.add(60 + (side * 3 + axis) * 4)?)?));
    }}
    let count = streams.u32(root.add(28)?)? as usize;
    if count != 0 {
        let sides = pointer(streams, root.add(32)?)?;
        for index in 0..count {
            let side = sides.add(index * 12)?;
            let plane = pointer(streams, side)?;
            planes.push((vector(streams, plane)?, scalar(streams, plane.add(12)?)?, streams.u32(side.add(8)?)?));
        }
    }
    let mut vertices = Vec::new();
    let count = streams.u32(root.add(84)?)? as usize;
    if count != 0 {
        let array = pointer(streams, root.add(88)?)?;
        for index in 0..count { vertices.push(vector(streams, array.add(index * 12)?)?); }
    } else {
        for a in 0..planes.len() { for b in a + 1..planes.len() { for c in b + 1..planes.len() {
            let Some(point) = solve([planes[a].0, planes[b].0, planes[c].0], [planes[a].1, planes[b].1, planes[c].1]) else { continue; };
            if planes.iter().all(|(normal, distance, _)| dot(point, *normal) <= distance + 0.002)
                && vertices.iter().all(|other| dot(subtract(point, *other), subtract(point, *other)) > 1e-10) {
                vertices.push(point);
            }
        }}}
    }
    if vertices.len() < 4 { return Err("Native solid collision brush has fewer than four vertices".into()); }
    Ok((vertices, planes))
}

fn brush(streams: &Streams, root: Address, batches: &mut Batches) -> Result<()> {
    let (vertices, planes) = brush_geometry(streams, root)?;
    for (normal, distance, flag) in planes {
        let mut face: Vec<_> = vertices.iter().copied().filter(|point| (dot(*point, normal) - distance).abs() < 0.01).collect();
        if face.len() < 3 { continue; }
        let center: Vector = std::array::from_fn(|axis| face.iter().map(|point| point[axis]).sum::<f64>() / face.len() as f64);
        let first = subtract(face[0], center); let second = cross(normal, first);
        face.sort_by(|a, b| dot(subtract(*a, center), second).atan2(dot(subtract(*a, center), first))
            .total_cmp(&dot(subtract(*b, center), second).atan2(dot(subtract(*b, center), first))));
        for index in 1..face.len() - 1 { batches.entry(flag).or_default().push([face[0], face[index + 1], face[index]]); }
    }
    Ok(())
}

fn model(catalog: &Catalog<'_>, zone: usize, root: Address) -> Result<Batches> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u32(root.add(156)?)? as usize;
    let mut batches = Batches::new();
    if count == 0 { return Ok(batches); }
    let surfaces = pointer(streams, root.add(152)?)?;
    for index in 0..count {
        let surface = surfaces.add(index * 44)?;
        if streams.u32(surface.add(36)?)? & 0x10001 == 0 { continue; }
        let flags = streams.u32(surface.add(40)?)?;
        let count = streams.u32(surface.add(4)?)? as usize;
        if count == 0 { continue; }
        let triangles = pointer(streams, surface)?;
        for index in 0..count {
            let triangle = triangles.add(index * 48)?;
            let planes = [triangle, triangle.add(16)?, triangle.add(32)?];
            let matrix = [vector(streams, planes[0])?, vector(streams, planes[1])?, vector(streams, planes[2])?];
            let distances = [scalar(streams, planes[0].add(12)?)?, scalar(streams, planes[1].add(12)?)?, scalar(streams, planes[2].add(12)?)?];
            let a = solve(matrix, distances).ok_or("Degenerate native model collision triangle")?;
            let b = solve(matrix, [distances[0], distances[1] + 1., distances[2]]).ok_or("Degenerate native model collision triangle")?;
            let c = solve(matrix, [distances[0], distances[1], distances[2] + 1.]).ok_or("Degenerate native model collision triangle")?;
            batches.entry(flags).or_default().push([a, c, b]);
        }
    }
    Ok(batches)
}

fn terrain_partitions(streams: &Streams, root: Address, first: usize, count: usize) -> Result<BTreeSet<(usize, usize)>> {
    if count == 0 { return Ok(BTreeSet::new()); }
    let tree_count = streams.u32(root.add(136)?)? as usize;
    if first.checked_add(count).is_none_or(|end| end > tree_count) { return Err("Native collision terrain root range exceeds AABB trees".into()); }
    let trees = pointer(streams, root.add(140)?)?;
    let partition_count = streams.u32(root.add(128)?)? as usize;
    let mut result = BTreeSet::new();
    for index in first..first + count {
        let material = streams.u16(trees.add(index * 32 + 12)?)? as usize;
        let mut pending = vec![(index, false)];
        let mut active = BTreeSet::new();
        let mut complete = BTreeSet::new();
        while let Some((index, leaving)) = pending.pop() {
            if leaving { active.remove(&index); complete.insert(index); continue; }
            if complete.contains(&index) { continue; }
            if index >= tree_count || !active.insert(index) { return Err("Invalid or cyclic native collision AABB tree".into()); }
            let tree = trees.add(index * 32)?;
            let children = streams.u16(tree.add(14)?)? as usize;
            let target = streams.u32(tree.add(28)?)? as usize;
            pending.push((index, true));
            if children == 0 {
                if target >= partition_count { return Err("Native collision AABB leaf exceeds partitions".into()); }
                result.insert((target, material));
            } else {
                let end = target.checked_add(children).filter(|end| *end <= tree_count).ok_or("Native collision AABB child range exceeds trees")?;
                for child in (target..end).rev() { pending.push((child, false)); }
            }
        }
    }
    Ok(result)
}

fn triangle_shape(streams: &Streams, root: Address, info: Address, partition: usize, material: usize) -> Result<(Value, BTreeSet<usize>)> {
    let material_count = streams.u32(info.add(8)?)? as usize;
    if material >= material_count { return Err("Native collision terrain material exceeds ClipInfo materials".into()); }
    let material = pointer(streams, info.add(12)?)?.add(material * 12)?;
    let surface_flags = streams.u32(material.add(4)?)?;
    let contents = streams.u32(material.add(8)?)?;
    let entry = pointer(streams, root.add(132)?)?.add(partition * 16)?;
    let count = streams.u8(entry)? as usize;
    let first = streams.u32(entry.add(4)?)? as usize;
    let triangle_count = streams.u32(root.add(116)?)? as usize;
    let end = first.checked_add(count).filter(|end| *end <= triangle_count).ok_or("Native collision partition triangle range exceeds triangles")?;
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
            if vertex >= vertex_count { return Err("Native collision partition triangle exceeds terrain vertices".into()); }
            let next = u32::try_from(vertices.len())?;
            triangle[corner] = *remap.entry(vertex).or_insert_with(|| next);
            if triangle[corner] == next {
                let [x,y,z] = vector(streams, source_vertices.add(vertex * 12)?)?;
                vertices.push([x*0.0254,z*0.0254,-y*0.0254]);
            }
        }
        indices.push(triangle);
        owned.insert(index);
    }
    Ok((json!({"native_partition":partition,"contents":contents,"surface_flags":surface_flags,"vertices":vertices,"indices":indices}), owned))
}

fn inactive_terrain_triangles(streams: &Streams, root: Address) -> Result<BTreeSet<usize>> {
    let partition_count = streams.u32(root.add(128)?)? as usize;
    if partition_count == 0 { return Ok(BTreeSet::new()); }
    let partitions = pointer(streams, root.add(132)?)?;
    let tree_count = streams.u32(root.add(136)?)? as usize;
    let mut referenced = BTreeSet::new();
    if tree_count != 0 {
        let trees = pointer(streams, root.add(140)?)?;
        for index in 0..tree_count {
            let tree = trees.add(index * 32)?;
            if streams.u16(tree.add(14)?)? == 0 {
                let partition = streams.u32(tree.add(28)?)? as usize;
                if partition >= partition_count { return Err("Native AABB leaf references an invalid terrain partition".into()); }
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
        if count == 0 { continue; }
        let end = first.checked_add(count).filter(|end| *end <= triangle_count).ok_or("Native terrain partition storage exceeds triangle range")?;
        if referenced.contains(&index) { active.extend(first..end); } else { inactive.extend(first..end); }
    }
    inactive.retain(|triangle| !active.contains(triangle));
    Ok(inactive)
}

pub fn convert(catalog: &Catalog<'_>, pack: &mut Pack) -> Result<Value> {
    let (name, zone, root) = catalog.definitions(12).next().ok_or("Origins clip map is missing")?;
    let streams = &catalog.zones[zone].1.streams;
    let info = root.add(8)?;
    let world_brushes = pointer(streams, info.add(60)?)?;
    let mut dynamic = BTreeSet::new();
    let mut dynamic_triangles = BTreeSet::new();
    let mut static_triangles = BTreeMap::<usize, BTreeSet<(u32, u32)>>::new();
    let leaf_count = streams.u32(root.add(100)?)? as usize;
    if leaf_count != 0 {
        let leaves = pointer(streams, root.add(104)?)?;
        let mut partitions = BTreeSet::new();
        for index in 0..leaf_count {
            let leaf = leaves.add(index * 44)?;
            partitions.extend(terrain_partitions(streams, root, streams.u16(leaf)? as usize, streams.u16(leaf.add(2)?)? as usize)?);
        }
        for (partition, material) in partitions {
            let (shape, owned) = triangle_shape(streams, root, info, partition, material)?;
            let flags = (shape["contents"].as_u64().ok_or("Missing native terrain contents")? as u32,
                shape["surface_flags"].as_u64().ok_or("Missing native terrain surface flags")? as u32);
            for triangle in owned { static_triangles.entry(triangle).or_default().insert(flags); }
        }
    }
    let mut brush_models = Vec::new();
    let count = streams.u32(root.add(144)?)? as usize;
    if count != 0 {
        let submodels = pointer(streams, root.add(148)?)?;
        for index in 1..count {
            let model = submodels.add(index * 76)?;
            let model_info = if streams.u32(model.add(28)?)? == 0 {
                info
            } else {
                pointer(streams, model.add(28)?)
                    .map_err(|error| format!("Native collision submodel {index} has an unresolved ClipInfo: {error}"))?
            };
            let terrain_count = streams.u16(model.add(34)?)? as usize;
            let first_tree = streams.u16(model.add(32)?)? as usize;
            let mut triangle_shapes = Vec::new();
            for (partition, material) in terrain_partitions(streams, root, first_tree, terrain_count)? {
                let (shape, owned) = triangle_shape(streams, root, info, partition, material)?;
                dynamic_triangles.extend(owned);
                if !shape["indices"].as_array().is_some_and(|indices| indices.is_empty()) { triangle_shapes.push(shape); }
            }
            let node = streams.u32(model.add(68)?)? as usize;
            let indices = if node == 0 { BTreeSet::new() } else { brushes_in_node(streams, model_info, node)? };
            let brush_count = streams.u16(model_info.add(56)?)? as usize;
            let model_brushes = if indices.is_empty() { None } else { Some(pointer(streams, model_info.add(60)?)?) };
            let mut volumes = Vec::new();
            for brush_index in indices {
                if brush_index >= brush_count { return Err(format!("Native collision submodel {index} brush {brush_index} exceeds ClipInfo brush count {brush_count}").into()); }
                let entry = model_brushes.ok_or("Native collision submodel has no brushes")?.add(brush_index * 96)?;
                let (vertices, _) = brush_geometry(streams, entry)?;
                let vertices: Vec<_> = vertices.into_iter().map(|[x,y,z]| [x*0.0254,z*0.0254,-y*0.0254]).collect();
                volumes.push(json!({"native_brush":brush_index,"contents":streams.u32(entry.add(12)?)?,"vertices":vertices}));
                if model_brushes == Some(world_brushes) { dynamic.insert(brush_index); }
            }
            let minimum = vector(streams, model)?;
            let maximum = vector(streams, model.add(12)?)?;
            brush_models.push(json!({"native_brush_model":index,
                "bounds":[[minimum[0]*0.0254,minimum[2]*0.0254,-maximum[1]*0.0254],
                    [maximum[0]*0.0254,maximum[2]*0.0254,-minimum[1]*0.0254]],"volumes":volumes,"triangle_shapes":triangle_shapes}));
        }
    }
    let mut batches = Batches::new();
    let count = streams.u16(info.add(56)?)? as usize;
    let brushes = pointer(streams, info.add(60)?)?;
    for index in 0..count {
        let entry = brushes.add(index * 96)?;
        if !dynamic.contains(&index) && streams.u32(entry.add(12)?)? & 0x10001 != 0 {
            brush(streams, entry, &mut batches).map_err(|error| format!("Collision brush {index}: {error}"))?;
        }
    }
    let mut terrain_batches = BTreeMap::<(u32, u32), Vec<Triangle>>::new();
    let inactive_triangles = inactive_terrain_triangles(streams, root)?;
    let count = streams.u32(root.add(116)?)? as usize;
    if count != 0 {
        let vertices = pointer(streams, root.add(112)?)?;
        let vertex_count = streams.u32(root.add(108)?)? as usize;
        let triangles = pointer(streams, root.add(120)?)?;
        for index in 0..count {
            if inactive_triangles.contains(&index) { continue; }
            if dynamic_triangles.contains(&index) && !static_triangles.contains_key(&index) { continue; }
            let mut triangle = [[0.; 3]; 3];
            for (corner, offset) in [4, 2, 0].into_iter().enumerate() {
                let vertex = streams.u16(triangles.add(index * 6 + offset)?)? as usize;
                if vertex >= vertex_count { return Err("Native collision triangle exceeds terrain vertices".into()); }
                triangle[corner] = vector(streams, vertices.add(vertex * 12)?)?;
            }
            if let Some(flags) = static_triangles.get(&index) {
                for &(contents, surface_flags) in flags {
                    if contents & 0x10001 != 0 { terrain_batches.entry((contents, surface_flags)).or_default().push(triangle); }
                }
            } else {
                let partition_count = streams.u32(root.add(128)?)? as usize;
                let partitions = pointer(streams, root.add(132)?)?;
                let mut covering = BTreeSet::new();
                for partition in 0..partition_count {
                    let entry = partitions.add(partition * 16)?;
                    let first = streams.u32(entry.add(4)?)? as usize;
                    let length = streams.u8(entry)? as usize;
                    let end = first.checked_add(length).ok_or("Native terrain partition range overflow")?;
                    if (first..end).contains(&index) { covering.insert(partition); }
                }
                let tree_count = streams.u32(root.add(136)?)? as usize;
                let trees = pointer(streams, root.add(140)?)?;
                let mut leaf_references = 0;
                for tree in 0..tree_count {
                    let entry = trees.add(tree * 32)?;
                    if streams.u16(entry.add(14)?)? == 0 && covering.contains(&(streams.u32(entry.add(28)?)? as usize)) { leaf_references += 1; }
                }
                return Err(format!("Native terrain triangle {index} has no static leaf or dynamic model owner; {} serialized partitions cover it and {leaf_references} AABB leaves reference those partitions", covering.len()).into());
            }
        }
    }
    let count = streams.u32(root.add(84)?)? as usize;
    let mut model_cache = BTreeMap::new();
    if count != 0 {
        let instances = pointer(streams, root.add(88)?)?;
        for index in 0..count {
            let instance = instances.add(index * 84)?;
            if streams.u32(instance.add(8)?)? & 0x10001 == 0 { continue; }
            let (name, owner, model_root) = catalog.pointer(zone, instance.add(4)?, 5)?;
            if !model_cache.contains_key(&name) { model_cache.insert(name.clone(), model(catalog, owner, model_root)?); }
            let origin = vector(streams, instance.add(12)?)?;
            let matrix = [vector(streams, instance.add(24)?)?, vector(streams, instance.add(36)?)?, vector(streams, instance.add(48)?)?];
            for (flags, triangles) in &model_cache[&name] { for triangle in triangles {
                let mut placed = [[0.; 3]; 3];
                for corner in 0..3 {
                    let point = solve(matrix, triangle[corner]).ok_or("Singular native collision placement")?;
                    placed[corner] = std::array::from_fn(|axis| point[axis] + origin[axis]);
                }
                batches.entry(*flags).or_default().push(placed);
            }}
        }
    }
    let mut scene = Scene::new(name);
    scene.data["extras"] = json!({"brush_models":brush_models});
    let mut triangles = 0;
    for (flags, batch) in &batches {
        let positions: Vec<_> = batch.iter().flatten().map(|[x, y, z]| [(*x * 0.0254) as f32, (*z * 0.0254) as f32, (-*y * 0.0254) as f32]).collect();
        triangles += batch.len();
        scene.collision(&format!("native-surface-{flags:08x}"), &positions, *flags)?;
    }
    for ((contents, flags), batch) in &terrain_batches {
        let positions: Vec<_> = batch.iter().flatten().map(|[x,y,z]| [(*x*0.0254) as f32,(*z*0.0254) as f32,(-*y*0.0254) as f32]).collect();
        triangles += batch.len();
        scene.collision(&format!("native-terrain-{contents:08x}-{flags:08x}"), &positions, *flags)?;
        let material = scene.data["materials"].as_array_mut().ok_or("Collision scene has no materials")?.last_mut().ok_or("Collision terrain has no material")?;
        material["extras"]["surface"]["contents"] = json!(contents);
    }
    let uri = "prepared/worlds/zm_tomb.collision.gltf";
    scene.write(uri, pack)?;
    let mesh_count = batches.len() + terrain_batches.len();
    println!("Excluded {} terrain triangles belonging only to partitions unreferenced by every native AABB leaf", inactive_triangles.len());
    println!("Converted native collision: {mesh_count} meshes, {triangles} triangles, {} dynamic brushes excluded from static collision and {} native brush models retained", dynamic.len(), brush_models.len());
    Ok(json!({"uri":uri,"meshes":mesh_count,"triangles":triangles,"brush_models":brush_models}))
}
