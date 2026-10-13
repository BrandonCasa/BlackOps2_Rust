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

pub fn convert(catalog: &Catalog<'_>, pack: &mut Pack) -> Result<Value> {
    let (name, zone, root) = catalog.definitions(12).next().ok_or("Origins clip map is missing")?;
    let streams = &catalog.zones[zone].1.streams;
    let info = root.add(8)?;
    let world_brushes = pointer(streams, info.add(60)?)?;
    let mut dynamic = BTreeSet::new();
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
            let terrain_count = streams.u16(model.add(34)?)?;
            if terrain_count != 0 {
                return Err(format!("Native collision submodel {index} requires {terrain_count} terrain AABB trees starting at {}; dynamic terrain collision conversion is not recovered", streams.u16(model.add(32)?)?).into());
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
                    [maximum[0]*0.0254,maximum[2]*0.0254,-minimum[1]*0.0254]],"volumes":volumes}));
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
    let count = streams.u32(root.add(116)?)? as usize;
    if count != 0 {
        let vertices = pointer(streams, root.add(112)?)?;
        let vertex_count = streams.u32(root.add(108)?)? as usize;
        let triangles = pointer(streams, root.add(120)?)?;
        for index in 0..count {
            let mut triangle = [[0.; 3]; 3];
            for (corner, offset) in [4, 2, 0].into_iter().enumerate() {
                let vertex = streams.u16(triangles.add(index * 6 + offset)?)? as usize;
                if vertex >= vertex_count { return Err("Native collision triangle exceeds terrain vertices".into()); }
                triangle[corner] = vector(streams, vertices.add(vertex * 12)?)?;
            }
            batches.entry(0).or_default().push(triangle);
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
    let uri = "prepared/worlds/zm_tomb.collision.gltf";
    scene.write(uri, pack)?;
    println!("Converted native collision: {} meshes, {triangles} triangles, {} dynamic brushes excluded from static collision and {} native brush models retained", batches.len(), dynamic.len(), brush_models.len());
    Ok(json!({"uri":uri,"meshes":batches.len(),"triangles":triangles,"brush_models":brush_models}))
}
