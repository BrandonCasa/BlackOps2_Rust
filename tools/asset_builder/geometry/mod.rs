use crate::{
    catalog::Catalog,
    core::*,
    fastfile::{Address, Streams},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Influence {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

#[derive(Clone)]
pub struct Bindings {
    pub flags: u16,
    pub blended: Vec<Option<Influence>>,
    pub rigid: Vec<Option<Influence>>,
}

pub struct Joint {
    pub name: Option<String>,
    pub parent: Option<usize>,
    pub bind: [f32; 16],
    pub bind_local: [f32; 16],
    pub inverse_bind: [f32; 16],
    pub default_rotation: [f32; 4],
    pub default_translation: [f32; 3],
}

impl Bindings {
    pub fn selected(&self) -> &[Option<Influence>] {
        if self.flags & 0x80 != 0 {
            &self.blended
        } else {
            &self.rigid
        }
    }
}

#[derive(Clone)]
pub struct Surface {
    pub name: String,
    pub source_index: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub raw_normals: Vec<[f32; 3]>,
    pub raw_tangents: Vec<[f32; 3]>,
    pub packed_basis: Vec<[u32; 2]>,
    pub colors: Vec<[u8; 4]>,
    pub uv: Vec<[f32; 2]>,
    pub lightmap_uv: Vec<[f32; 2]>,
    pub auxiliary: Vec<Vec<u8>>,
    pub bindings: Option<Bindings>,
    source_vertices: Vec<u16>,
    pub indices: Vec<u16>,
    pub material: String,
}

fn float(bytes: &[u8], offset: usize) -> Result<f32> {
    let value = f32::from_bits(u32_at(bytes, offset));
    if !value.is_finite() {
        return Err("Non-finite native geometry component".into());
    }
    Ok(value)
}

fn vector(bytes: &[u8], offset: usize) -> Result<[f32; 3]> {
    Ok([
        float(bytes, offset)?,
        float(bytes, offset + 4)?,
        float(bytes, offset + 8)?,
    ])
}

fn direction([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

fn half(value: u16) -> f32 {
    half::f16::from_bits(value).to_f32()
}

fn decoded_normal(packed: u32) -> [f32; 3] {
    [0, 10, 20].map(|shift| {
        let part = (packed >> shift & 1023) as f32 / 1023.;
        if part >= 0.5 {
            2. * part - 2.
        } else {
            2. * part
        }
    })
}

fn raw_normal(packed: u32) -> [f32; 3] {
    direction(decoded_normal(packed))
}

fn normal(packed: u32) -> Result<[f32; 3]> {
    let components = decoded_normal(packed);
    let length = components.iter().map(|v| v * v).sum::<f32>().sqrt();
    if length < 0.001 {
        return Err(format!("Zero native packed direction {packed:08x}").into());
    }
    Ok(direction(components.map(|v| v / length)))
}

impl Surface {
    pub fn select_triangles(&self, triangles: &[usize]) -> Result<Self> {
        let mut selected = self.clone();
        selected.indices = triangles
            .iter()
            .map(|&triangle| {
                self.indices
                    .get(triangle * 3..triangle * 3 + 3)
                    .ok_or_else(|| "Selected triangle exceeds surface".into())
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .copied()
            .collect();
        selected.unindex()
    }

    pub fn unindex(&self) -> Result<Self> {
        let mut result = self.clone();
        let indices: Vec<_> = self.indices.iter().map(|index| *index as usize).collect();
        result.positions = indices.iter().map(|&i| self.positions[i]).collect();
        result.normals = indices.iter().map(|&i| self.normals[i]).collect();
        result.tangents = indices.iter().map(|&i| self.tangents[i]).collect();
        result.raw_normals = indices.iter().map(|&i| self.raw_normals[i]).collect();
        result.raw_tangents = indices.iter().map(|&i| self.raw_tangents[i]).collect();
        result.packed_basis = indices.iter().map(|&i| self.packed_basis[i]).collect();
        result.colors = indices.iter().map(|&i| self.colors[i]).collect();
        result.uv = indices.iter().map(|&i| self.uv[i]).collect();
        result.lightmap_uv = indices
            .iter()
            .filter_map(|&i| self.lightmap_uv.get(i).copied())
            .collect();
        result.auxiliary = indices
            .iter()
            .filter_map(|&i| self.auxiliary.get(i).cloned())
            .collect();
        result.source_vertices = indices.iter().map(|&i| self.source_vertices[i]).collect();
        if let Some(bindings) = &self.bindings {
            result.bindings = Some(Bindings {
                flags: bindings.flags,
                blended: indices.iter().map(|&i| bindings.blended[i]).collect(),
                rigid: indices.iter().map(|&i| bindings.rigid[i]).collect(),
            });
        }
        result.indices = (0..indices.len())
            .map(u16::try_from)
            .collect::<std::result::Result<_, _>>()?;
        Ok(result)
    }

    fn read(
        name: String,
        source_index: usize,
        vertices: &[u8],
        stride: usize,
        indices: &[u8],
        material: String,
    ) -> Result<Self> {
        let mut surface = Self {
            name,
            source_index,
            positions: Vec::new(),
            normals: Vec::new(),
            tangents: Vec::new(),
            raw_normals: Vec::new(),
            raw_tangents: Vec::new(),
            packed_basis: Vec::new(),
            colors: Vec::new(),
            uv: Vec::new(),
            lightmap_uv: Vec::new(),
            auxiliary: Vec::new(),
            bindings: None,
            source_vertices: Vec::new(),
            indices: Vec::new(),
            material,
        };
        if vertices.len() % stride != 0 || indices.len() % 6 != 0 {
            return Err("Misaligned native geometry arrays".into());
        }
        let mut remap = std::collections::BTreeMap::new();
        for triangle in indices.chunks_exact(6) {
            for offset in [4, 2, 0] {
                let index = u16_at(triangle, offset);
                if index as usize >= vertices.len() / stride {
                    return Err("Native triangle exceeds bound vertex group".into());
                }
                let next = u16::try_from(remap.len())?;
                surface.indices.push(*remap.entry(index).or_insert(next));
            }
        }
        let mut source_indices: Vec<_> = remap.into_iter().collect();
        source_indices.sort_by_key(|(_, destination)| *destination);
        for (index, _) in source_indices {
            surface.source_vertices.push(index);
            let vertex = &vertices[index as usize * stride..(index as usize + 1) * stride];
            surface
                .positions
                .push(direction(vector(vertex, 0)?).map(|v| v * 0.0254));
            surface.normals.push(normal(u32_at(vertex, 24))?);
            surface.raw_normals.push(raw_normal(u32_at(vertex, 24)));
            surface.raw_tangents.push(raw_normal(u32_at(vertex, 28)));
            surface
                .packed_basis
                .push([u32_at(vertex, 24), u32_at(vertex, 28)]);
            let [x, y, z] = normal(u32_at(vertex, 28))?;
            let sign = float(vertex, 12)?;
            surface
                .tangents
                .push([x, y, z, (sign > 0.) as u8 as f32 - (sign < 0.) as u8 as f32]);
            surface.colors.push(vertex[16..20].try_into()?);
            let uv = [half(u16_at(vertex, 20)), half(u16_at(vertex, 22))];
            if !uv.iter().all(|v| v.is_finite()) {
                return Err("Non-finite native UV".into());
            }
            surface.uv.push(uv);
            if stride == 36 {
                surface.lightmap_uv.push([
                    u16_at(vertex, 32) as f32 / 65535.,
                    u16_at(vertex, 34) as f32 / 65535.,
                ]);
            }
        }
        Ok(surface)
    }
}

mod model;
mod world;
use model::pointer;
#[cfg(test)]
use model::{bind_pose, blended_bindings, rigid_bindings};
pub use model::{compose, model, skeleton};
pub use world::world;

pub fn material_names(
    catalog: &Catalog<'_>,
    required_models: Option<&std::collections::BTreeSet<String>>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut names = std::collections::BTreeSet::new();
    for (name, zone, root) in catalog.definitions(5) {
        if required_models.is_some_and(|models| !models.contains(&name.to_ascii_lowercase())) {
            continue;
        }
        let streams = &catalog.zones[zone].1.streams;
        let surfaces = pointer(streams, root.add(32)?)?;
        let materials = pointer(streams, root.add(36)?)?;
        let surface_count = streams.u8(root.add(6)?)? as usize;
        let lod_count = streams.u16(root.add(196)?)? as usize;
        if lod_count == 0 || lod_count > 4 {
            return Err(format!("Model {name} has invalid LOD count {lod_count}").into());
        }
        for lod in 0..lod_count {
            let info = root.add(40 + lod * 28)?;
            let count = streams.u16(info.add(4)?)? as usize;
            let first = streams.u16(info.add(6)?)? as usize;
            if first + count > surface_count {
                return Err(
                    format!("Model {name} LOD material range exceeds surface array").into(),
                );
            }
            for index in first..first + count {
                let surface = surfaces.add(index * 80)?;
                if streams.u16(surface.add(4)?)? != 0 && streams.u16(surface.add(6)?)? != 0 {
                    names.insert(catalog.pointer(zone, materials.add(index * 4)?, 6)?.0);
                }
            }
        }
    }
    for (_, zone, root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces = pointer(streams, root.add(872)?)?;
        for index in 0..streams.u32(root.add(16)?)? as usize {
            let surface = surfaces.add(index * 80)?;
            if streams.u16(surface.add(42)?)? != 0 {
                names.insert(catalog.pointer(zone, surface.add(48)?, 6)?.0);
            }
        }
    }
    Ok(names)
}

pub fn validate(
    catalog: &Catalog<'_>,
    required_models: Option<&std::collections::BTreeSet<String>>,
) -> Result<std::collections::BTreeSet<String>> {
    for (zone_name, zone) in catalog.zones {
        for asset in zone
            .assets
            .iter()
            .filter(|asset| asset.kind == 17 && !asset.reference)
        {
            println!(
                "Native world {zone_name}/{}: {} surfaces",
                asset.name,
                zone.streams.u32(asset.root.add(16)?)?
            );
        }
    }
    let mut model_count = 0;
    let mut surface_count = 0;
    let mut vertices = 0;
    let mut triangles = 0;
    let mut materials = std::collections::BTreeSet::new();
    for (name, zone, root) in catalog.definitions(5) {
        if required_models.is_some_and(|models| !models.contains(&name.to_ascii_lowercase())) {
            continue;
        }
        let joints = skeleton(catalog, name, zone, root)?;
        for joint in &joints {
            let identity = compose(joint.bind, joint.inverse_bind);
            if identity.iter().enumerate().any(|(index, &value)| {
                (value - if index % 5 == 0 { 1. } else { 0. }).abs() > 0.0001
            }) {
                return Err(format!(
                    "Model {name} joint {:?}: native bind and inverse do not close to identity",
                    joint.name
                )
                .into());
            }
            let reconstructed = joint.parent.map_or(joint.bind_local, |parent| {
                compose(joints[parent].bind, joint.bind_local)
            });
            if reconstructed
                .iter()
                .zip(joint.bind)
                .any(|(&a, b)| (a - b).abs() > 0.0001)
            {
                return Err(format!("Model {name} joint {:?}: exported hierarchy does not reconstruct its native bind pose",joint.name).into());
            }
        }
        let lods =
            model(catalog, name, zone, root).map_err(|error| format!("Model {name}: {error}"))?;
        model_count += 1;
        for surface in lods.iter().flatten() {
            materials.insert(surface.material.clone());
            surface_count += 1;
            vertices += surface.positions.len();
            triangles += surface.indices.len() / 3;
        }
    }
    println!(
        "Converted geometry for {model_count} models: {surface_count} surfaces, {vertices} vertices, {triangles} triangles"
    );
    for (name, zone, root) in catalog.definitions(17) {
        let surfaces = world(catalog, zone, root)?;
        materials.extend(surfaces.iter().map(|surface| surface.material.clone()));
        println!("Converted world {name}: {} surfaces", surfaces.len());
    }
    Ok(materials)
}

#[cfg(test)]
mod tests;
