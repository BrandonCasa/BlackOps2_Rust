use crate::{
    catalog::Catalog,
    core::*,
    fastfile::{Address, Streams},
    gltf::Scene,
    pack::Pack,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

type Vector = [f64; 3];
type Triangle = [Vector; 3];
type Batches = BTreeMap<u32, Vec<Triangle>>;

pub(crate) use brush::model as model_triangles;

fn pointer(streams: &Streams, field: Address) -> Result<Address> {
    Ok(Address::decode(streams.u32(field)?)?)
}
fn scalar(streams: &Streams, field: Address) -> Result<f64> {
    let value = f32::from_bits(streams.u32(field)?);
    if !value.is_finite() {
        return Err("Non-finite collision component".into());
    }
    Ok(value as f64)
}
fn vector(streams: &Streams, field: Address) -> Result<Vector> {
    Ok([
        scalar(streams, field)?,
        scalar(streams, field.add(4)?)?,
        scalar(streams, field.add(8)?)?,
    ])
}
fn dot(a: Vector, b: Vector) -> f64 {
    (0..3).map(|i| a[i] * b[i]).sum()
}
fn cross(a: Vector, b: Vector) -> Vector {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn subtract(a: Vector, b: Vector) -> Vector {
    std::array::from_fn(|i| a[i] - b[i])
}
fn solve(matrix: [Vector; 3], rhs: Vector) -> Option<Vector> {
    let determinant = dot(matrix[0], cross(matrix[1], matrix[2]));
    if determinant.abs() < 1e-12 {
        return None;
    }
    let a = cross(matrix[1], matrix[2]);
    let b = cross(matrix[2], matrix[0]);
    let c = cross(matrix[0], matrix[1]);
    Some(std::array::from_fn(|i| {
        (a[i] * rhs[0] + b[i] * rhs[1] + c[i] * rhs[2]) / determinant
    }))
}

mod brush;
mod terrain;
use brush::{brush, brush_geometry, brushes_in_node, model};
use terrain::{inactive_terrain_triangles, terrain_partitions, triangle_shape};

pub fn convert(catalog: &Catalog<'_>, pack: &mut Pack) -> Result<Value> {
    let (name, zone, root) = catalog
        .definitions(12)
        .next()
        .ok_or("Origins clip map is missing")?;
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
            partitions.extend(terrain_partitions(
                streams,
                root,
                streams.u16(leaf)? as usize,
                streams.u16(leaf.add(2)?)? as usize,
            )?);
        }
        for (partition, material) in partitions {
            let (shape, owned) = triangle_shape(streams, root, info, partition, material)?;
            let flags = (
                shape["contents"]
                    .as_u64()
                    .ok_or("Missing native terrain contents")? as u32,
                shape["surface_flags"]
                    .as_u64()
                    .ok_or("Missing native terrain surface flags")? as u32,
            );
            for triangle in owned {
                static_triangles.entry(triangle).or_default().insert(flags);
            }
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
                pointer(streams, model.add(28)?).map_err(|error| {
                    format!("Native collision submodel {index} has an unresolved ClipInfo: {error}")
                })?
            };
            let terrain_count = streams.u16(model.add(34)?)? as usize;
            let first_tree = streams.u16(model.add(32)?)? as usize;
            let mut triangle_shapes = Vec::new();
            for (partition, material) in
                terrain_partitions(streams, root, first_tree, terrain_count)?
            {
                let (shape, owned) = triangle_shape(streams, root, info, partition, material)?;
                dynamic_triangles.extend(owned);
                if !shape["indices"]
                    .as_array()
                    .is_some_and(|indices| indices.is_empty())
                {
                    triangle_shapes.push(shape);
                }
            }
            let node = streams.u32(model.add(68)?)? as usize;
            let indices = if node == 0 {
                BTreeSet::new()
            } else {
                brushes_in_node(streams, model_info, node)?
            };
            let brush_count = streams.u16(model_info.add(56)?)? as usize;
            let model_brushes = if indices.is_empty() {
                None
            } else {
                Some(pointer(streams, model_info.add(60)?)?)
            };
            let mut volumes = Vec::new();
            for brush_index in indices {
                if brush_index >= brush_count {
                    return Err(format!("Native collision submodel {index} brush {brush_index} exceeds ClipInfo brush count {brush_count}").into());
                }
                let entry = model_brushes
                    .ok_or("Native collision submodel has no brushes")?
                    .add(brush_index * 96)?;
                let (vertices, planes) = brush_geometry(streams, entry)?;
                let vertices: Vec<_> = vertices
                    .into_iter()
                    .map(|[x, y, z]| [x * 0.0254, z * 0.0254, -y * 0.0254])
                    .collect();
                let surface_flags: Vec<_> = planes.iter().map(|(_, _, flags)| *flags).collect();
                volumes.push(
                    json!({"native_brush":brush_index,"contents":streams.u32(entry.add(12)?)?,
                    "surface_flags":surface_flags,"vertices":vertices}),
                );
                if model_brushes == Some(world_brushes) {
                    dynamic.insert(brush_index);
                }
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
            brush(streams, entry, &mut batches)
                .map_err(|error| format!("Collision brush {index}: {error}"))?;
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
            if inactive_triangles.contains(&index) {
                continue;
            }
            if dynamic_triangles.contains(&index) && !static_triangles.contains_key(&index) {
                continue;
            }
            let mut triangle = [[0.; 3]; 3];
            for (corner, offset) in [4, 2, 0].into_iter().enumerate() {
                let vertex = streams.u16(triangles.add(index * 6 + offset)?)? as usize;
                if vertex >= vertex_count {
                    return Err("Native collision triangle exceeds terrain vertices".into());
                }
                triangle[corner] = vector(streams, vertices.add(vertex * 12)?)?;
            }
            if let Some(flags) = static_triangles.get(&index) {
                for &(contents, surface_flags) in flags {
                    if contents & 0x10001 != 0 {
                        terrain_batches
                            .entry((contents, surface_flags))
                            .or_default()
                            .push(triangle);
                    }
                }
            } else {
                let partition_count = streams.u32(root.add(128)?)? as usize;
                let partitions = pointer(streams, root.add(132)?)?;
                let mut covering = BTreeSet::new();
                for partition in 0..partition_count {
                    let entry = partitions.add(partition * 16)?;
                    let first = streams.u32(entry.add(4)?)? as usize;
                    let length = streams.u8(entry)? as usize;
                    let end = first
                        .checked_add(length)
                        .ok_or("Native terrain partition range overflow")?;
                    if (first..end).contains(&index) {
                        covering.insert(partition);
                    }
                }
                let tree_count = streams.u32(root.add(136)?)? as usize;
                let trees = pointer(streams, root.add(140)?)?;
                let mut leaf_references = 0;
                for tree in 0..tree_count {
                    let entry = trees.add(tree * 32)?;
                    if streams.u16(entry.add(14)?)? == 0
                        && covering.contains(&(streams.u32(entry.add(28)?)? as usize))
                    {
                        leaf_references += 1;
                    }
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
            if streams.u32(instance.add(8)?)? & 0x10001 == 0 {
                continue;
            }
            let (name, owner, model_root) = catalog.pointer(zone, instance.add(4)?, 5)?;
            if !model_cache.contains_key(&name) {
                model_cache.insert(
                    name.clone(),
                    model(catalog, owner, model_root, 0x10001, None)?,
                );
            }
            let origin = vector(streams, instance.add(12)?)?;
            let matrix = [
                vector(streams, instance.add(24)?)?,
                vector(streams, instance.add(36)?)?,
                vector(streams, instance.add(48)?)?,
            ];
            for (flags, triangles) in &model_cache[&name] {
                for triangle in triangles {
                    let mut placed = [[0.; 3]; 3];
                    for corner in 0..3 {
                        let point = solve(matrix, triangle[corner])
                            .ok_or("Singular native collision placement")?;
                        placed[corner] = std::array::from_fn(|axis| point[axis] + origin[axis]);
                    }
                    batches.entry(*flags).or_default().push(placed);
                }
            }
        }
    }
    let mut scene = Scene::new(name);
    scene.data["extras"] = json!({"brush_models":brush_models});
    let mut triangles = 0;
    for (flags, batch) in &batches {
        let positions: Vec<_> = batch
            .iter()
            .flatten()
            .map(|[x, y, z]| {
                [
                    (*x * 0.0254) as f32,
                    (*z * 0.0254) as f32,
                    (-*y * 0.0254) as f32,
                ]
            })
            .collect();
        triangles += batch.len();
        scene.collision(&format!("native-surface-{flags:08x}"), &positions, *flags)?;
    }
    for ((contents, flags), batch) in &terrain_batches {
        let positions: Vec<_> = batch
            .iter()
            .flatten()
            .map(|[x, y, z]| {
                [
                    (*x * 0.0254) as f32,
                    (*z * 0.0254) as f32,
                    (-*y * 0.0254) as f32,
                ]
            })
            .collect();
        triangles += batch.len();
        scene.collision(
            &format!("native-terrain-{contents:08x}-{flags:08x}"),
            &positions,
            *flags,
        )?;
        let material = scene.data["materials"]
            .as_array_mut()
            .ok_or("Collision scene has no materials")?
            .last_mut()
            .ok_or("Collision terrain has no material")?;
        material["extras"]["surface"]["contents"] = json!(contents);
    }
    let uri = "prepared/worlds/zm_tomb.collision.gltf";
    scene.write(uri, pack)?;
    let mesh_count = batches.len() + terrain_batches.len();
    println!(
        "Excluded {} terrain triangles belonging only to partitions unreferenced by every native AABB leaf",
        inactive_triangles.len()
    );
    println!(
        "Converted native collision: {mesh_count} meshes, {triangles} triangles, {} dynamic brushes excluded from static collision and {} native brush models retained",
        dynamic.len(),
        brush_models.len()
    );
    Ok(json!({"uri":uri,"meshes":mesh_count,"triangles":triangles,"brush_models":brush_models}))
}
