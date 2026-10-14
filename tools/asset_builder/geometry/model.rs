use super::*;

pub(super) fn pointer(streams: &Streams, field: Address) -> Result<Address> {
    Ok(Address::decode(streams.u32(field)?)?)
}

pub fn compose(left: [f32; 16], right: [f32; 16]) -> [f32; 16] {
    std::array::from_fn(|index| {
        (0..4)
            .map(|k| left[k * 4 + index % 4] as f64 * right[index / 4 * 4 + k] as f64)
            .sum::<f64>() as f32
    })
}

pub(super) fn bind_pose(q: [f32; 4], translation: [f32; 3], h: f32) -> ([f32; 16], [f32; 16]) {
    let [x, y, z, w] = q;
    let native = [
        [
            1. - h * (y * y + z * z),
            h * (x * y - w * z),
            h * (x * z + w * y),
        ],
        [
            h * (x * y + w * z),
            1. - h * (x * x + z * z),
            h * (y * z - w * x),
        ],
        [
            h * (x * z - w * y),
            h * (y * z + w * x),
            1. - h * (x * x + y * y),
        ],
    ];
    let axes = [0, 2, 1];
    let signs = [1., 1., -1.];
    let rotation: [[f32; 3]; 3] = std::array::from_fn(|row| {
        std::array::from_fn(|column| signs[row] * signs[column] * native[axes[row]][axes[column]])
    });
    let translation = direction(translation).map(|v| v * 0.0254);
    let mut forward = [0.; 16];
    let mut inverse = [0.; 16];
    for row in 0..3 {
        for column in 0..3 {
            forward[column * 4 + row] = rotation[row][column];
            inverse[column * 4 + row] = rotation[column][row];
        }
    }
    for row in 0..3 {
        forward[12 + row] = translation[row];
        inverse[12 + row] = -(0..3)
            .map(|k| rotation[k][row] as f64 * translation[k] as f64)
            .sum::<f64>() as f32;
    }
    forward[15] = 1.;
    inverse[15] = 1.;
    (forward, inverse)
}

pub fn skeleton(
    catalog: &Catalog<'_>,
    name: &str,
    zone: usize,
    root: Address,
) -> Result<Vec<Joint>> {
    let streams = &catalog.zones[zone].1.streams;
    let count = streams.u8(root.add(4)?)? as usize;
    let roots = streams.u8(root.add(5)?)? as usize;
    if count == 0 || roots == 0 || roots > count {
        return Err(
            format!("Model {name} has invalid native joint/root counts {count}/{roots}").into(),
        );
    }
    let names = pointer(streams, root.add(8)?)?;
    let parents = if roots < count {
        Some(pointer(streams, root.add(12)?)?)
    } else {
        None
    };
    let defaults = if roots < count {
        Some((
            pointer(streams, root.add(16)?)?,
            pointer(streams, root.add(20)?)?,
        ))
    } else {
        None
    };
    let poses = pointer(streams, root.add(28)?)?;
    let mut joints = Vec::<Joint>::with_capacity(count);
    for index in 0..count {
        let word = streams.u16(names.add(index * 2)?)? as usize;
        let address = streams.script_strings.get(word).copied()
            .ok_or_else(|| format!("Model {name} joint {index}: serialized script-string index {word} is missing in its owning zone"))?;
        let joint_name = address.map(|address| streams.text(address)).transpose()?;
        let parent = if index < roots {
            None
        } else {
            let delta = streams.u8(parents.unwrap().add(index - roots)?)? as usize;
            let parent = index
                .checked_sub(delta)
                .filter(|&parent| parent < index)
                .ok_or_else(|| {
                    format!("Model {name} joint {index} has invalid native parent delta {delta}")
                })?;
            Some(parent)
        };
        let bytes = streams.read(poses.add(index * 32)?, 32)?;
        let q = [
            float(bytes, 0)?,
            float(bytes, 4)?,
            float(bytes, 8)?,
            float(bytes, 12)?,
        ];
        let (bind, inverse_bind) = bind_pose(q, vector(bytes, 16)?, float(bytes, 28)?);
        if bind
            .iter()
            .chain(&inverse_bind)
            .any(|value| !value.is_finite())
        {
            return Err(format!("Model {name} joint {index} bind matrix is not finite").into());
        }
        let bind_local = parent.map_or(bind, |parent| compose(joints[parent].inverse_bind, bind));
        let (default_rotation, default_translation) = if index < roots {
            ([0., 0., 0., 1.], [0.; 3])
        } else {
            let (quaternions, translations) = defaults.unwrap();
            let ordinal = index - roots;
            let quaternion = streams.read(quaternions.add(ordinal * 8)?, 8)?;
            let q = [0, 2, 4, 6].map(|offset| {
                (u16_at(quaternion, offset) as i16) as f32 * f32::from_bits(0x38000100)
            });
            let [x, y, z] = direction([q[0], q[1], q[2]]);
            let translation = direction(vector(
                streams.read(translations.add(ordinal * 12)?, 12)?,
                0,
            )?)
            .map(|v| v * 0.0254);
            ([x, y, z, q[3]], translation)
        };
        joints.push(Joint {
            name: joint_name,
            parent,
            bind,
            bind_local,
            inverse_bind,
            default_rotation,
            default_translation,
        });
    }
    Ok(joints)
}

pub(super) fn joint(offset: u16, bones: usize) -> Result<u16> {
    if offset % 64 != 0 || offset as usize / 64 >= bones {
        return Err(format!(
            "Native bone palette offset {offset} is invalid for {bones} model-local joints"
        )
        .into());
    }
    Ok(offset / 64)
}

pub(super) fn blended_bindings(
    counts: [usize; 4],
    bytes: &[u8],
    bones: usize,
    vertices: usize,
) -> Result<Vec<Influence>> {
    let count: usize = counts.iter().sum();
    let words: usize = counts
        .iter()
        .enumerate()
        .map(|(group, count)| count * (2 * group + 1))
        .sum();
    if count > vertices || bytes.len() != words * 2 {
        return Err("Native blend records exceed their raw vertex domain or byte span".into());
    }
    let mut result = Vec::with_capacity(count);
    let mut cursor = 0;
    for (group, count) in counts.into_iter().enumerate() {
        for _ in 0..count {
            let mut value = Influence {
                joints: [0; 4],
                weights: [0.; 4],
            };
            value.joints[0] = joint(u16_at(bytes, cursor), bones)?;
            cursor += 2;
            let mut remaining = 65536_u32;
            for lane in 1..=group {
                value.joints[lane] = joint(u16_at(bytes, cursor), bones)?;
                let weight = u16_at(bytes, cursor + 2) as u32;
                remaining = remaining
                    .checked_sub(weight)
                    .ok_or("Native explicit blend weights exceed one")?;
                value.weights[lane] = weight as f32 * f32::from_bits(0x37800000);
                cursor += 4;
            }
            value.weights[0] = remaining as f32 * f32::from_bits(0x37800000);
            result.push(value);
        }
    }
    Ok(result)
}

pub(super) fn rigid_bindings(
    bytes: &[u8],
    bones: usize,
    vertices: usize,
) -> Result<Vec<Influence>> {
    if bytes.len() % 12 != 0 {
        return Err("Misaligned native rigid vertex lists".into());
    }
    let mut result = Vec::new();
    for record in bytes.chunks_exact(12) {
        let bone = joint(u16_at(record, 0), bones)?;
        let end = result.len() + u16_at(record, 2) as usize;
        if end > vertices {
            return Err("Native rigid lists exceed their raw vertex domain".into());
        }
        result.resize(
            end,
            Influence {
                joints: [bone, 0, 0, 0],
                weights: [1., 0., 0., 0.],
            },
        );
    }
    Ok(result)
}

pub(super) fn bindings(
    streams: &Streams,
    surface: Address,
    bones: usize,
    vertices: usize,
    source: &[u16],
) -> Result<Bindings> {
    let flags = streams.u16(surface.add(2)?)?;
    let mut counts = [0; 4];
    for (group, count) in counts.iter_mut().enumerate() {
        let value = streams.u16(surface.add(16 + group * 2)?)? as i16;
        if value < 0 {
            return Err("Negative native blend vertex count".into());
        }
        *count = value as usize;
    }
    let words: usize = counts
        .iter()
        .enumerate()
        .map(|(group, count)| count * (2 * group + 1))
        .sum();
    let blend_bytes = if words == 0 {
        &[]
    } else {
        streams.read(pointer(streams, surface.add(24)?)?, words * 2)?
    };
    let blended = blended_bindings(counts, blend_bytes, bones, vertices)?;
    let lists = streams.u8(surface.add(1)?)? as usize;
    let rigid_bytes = if lists == 0 {
        &[]
    } else {
        streams.read(pointer(streams, surface.add(40)?)?, lists * 12)?
    };
    let rigid = rigid_bindings(rigid_bytes, bones, vertices)?;
    let result = Bindings {
        flags,
        blended: source
            .iter()
            .map(|&vertex| blended.get(vertex as usize).copied())
            .collect(),
        rigid: source
            .iter()
            .map(|&vertex| rigid.get(vertex as usize).copied())
            .collect(),
    };
    if result.selected().iter().any(Option::is_none) {
        return Err(format!("Native model vertex has no binding in its selected consumer (flags {flags:#x}, blended {}, rigid {}, raw vertices {vertices})",blended.len(),rigid.len()).into());
    }
    Ok(result)
}

pub fn model(
    catalog: &Catalog<'_>,
    name: &str,
    zone: usize,
    root: Address,
) -> Result<Vec<Vec<Surface>>> {
    let streams = &catalog.zones[zone].1.streams;
    let bones = streams.u8(root.add(4)?)? as usize;
    let surface_count = streams.u8(root.add(6)?)? as usize;
    let surfaces = pointer(streams, root.add(32)?)?;
    let materials = pointer(streams, root.add(36)?)?;
    let lod_count = streams.u16(root.add(196)?)? as usize;
    if lod_count == 0 || lod_count > 4 {
        return Err(format!("Model {name} has invalid LOD count {lod_count}").into());
    }
    let mut lods = Vec::new();
    for lod in 0..lod_count {
        let info = root.add(40 + lod * 28)?;
        let count = streams.u16(info.add(4)?)? as usize;
        let first = streams.u16(info.add(6)?)? as usize;
        if first + count > surface_count {
            return Err(format!("Model {name} LOD range exceeds surface array").into());
        }
        let mut meshes = Vec::new();
        for index in first..first + count {
            let surface = surfaces.add(index * 80)?;
            let vertices = streams.u16(surface.add(4)?)? as usize;
            let triangles = streams.u16(surface.add(6)?)? as usize;
            if vertices == 0 || triangles == 0 {
                continue;
            }
            let flags = streams.u16(surface.add(2)?)?;
            if flags & 1 != 0 {
                return Err(format!("Model {name} surface {index}: quantized vertex source is not recovered (flags {flags:#x})").into());
            }
            let material = catalog.pointer(zone, materials.add(index * 4)?, 6)?.0;
            let mut mesh = Surface::read(
                format!("{name}_surface{index}"),
                index,
                streams.read(pointer(streams, surface.add(32)?)?, vertices * 32)?,
                32,
                streams.read(pointer(streams, surface.add(12)?)?, triangles * 6)?,
                material,
            )?;
            mesh.bindings = Some(
                bindings(streams, surface, bones, vertices, &mesh.source_vertices)
                    .map_err(|error| format!("Model {name} surface {index} bindings: {error}"))?,
            );
            meshes.push(mesh);
        }
        lods.push(meshes);
    }
    Ok(lods)
}
