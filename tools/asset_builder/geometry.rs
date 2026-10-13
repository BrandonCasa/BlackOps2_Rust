use crate::{convert::Catalog, core::*, fastfile::{Address, Streams}};

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
    pub bind: [f32;16],
    pub bind_local: [f32;16],
    pub inverse_bind: [f32;16],
    pub default_rotation: [f32;4],
    pub default_translation: [f32;3],
}

impl Bindings {
    pub fn selected(&self) -> &[Option<Influence>] {
        if self.flags&0x80 != 0 { &self.blended } else { &self.rigid }
    }
}

#[derive(Clone)]
pub struct Surface {
    pub name: String,
    pub source_index: usize,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
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
    if !value.is_finite() { return Err("Non-finite native geometry component".into()); }
    Ok(value)
}

fn vector(bytes: &[u8], offset: usize) -> Result<[f32; 3]> {
    Ok([float(bytes, offset)?, float(bytes, offset + 4)?, float(bytes, offset + 8)?])
}

fn direction([x, y, z]: [f32; 3]) -> [f32; 3] { [x, z, -y] }

fn half(value: u16) -> f32 {
    half::f16::from_bits(value).to_f32()
}

fn normal(packed: u32) -> Result<[f32; 3]> {
    let components = [0, 10, 20].map(|shift| {
        let part = (packed >> shift & 1023) as f32 / 1023.;
        if part >= 0.5 { 2. * part - 2. } else { 2. * part }
    });
    let length = components.iter().map(|v| v * v).sum::<f32>().sqrt();
    if length < 0.001 { return Err(format!("Zero native packed direction {packed:08x}").into()); }
    Ok(direction(components.map(|v| v / length)))
}

impl Surface {
    pub fn select_triangles(&self, triangles: &[usize]) -> Result<Self> {
        let mut selected = self.clone();
        selected.indices = triangles.iter().map(|&triangle| self.indices.get(triangle*3..triangle*3+3)
            .ok_or_else(|| "Selected triangle exceeds surface".into())).collect::<Result<Vec<_>>>()?
            .into_iter().flatten().copied().collect();
        selected.unindex()
    }

    pub fn unindex(&self) -> Result<Self> {
        let mut result = self.clone();
        let indices: Vec<_> = self.indices.iter().map(|index| *index as usize).collect();
        result.positions = indices.iter().map(|&i| self.positions[i]).collect();
        result.normals = indices.iter().map(|&i| self.normals[i]).collect();
        result.tangents = indices.iter().map(|&i| self.tangents[i]).collect();
        result.colors = indices.iter().map(|&i| self.colors[i]).collect();
        result.uv = indices.iter().map(|&i| self.uv[i]).collect();
        result.lightmap_uv = indices.iter().filter_map(|&i| self.lightmap_uv.get(i).copied()).collect();
        result.auxiliary = indices.iter().filter_map(|&i| self.auxiliary.get(i).cloned()).collect();
        result.source_vertices = indices.iter().map(|&i| self.source_vertices[i]).collect();
        if let Some(bindings) = &self.bindings {
            result.bindings = Some(Bindings { flags:bindings.flags,
                blended:indices.iter().map(|&i| bindings.blended[i]).collect(),
                rigid:indices.iter().map(|&i| bindings.rigid[i]).collect() });
        }
        result.indices = (0..indices.len()).map(u16::try_from).collect::<std::result::Result<_, _>>()?;
        Ok(result)
    }

    fn read(name: String, source_index: usize, vertices: &[u8], stride: usize, indices: &[u8], material: String) -> Result<Self> {
        let mut surface = Self { name, source_index, positions:Vec::new(), normals:Vec::new(), tangents:Vec::new(),
            colors:Vec::new(), uv:Vec::new(), lightmap_uv:Vec::new(), auxiliary:Vec::new(), bindings:None, source_vertices:Vec::new(), indices:Vec::new(), material };
        if vertices.len() % stride != 0 || indices.len() % 6 != 0 { return Err("Misaligned native geometry arrays".into()); }
        let mut remap = std::collections::BTreeMap::new();
        for triangle in indices.chunks_exact(6) {
            for offset in [4, 2, 0] {
                let index = u16_at(triangle, offset);
                if index as usize >= vertices.len() / stride { return Err("Native triangle exceeds bound vertex group".into()); }
                let next = u16::try_from(remap.len())?;
                surface.indices.push(*remap.entry(index).or_insert(next));
            }
        }
        let mut source_indices: Vec<_> = remap.into_iter().collect();
        source_indices.sort_by_key(|(_, destination)| *destination);
        for (index, _) in source_indices {
            surface.source_vertices.push(index);
            let vertex = &vertices[index as usize * stride..(index as usize + 1) * stride];
            surface.positions.push(direction(vector(vertex, 0)?).map(|v| v * 0.0254));
            surface.normals.push(normal(u32_at(vertex, 24))?);
            let [x, y, z] = normal(u32_at(vertex, 28))?;
            let sign = float(vertex, 12)?;
            surface.tangents.push([x, y, z, (sign > 0.) as u8 as f32 - (sign < 0.) as u8 as f32]);
            surface.colors.push(vertex[16..20].try_into()?);
            let uv = [half(u16_at(vertex, 20)), half(u16_at(vertex, 22))];
            if !uv.iter().all(|v| v.is_finite()) { return Err("Non-finite native UV".into()); }
            surface.uv.push(uv);
            if stride == 36 {
                surface.lightmap_uv.push([u16_at(vertex, 32) as f32 / 65535., u16_at(vertex, 34) as f32 / 65535.]);
            }
        }
        Ok(surface)
    }
}

fn pointer(streams: &Streams, field: Address) -> Result<Address> {
    Ok(Address::decode(streams.u32(field)?)?)
}

pub fn compose(left: [f32;16], right: [f32;16]) -> [f32;16] {
    std::array::from_fn(|index| (0..4).map(|k| left[k*4+index%4] as f64*right[index/4*4+k] as f64).sum::<f64>() as f32)
}

fn bind_pose(q: [f32;4], translation: [f32;3], h: f32) -> ([f32;16],[f32;16]) {
    let [x,y,z,w] = q;
    let native = [[1.-h*(y*y+z*z),h*(x*y-w*z),h*(x*z+w*y)],
        [h*(x*y+w*z),1.-h*(x*x+z*z),h*(y*z-w*x)],
        [h*(x*z-w*y),h*(y*z+w*x),1.-h*(x*x+y*y)]];
    let axes = [0,2,1]; let signs = [1.,1.,-1.];
    let rotation: [[f32;3];3] = std::array::from_fn(|row| std::array::from_fn(|column| signs[row]*signs[column]*native[axes[row]][axes[column]]));
    let translation = direction(translation).map(|v| v*0.0254);
    let mut forward = [0.;16]; let mut inverse = [0.;16];
    for row in 0..3 { for column in 0..3 {
        forward[column*4+row]=rotation[row][column];
        inverse[column*4+row]=rotation[column][row];
    }}
    for row in 0..3 {
        forward[12+row]=translation[row];
        inverse[12+row]=-(0..3).map(|k| rotation[k][row] as f64*translation[k] as f64).sum::<f64>() as f32;
    }
    forward[15]=1.; inverse[15]=1.;
    (forward,inverse)
}

pub fn skeleton(catalog: &Catalog<'_>, name: &str, zone: usize, root: Address) -> Result<Vec<Joint>> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u8(root.add(4)?)? as usize;
    let roots = streams.u8(root.add(5)?)? as usize;
    if count == 0 || roots == 0 || roots > count { return Err(format!("Model {name} has invalid native joint/root counts {count}/{roots}").into()); }
    let names = pointer(streams,root.add(8)?)?;
    let parents = if roots < count { Some(pointer(streams,root.add(12)?)?) } else { None };
    let defaults = if roots < count { Some((pointer(streams,root.add(16)?)?,pointer(streams,root.add(20)?)?)) } else { None };
    let poses = pointer(streams,root.add(28)?)?;
    let mut joints = Vec::<Joint>::with_capacity(count);
    for index in 0..count {
        let word = streams.u16(names.add(index*2)?)? as usize;
        let address = streams.script_strings.get(word).copied()
            .ok_or_else(|| format!("Model {name} joint {index}: serialized script-string index {word} is missing in its owning zone"))?;
        let joint_name = address.map(|address| streams.text(address)).transpose()?;
        let parent = if index < roots { None } else {
            let delta = streams.u8(parents.unwrap().add(index-roots)?)? as usize;
            let parent = index.checked_sub(delta).filter(|&parent| parent < index)
                .ok_or_else(|| format!("Model {name} joint {index} has invalid native parent delta {delta}"))?;
            Some(parent)
        };
        let bytes = streams.read(poses.add(index*32)?,32)?;
        let q = [float(bytes,0)?,float(bytes,4)?,float(bytes,8)?,float(bytes,12)?];
        let (bind,inverse_bind) = bind_pose(q,vector(bytes,16)?,float(bytes,28)?);
        if bind.iter().chain(&inverse_bind).any(|value| !value.is_finite()) { return Err(format!("Model {name} joint {index} bind matrix is not finite").into()); }
        let bind_local = parent.map_or(bind,|parent| compose(joints[parent].inverse_bind,bind));
        let (default_rotation,default_translation) = if index < roots { ([0.,0.,0.,1.],[0.;3]) } else {
            let (quaternions,translations) = defaults.unwrap();
            let ordinal = index-roots;
            let quaternion = streams.read(quaternions.add(ordinal*8)?,8)?;
            let q = [0,2,4,6].map(|offset| (u16_at(quaternion,offset) as i16) as f32*f32::from_bits(0x38000100));
            let [x,y,z] = direction([q[0],q[1],q[2]]);
            let translation = direction(vector(streams.read(translations.add(ordinal*12)?,12)?,0)?).map(|v| v*0.0254);
            ([x,y,z,q[3]],translation)
        };
        joints.push(Joint {name:joint_name,parent,bind,bind_local,inverse_bind,default_rotation,default_translation});
    }
    Ok(joints)
}

fn joint(offset: u16, bones: usize) -> Result<u16> {
    if offset%64 != 0 || offset as usize/64 >= bones {
        return Err(format!("Native bone palette offset {offset} is invalid for {bones} model-local joints").into());
    }
    Ok(offset/64)
}

fn blended_bindings(counts: [usize;4], bytes: &[u8], bones: usize, vertices: usize) -> Result<Vec<Influence>> {
    let count: usize = counts.iter().sum();
    let words: usize = counts.iter().enumerate().map(|(group,count)| count*(2*group+1)).sum();
    if count > vertices || bytes.len() != words*2 { return Err("Native blend records exceed their raw vertex domain or byte span".into()); }
    let mut result = Vec::with_capacity(count);
    let mut cursor = 0;
    for (group,count) in counts.into_iter().enumerate() {
        for _ in 0..count {
            let mut value = Influence {joints:[0;4],weights:[0.;4]};
            value.joints[0]=joint(u16_at(bytes,cursor),bones)?;
            cursor+=2;
            let mut remaining = 65536_u32;
            for lane in 1..=group {
                value.joints[lane]=joint(u16_at(bytes,cursor),bones)?;
                let weight = u16_at(bytes,cursor+2) as u32;
                remaining=remaining.checked_sub(weight).ok_or("Native explicit blend weights exceed one")?;
                value.weights[lane]=weight as f32*f32::from_bits(0x37800000);
                cursor+=4;
            }
            value.weights[0]=remaining as f32*f32::from_bits(0x37800000);
            result.push(value);
        }
    }
    Ok(result)
}

fn rigid_bindings(bytes: &[u8], bones: usize, vertices: usize) -> Result<Vec<Influence>> {
    if bytes.len()%12 != 0 { return Err("Misaligned native rigid vertex lists".into()); }
    let mut result = Vec::new();
    for record in bytes.chunks_exact(12) {
        let bone = joint(u16_at(record,0),bones)?;
        let end = result.len()+u16_at(record,2) as usize;
        if end > vertices { return Err("Native rigid lists exceed their raw vertex domain".into()); }
        result.resize(end,Influence {joints:[bone,0,0,0],weights:[1.,0.,0.,0.]});
    }
    Ok(result)
}

fn bindings(streams: &Streams, surface: Address, bones: usize, vertices: usize, source: &[u16]) -> Result<Bindings> {
    let flags = streams.u16(surface.add(2)?)?;
    let mut counts = [0;4];
    for (group,count) in counts.iter_mut().enumerate() {
        let value = streams.u16(surface.add(16+group*2)?)? as i16;
        if value < 0 { return Err("Negative native blend vertex count".into()); }
        *count=value as usize;
    }
    let words: usize = counts.iter().enumerate().map(|(group,count)| count*(2*group+1)).sum();
    let blend_bytes = if words == 0 { &[] } else { streams.read(pointer(streams,surface.add(24)?)?,words*2)? };
    let blended = blended_bindings(counts,blend_bytes,bones,vertices)?;
    let lists = streams.u8(surface.add(1)?)? as usize;
    let rigid_bytes = if lists == 0 { &[] } else { streams.read(pointer(streams,surface.add(40)?)?,lists*12)? };
    let rigid = rigid_bindings(rigid_bytes,bones,vertices)?;
    let result = Bindings {flags,
        blended:source.iter().map(|&vertex| blended.get(vertex as usize).copied()).collect(),
        rigid:source.iter().map(|&vertex| rigid.get(vertex as usize).copied()).collect()};
    if result.selected().iter().any(Option::is_none) {
        return Err(format!("Native model vertex has no binding in its selected consumer (flags {flags:#x}, blended {}, rigid {}, raw vertices {vertices})",blended.len(),rigid.len()).into());
    }
    Ok(result)
}

pub fn model(catalog: &Catalog<'_>, name: &str, zone: usize, root: Address) -> Result<Vec<Vec<Surface>>> {
    let streams = &catalog.zones[zone].1.streams;
    let bones = streams.u8(root.add(4)?)? as usize;
    let surface_count = streams.u8(root.add(6)?)? as usize;
    let surfaces = pointer(streams, root.add(32)?)?;
    let materials = pointer(streams, root.add(36)?)?;
    let lod_count = streams.u16(root.add(196)?)? as usize;
    if lod_count == 0 || lod_count > 4 { return Err(format!("Model {name} has invalid LOD count {lod_count}").into()); }
    let mut lods = Vec::new();
    for lod in 0..lod_count {
        let info = root.add(40 + lod * 28)?;
        let count = streams.u16(info.add(4)?)? as usize;
        let first = streams.u16(info.add(6)?)? as usize;
        if first + count > surface_count { return Err(format!("Model {name} LOD range exceeds surface array").into()); }
        let mut meshes = Vec::new();
        for index in first..first + count {
            let surface = surfaces.add(index * 80)?;
            let vertices = streams.u16(surface.add(4)?)? as usize;
            let triangles = streams.u16(surface.add(6)?)? as usize;
            if vertices == 0 || triangles == 0 { continue; }
            let flags = streams.u16(surface.add(2)?)?;
            if flags & 1 != 0 { return Err(format!("Model {name} surface {index}: quantized vertex source is not recovered (flags {flags:#x})").into()); }
            let material = catalog.pointer(zone, materials.add(index * 4)?, 6)?.0;
            let mut mesh = Surface::read(format!("{name}_surface{index}"), index,
                streams.read(pointer(streams, surface.add(32)?)?, vertices * 32)?, 32,
                streams.read(pointer(streams, surface.add(12)?)?, triangles * 6)?, material)?;
            mesh.bindings=Some(bindings(streams,surface,bones,vertices,&mesh.source_vertices)
                .map_err(|error| format!("Model {name} surface {index} bindings: {error}"))?);
            meshes.push(mesh);
        }
        lods.push(meshes);
    }
    Ok(lods)
}

pub fn world(catalog: &Catalog<'_>, zone: usize, root: Address) -> Result<Vec<Surface>> {
    let streams = &catalog.zones[zone].1.streams;
    let draw = root.add(396)?;
    let size = streams.u32(draw.add(32)?)? as usize;
    let vertices = streams.read(pointer(streams, draw.add(36)?)?, size)?;
    let indices = streams.read(pointer(streams, draw.add(60)?)?, streams.u32(draw.add(56)?)? as usize * 2)?;
    let surfaces = pointer(streams, root.add(872)?)?;
    let count = streams.u32(root.add(16)?)? as usize;
    let secondary_size = streams.u32(draw.add(44)?)? as usize;
    let secondary = if secondary_size == 0 { &[] } else {
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
        let bytes = indices.get(first * 2..(first + index_count) * 2).ok_or("World index span exceeds buffer")?;
        for visited in covered.get_mut(first..first + index_count).ok_or("World index span exceeds buffer")? {
            if std::mem::replace(visited, true) { return Err("Overlapping world surface index spans".into()); }
        }
        let span = bytes.chunks_exact(2).map(|v| u16_at(v, 0) as usize + 1).max().unwrap_or(0);
        let group = groups.entry(offset).or_insert((secondary_offset, 0));
        if group.0 != secondary_offset { return Err("World primary group has inconsistent secondary offset".into()); }
        group.1 = group.1.max(span);
    }
    if covered.iter().any(|visited| !visited) { return Err("World has unreferenced index-buffer entries".into()); }
    let offsets: Vec<_> = groups.keys().copied().collect();
    let mut auxiliary_stride = std::collections::BTreeMap::new();
    let mut group_vertices = 0;
    for (index, &offset) in offsets.iter().enumerate() {
        let (secondary_offset, span) = groups[&offset];
        if span == 0 { return Err("Empty native world vertex group".into()); }
        group_vertices += span;
        let next_primary = offsets.get(index + 1).copied().unwrap_or(vertices.len());
        if (offset + span * 36).next_multiple_of(16) != next_primary {
            return Err("Native world vertex groups overlap or have unexpected padding".into());
        }
        let next_secondary = offsets.get(index + 1).map(|next| groups[next].0).unwrap_or(secondary.len());
        let bytes = next_secondary.checked_sub(secondary_offset).ok_or("World secondary offsets regress")?;
        if bytes % span != 0 || !matches!(bytes / span, 0 | 4 | 8 | 12 | 16) {
            return Err("Unknown native world auxiliary vertex stride".into());
        }
        auxiliary_stride.insert(offset, bytes / span);
    }
    if group_vertices != streams.u32(draw.add(28)?)? as usize { return Err("World shared groups do not cover native vertex count".into()); }
    println!("Validated {} shared world vertex groups: {group_vertices} primary vertices, {} auxiliary bytes", groups.len(), secondary.len());
    let mut meshes = Vec::new();
    let mut total_indices = 0;
    for index in 0..count {
        let surface = surfaces.add(index * 80)?;
        let offset = streams.u32(surface.add(12)?)? as usize;
        let native_count = streams.u16(surface.add(40)?)? as usize;
        let triangles = streams.u16(surface.add(42)?)? as usize;
        let first = streams.u32(surface.add(44)?)? as usize;
        total_indices += triangles * 3;
        if triangles == 0 { continue; }
        let triangle_bytes = indices.get(first * 2..first * 2 + triangles * 6)
            .ok_or("World surface exceeds native index buffer")?;
        let referenced = triangle_bytes.chunks_exact(2).map(|bytes| u16_at(bytes, 0) as usize + 1)
            .max().ok_or("Empty world triangle span")?;
        let vertex_count = if native_count == 0 { referenced } else { native_count };
        if vertex_count < referenced { return Err("World indices exceed declared vertex group".into()); }
        let material = catalog.pointer(zone, surface.add(48)?, 6)?.0;
        let mut mesh = Surface::read(format!("world_surface{index}"), index,
            vertices.get(offset..offset + vertex_count * 36).ok_or("World surface exceeds native vertex buffer")?, 36,
            triangle_bytes, material)?;
        let stride = auxiliary_stride[&offset];
        if stride != 0 {
            let base = groups[&offset].0;
            for &vertex in &mesh.source_vertices {
                let start = base + vertex as usize * stride;
                mesh.auxiliary.push(secondary.get(start..start + stride).ok_or("World auxiliary vertex exceeds buffer")?.to_vec());
            }
        }
        meshes.push(mesh);
    }
    if total_indices != indices.len() / 2 { return Err("World surface index spans do not cover the native index buffer".into()); }
    Ok(meshes)
}

pub fn material_names(catalog: &Catalog<'_>) -> Result<std::collections::BTreeSet<String>> {
    let mut names = std::collections::BTreeSet::new();
    for (name,zone,root) in catalog.definitions(5) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces = pointer(streams,root.add(32)?)?;
        let materials = pointer(streams,root.add(36)?)?;
        let surface_count = streams.u8(root.add(6)?)? as usize;
        let lod_count = streams.u16(root.add(196)?)? as usize;
        if lod_count == 0 || lod_count > 4 { return Err(format!("Model {name} has invalid LOD count {lod_count}").into()); }
        for lod in 0..lod_count {
            let info = root.add(40+lod*28)?;
            let count = streams.u16(info.add(4)?)? as usize;
            let first = streams.u16(info.add(6)?)? as usize;
            if first+count > surface_count { return Err(format!("Model {name} LOD material range exceeds surface array").into()); }
            for index in first..first+count {
                let surface = surfaces.add(index*80)?;
                if streams.u16(surface.add(4)?)? != 0 && streams.u16(surface.add(6)?)? != 0 {
                    names.insert(catalog.pointer(zone,materials.add(index*4)?,6)?.0);
                }
            }
        }
    }
    for (_,zone,root) in catalog.definitions(17) {
        let streams = &catalog.zones[zone].1.streams;
        let surfaces = pointer(streams,root.add(872)?)?;
        for index in 0..streams.u32(root.add(16)?)? as usize {
            let surface = surfaces.add(index*80)?;
            if streams.u16(surface.add(42)?)? != 0 {
                names.insert(catalog.pointer(zone,surface.add(48)?,6)?.0);
            }
        }
    }
    Ok(names)
}

pub fn validate(catalog: &Catalog<'_>) -> Result<std::collections::BTreeSet<String>> {
    for (zone_name, zone) in catalog.zones {
        for asset in zone.assets.iter().filter(|asset| asset.kind == 17 && !asset.reference) {
            println!("Native world {zone_name}/{}: {} surfaces", asset.name, zone.streams.u32(asset.root.add(16)?)?);
        }
    }
    let mut model_count = 0;
    let mut surface_count = 0;
    let mut vertices = 0;
    let mut triangles = 0;
    let mut materials = std::collections::BTreeSet::new();
    for (name, zone, root) in catalog.definitions(5) {
        let joints = skeleton(catalog,name,zone,root)?;
        for joint in &joints {
            let identity = compose(joint.bind,joint.inverse_bind);
            if identity.iter().enumerate().any(|(index,&value)| (value-if index%5 == 0 {1.} else {0.}).abs() > 0.0001) {
                return Err(format!("Model {name} joint {:?}: native bind and inverse do not close to identity",joint.name).into());
            }
            let reconstructed = joint.parent.map_or(joint.bind_local,|parent| compose(joints[parent].bind,joint.bind_local));
            if reconstructed.iter().zip(joint.bind).any(|(&a,b)| (a-b).abs() > 0.0001) {
                return Err(format!("Model {name} joint {:?}: exported hierarchy does not reconstruct its native bind pose",joint.name).into());
            }
        }
        let lods = model(catalog, name, zone, root).map_err(|error| format!("Model {name}: {error}"))?;
        model_count += 1;
        for surface in lods.iter().flatten() {
            materials.insert(surface.material.clone());
            surface_count += 1; vertices += surface.positions.len(); triangles += surface.indices.len() / 3;
        }
    }
    println!("Converted geometry for {model_count} models: {surface_count} surfaces, {vertices} vertices, {triangles} triangles");
    for (name, zone, root) in catalog.definitions(17) {
        let surfaces = world(catalog, zone, root)?;
        materials.extend(surfaces.iter().map(|surface| surface.material.clone()));
        println!("Converted world {name}: {} surfaces", surfaces.len());
    }
    Ok(materials)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlit_vertex_alpha_matches_constant_rgb_source_interpolation() {
        let mut vertices = vec![0;3*32];
        for (index,vertex) in vertices.chunks_exact_mut(32).enumerate() {
            vertex[..4].copy_from_slice(&(index as f32).to_le_bytes());
            vertex[12..16].copy_from_slice(&1_f32.to_le_bytes());
            vertex[24..28].copy_from_slice(&1_u32.to_le_bytes());
            vertex[28..32].copy_from_slice(&1024_u32.to_le_bytes());
        }
        let indices: Vec<_> = [0_u16,1,2].into_iter().flat_map(u16::to_le_bytes).collect();
        let mut surface = Surface::read("unlit".into(),0,&vertices,32,&indices,"unlit".into()).unwrap();
        surface.colors = vec![[128,64,32,0],[128,64,32,128],[128,64,32,255]];
        for (transfer,coverage) in [("gamma2_alpha","unlit_blend"),("gamma2","unlit_add")] {
            let mut scene = crate::gltf::Scene::new("unlit");
            let converted = std::collections::BTreeMap::from([("unlit".into(),serde_json::json!({"name":"unlit",
                "extras":{"surface":{"source_vertex_color_transfer":transfer,"source_coverage":coverage}}}))]);
            let materials = scene.materials([&surface],&converted).unwrap();
            scene.mesh("unlit",[&surface],&materials,false).unwrap();
            let color_index = scene.data["meshes"][0]["primitives"][0]["attributes"]["COLOR_0"].as_u64().unwrap() as usize;
            let accessor = &scene.data["accessors"][color_index];
            assert_eq!(accessor["componentType"],5126);
            let start = scene.data["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize]["byteOffset"].as_u64().unwrap() as usize;
            let mut pack = crate::pack::Pack::default();
            scene.write("prepared/unlit.gltf",&mut pack).unwrap();
            let bytes = pack.get("prepared/unlit.bin").unwrap();
            let values: [[f32;4];3] = std::array::from_fn(|i| std::array::from_fn(|c| f32::from_bits(u32_at(bytes,start+i*16+c*4))));
            let weights = [0.2,0.3,0.5];
            let alpha = 0.3*128./255.+0.5;
            for c in 0..3 {
                let interpolated: f32 = (0..3).map(|i| values[i][c]*weights[i]).sum();
                let tint = [128.,64.,32.][c]/255.;
                let expected = tint*tint*if coverage == "unlit_blend" {alpha} else {1.};
                assert!((interpolated-expected).abs()<0.000001);
            }
            if coverage == "unlit_add" { assert!(values.iter().all(|value| value[3] == 1.)); }
            else { assert_eq!(values.map(|value| value[3]),[0.,128./255.,1.]); }
        }
    }

    #[test]
    fn native_binding_groups_and_weight_endpoints() {
        let words = [128_u16,192,64,65535,64,128,16384,192,32768,0,64,8192,128,16384,192,32768];
        let bytes: Vec<_> = words.into_iter().flat_map(u16::to_le_bytes).collect();
        let values = blended_bindings([1;4],&bytes,4,4).unwrap();
        assert_eq!(values[0],Influence {joints:[2,0,0,0],weights:[1.,0.,0.,0.]});
        assert_eq!(values[1],Influence {joints:[3,1,0,0],weights:[0.0000152587890625,0.9999847412109375,0.,0.]});
        assert_eq!(values[2],Influence {joints:[1,2,3,0],weights:[0.25,0.25,0.5,0.]});
        assert_eq!(values[3],Influence {joints:[0,1,2,3],weights:[0.125,0.125,0.25,0.5]});
        for offset in [65_u16,256] { assert!(blended_bindings([1,0,0,0],&offset.to_le_bytes(),4,1).is_err()); }
        let overflow: Vec<_> = [0_u16,64,65535,128,2].into_iter().flat_map(u16::to_le_bytes).collect();
        assert!(blended_bindings([0,0,1,0],&overflow,4,1).is_err());
        assert!(blended_bindings([1;4],&bytes[..bytes.len()-1],4,4).is_err());
        assert!(blended_bindings([1;4],&bytes,4,3).is_err());
        let rigid: Vec<_> = [192_u16,2,65535,65535,0,0,64,1,0,0,0,0].into_iter().flat_map(u16::to_le_bytes).collect();
        assert_eq!(rigid_bindings(&rigid,4,3).unwrap().iter().map(|value| value.joints[0]).collect::<Vec<_>>(),[3,3,1]);
        assert!(rigid_bindings(&rigid,4,2).is_err());
    }

    #[test]
    fn binding_remap_follows_raw_vertices_through_triangle_selection() {
        let mut vertices = vec![0;3*32];
        for (index,vertex) in vertices.chunks_exact_mut(32).enumerate() {
            vertex[..4].copy_from_slice(&(index as f32).to_le_bytes());
            vertex[12..16].copy_from_slice(&1_f32.to_le_bytes());
            vertex[24..28].copy_from_slice(&1_u32.to_le_bytes());
            vertex[28..32].copy_from_slice(&1024_u32.to_le_bytes());
        }
        let indices: Vec<_> = [2_u16,0,1,1,2,0].into_iter().flat_map(u16::to_le_bytes).collect();
        let mut surface = Surface::read("binding-test".into(),0,&vertices,32,&indices,"binding-test".into()).unwrap();
        surface.bindings=Some(Bindings {flags:0x80,
            blended:surface.source_vertices.iter().map(|&vertex| Some(Influence {joints:[vertex,0,0,0],weights:[1.,0.,0.,0.]})).collect(),
            rigid:surface.source_vertices.iter().map(|&vertex| Some(Influence {joints:[3-vertex,0,0,0],weights:[1.,0.,0.,0.]})).collect()});
        let selected = surface.select_triangles(&[1,0]).unwrap();
        let mut scene = crate::gltf::Scene::new("binding-test");
        let converted = std::collections::BTreeMap::from([("binding-test".into(),serde_json::json!({"name":"binding-test"}))]);
        let materials = scene.materials([&selected],&converted).unwrap();
        let mesh = scene.mesh("binding-test",[&selected],&materials,true).unwrap();
        let owner = scene.node(serde_json::json!({"name":"binding-test","mesh":mesh}));
        let (identity,inverse) = bind_pose([0.,0.,0.,1.],[0.;3],2.);
        let joints = vec![
            Joint {name:None,parent:None,bind:identity,bind_local:identity,inverse_bind:inverse,default_rotation:[0.,0.,0.,1.],default_translation:[0.;3]},
            Joint {name:Some("second-root".into()),parent:None,bind:identity,bind_local:identity,inverse_bind:inverse,default_rotation:[0.,0.,0.,1.],default_translation:[0.;3]},
            Joint {name:Some(String::new()),parent:Some(0),bind:identity,bind_local:identity,inverse_bind:inverse,default_rotation:[0.,0.,0.,1.],default_translation:[1.,2.,3.]},
        ];
        scene.skin(owner,&joints).unwrap();
        let mut pack = crate::pack::Pack::default();
        scene.write("prepared/binding-test.gltf",&mut pack).unwrap();
        let document: serde_json::Value = serde_json::from_slice(pack.get("prepared/binding-test.gltf").unwrap()).unwrap();
        let attributes = &document["meshes"][0]["primitives"][0]["attributes"];
        let accessor = &document["accessors"][attributes["JOINTS_0"].as_u64().unwrap() as usize];
        assert_eq!(accessor["componentType"],5123);
        assert_eq!(accessor["count"],6);
        assert_eq!(accessor["type"],"VEC4");
        let view = &document["bufferViews"][accessor["bufferView"].as_u64().unwrap() as usize];
        let bytes = pack.get("prepared/binding-test.bin").unwrap();
        let start = view["byteOffset"].as_u64().unwrap() as usize;
        assert_eq!((0..6).map(|vertex| u16_at(bytes,start+vertex*8)).collect::<Vec<_>>(),[0,2,1,1,0,2]);
        let weights = &document["accessors"][attributes["WEIGHTS_0"].as_u64().unwrap() as usize];
        assert_eq!(weights["componentType"],5126);
        assert_eq!(weights["count"],6);
        assert_eq!(weights["type"],"VEC4");
        assert_eq!(document["nodes"][owner]["skin"],0);
        assert_eq!(document["skins"][0]["joints"],serde_json::json!([1,2,3]));
        assert_eq!(document["nodes"][owner]["children"],serde_json::json!([1,2]));
        assert_eq!(document["nodes"][1]["children"],serde_json::json!([3]));
        assert!(document["nodes"][1].get("matrix").is_none());
        assert_eq!(document["nodes"][1]["translation"],serde_json::json!([0.,0.,0.]));
        assert_eq!(document["nodes"][1]["rotation"],serde_json::json!([0.,0.,0.,1.]));
        assert!(document["nodes"][1].get("name").is_none());
        assert_eq!(document["nodes"][2]["name"],"second-root");
        assert_eq!(document["nodes"][3]["name"],"");
        assert_eq!(document["nodes"][3]["translation"],serde_json::json!([0.,0.,0.]));
        assert_eq!(document["nodes"][3]["extras"]["native_local_default"]["translation"],serde_json::json!([1.,2.,3.]));
        let inverse = &document["accessors"][document["skins"][0]["inverseBindMatrices"].as_u64().unwrap() as usize];
        assert_eq!(inverse["componentType"],5126);
        assert_eq!(inverse["count"],3);
        assert_eq!(inverse["type"],"MAT4");
        let view = &document["bufferViews"][inverse["bufferView"].as_u64().unwrap() as usize];
        assert!(view.get("target").is_none());
        let start = view["byteOffset"].as_u64().unwrap() as usize;
        assert_eq!((0..16).map(|index| f32::from_bits(u32_at(bytes,start+index*4))).collect::<Vec<_>>(),identity);
        let bindings = selected.bindings.unwrap();
        assert_eq!(bindings.flags,0x80);
        assert_eq!(bindings.blended.iter().map(|value| value.unwrap().joints[0]).collect::<Vec<_>>(),[0,2,1,1,0,2]);
        assert_eq!(bindings.rigid.iter().map(|value| value.unwrap().joints[0]).collect::<Vec<_>>(),[3,1,2,2,3,1]);
    }

    #[test]
    fn bind_pose_converts_native_space_and_preserves_parent_offsets() {
        let q = [0.,0.,0.5_f32.sqrt(),0.5_f32.sqrt()];
        let (parent,inverse) = bind_pose(q,[10.,20.,30.],2.);
        let mapped: [f32;3] = std::array::from_fn(|row| parent[12+row]+parent[row]*0.0254);
        for (a,b) in mapped.into_iter().zip([0.254,0.762,-0.5334]) { assert!((a-b).abs() < 0.000001); }
        let restored: [f32;3] = std::array::from_fn(|row| inverse[12+row]+(0..3).map(|column| inverse[column*4+row]*mapped[column]).sum::<f32>());
        for (a,b) in restored.into_iter().zip([0.0254,0.,0.]) { assert!((a-b).abs() < 0.000001); }
        let (child,_) = bind_pose(q,[10.,25.,30.],2.);
        let local = compose(inverse,child);
        for (a,b) in local[12..15].iter().copied().zip([0.127,0.,0.]) { assert!((a-b).abs() < 0.000001); }
        let reconstructed = compose(parent,local);
        for (a,b) in reconstructed.into_iter().zip(child) { assert!((a-b).abs() < 0.000001); }
    }
}
