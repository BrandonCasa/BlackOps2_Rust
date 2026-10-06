//! Safe external translated-shader metadata. Admission is not rendering fidelity.
use crate::appearance::{AppearanceError, relative_dependency};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShaderStage {
    Vertex,
    Fragment,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureDimension {
    D2,
    Cube,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureSample {
    Float,
    Depth,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResourceKind {
    Uniform {
        size: u32,
    },
    Texture {
        dimension: TextureDimension,
        sample: TextureSample,
    },
    Sampler {
        comparison: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderResource {
    pub binding: u32,
    pub kind: ResourceKind,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VertexInput {
    pub location: u32,
    pub components: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderDeclaration {
    pub source_sha256: String,
    pub file: String,
    pub sha256: String,
    pub stage: ShaderStage,
    pub entry_point: String,
    pub inputs: Vec<VertexInput>,
    pub resources: Vec<ShaderResource>,
}
pub struct ShaderInput {
    pub declaration: ShaderDeclaration,
    pub wgsl: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArgumentDestination {
    Uniform { buffer: u16, offset: u16, size: u16 },
    TextureSampler { texture: u8, sampler: u8 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArgumentSource {
    MaterialConstant {
        name_hash: u32,
    },
    MaterialTexture {
        name_hash: u32,
    },
    CodeConstant {
        index: u16,
        first_row: u8,
        row_count: u8,
    },
    CodeTexture {
        index: u32,
    },
    Literal {
        value: [f32; 4],
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassArgument {
    pub stage: ShaderStage,
    pub destination: ArgumentDestination,
    pub source: ArgumentSource,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderPass {
    pub vertex_shader: usize,
    pub pixel_shader: usize,
    pub routing: Vec<[u8; 2]>,
    pub args: Vec<PassArgument>,
    pub custom_sampler_flags: u8,
    pub material_type: u8,
    pub precompiled_index: u8,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Technique {
    pub index: u8,
    pub flags: u32,
    pub passes: Vec<ShaderPass>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TechniqueFamily {
    pub source_technique_set: String,
    pub techniques: Vec<Technique>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialConstant {
    pub name_hash: u32,
    pub value: [f32; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MaterialImage {
    SceneImage { index: usize },
    UnresolvedBuiltin { source_sha256: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialTextureContract {
    pub name_hash: u32,
    pub semantic: u8,
    pub sampler: u8,
    pub image: MaterialImage,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialContract {
    pub scene_material: usize,
    pub family: usize,
    pub constants: Vec<MaterialConstant>,
    #[serde(default)]
    pub textures: Vec<MaterialTextureContract>,
    pub states: Vec<[u32; 2]>,
    pub state_entries: Vec<u8>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderBankManifest {
    pub version: u32,
    pub geometry_sha256: String,
    pub scene_image_count: usize,
    pub shaders: Vec<ShaderDeclaration>,
    pub families: Vec<TechniqueFamily>,
    pub materials: Vec<MaterialContract>,
}
pub struct ShaderBankInput {
    pub geometry_sha256: String,
    pub scene_image_count: usize,
    pub shaders: Vec<ShaderInput>,
    pub families: Vec<TechniqueFamily>,
    pub materials: Vec<MaterialContract>,
}

fn hash_valid(h: &str) -> bool {
    h.len() == 64
        && h.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn read(path: &Path, cap: u64) -> Result<Vec<u8>, AppearanceError> {
    let f = fs::File::open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.len() > cap {
        return Err(AppearanceError::Limit);
    }
    let mut b = Vec::new();
    b.try_reserve_exact(m.len() as usize)
        .map_err(|_| AppearanceError::Limit)?;
    f.take(cap + 1).read_to_end(&mut b)?;
    if b.len() as u64 != m.len() {
        return Err(AppearanceError::Limit);
    }
    Ok(b)
}
pub fn validate_bank(m: &ShaderBankManifest) -> Result<(), AppearanceError> {
    use AppearanceError as E;
    if m.version != 1 {
        return Err(E::Unsupported);
    }
    if !hash_valid(&m.geometry_sha256) {
        return Err(E::Identity);
    }
    if m.shaders.len() > 4096
        || m.families.len() > 512
        || m.materials.len() > 20_000
        || m.scene_image_count > 20_000
    {
        return Err(E::Limit);
    }
    let mut hashes = BTreeSet::new();
    for s in &m.shaders {
        if !hash_valid(&s.source_sha256)
            || !hash_valid(&s.sha256)
            || !hashes.insert(&s.source_sha256)
        {
            return Err(E::Identity);
        }
        if s.entry_point.is_empty()
            || s.entry_point.len() > 64
            || !s
                .entry_point
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(E::Unsupported);
        }
        if s.inputs.len() > 32 || s.resources.len() > 96 {
            return Err(E::Limit);
        }
        let mut locations = BTreeSet::new();
        for v in &s.inputs {
            if v.location >= 32 || !(1..=4).contains(&v.components) || !locations.insert(v.location)
            {
                return Err(E::Attributes);
            }
        }
        let mut bindings = BTreeSet::new();
        for r in &s.resources {
            if !bindings.insert(r.binding) {
                return Err(E::Reference);
            }
            match r.kind {
                ResourceKind::Uniform { size } => {
                    let start = if s.stage == ShaderStage::Vertex {
                        0
                    } else {
                        16
                    };
                    if !(start..start + 16).contains(&r.binding)
                        || size == 0
                        || size > 65536
                        || size % 16 != 0
                    {
                        return Err(E::Unsupported);
                    }
                }
                ResourceKind::Texture { dimension, sample } => {
                    if !(32..64).contains(&r.binding)
                        || (dimension == TextureDimension::Cube && sample == TextureSample::Depth)
                    {
                        return Err(E::Unsupported);
                    }
                }
                ResourceKind::Sampler { .. } => {
                    if !(64..96).contains(&r.binding) {
                        return Err(E::Unsupported);
                    }
                }
            }
        }
    }
    let mut names = BTreeSet::new();
    for f in &m.families {
        if f.source_technique_set.is_empty()
            || f.source_technique_set.len() > 256
            || !names.insert(&f.source_technique_set)
            || f.techniques.len() > 36
        {
            return Err(E::Limit);
        }
        let mut indexes = BTreeSet::new();
        for t in &f.techniques {
            if t.index >= 36 || !indexes.insert(t.index) || t.passes.len() > 4 {
                return Err(E::Reference);
            }
            for p in &t.passes {
                let vs = m.shaders.get(p.vertex_shader).ok_or(E::Reference)?;
                let ps = m.shaders.get(p.pixel_shader).ok_or(E::Reference)?;
                if vs.stage != ShaderStage::Vertex || ps.stage != ShaderStage::Fragment {
                    return Err(E::Unsupported);
                }
                if p.routing.len() > 16 || p.args.len() > 128 {
                    return Err(E::Limit);
                }
                for a in &p.args {
                    match a.destination {
                        ArgumentDestination::Uniform {
                            buffer,
                            offset,
                            size,
                        } => {
                            if buffer >= 16
                                || size == 0
                                || offset % 4 != 0
                                || size % 4 != 0
                                || u32::from(offset) + u32::from(size) > 65536
                            {
                                return Err(E::Attributes);
                            }
                            if matches!(
                                a.source,
                                ArgumentSource::MaterialTexture { .. }
                                    | ArgumentSource::CodeTexture { .. }
                            ) {
                                return Err(E::Unsupported);
                            }
                        }
                        ArgumentDestination::TextureSampler { texture, sampler } => {
                            if a.stage != ShaderStage::Fragment
                                || texture >= 32
                                || sampler >= 32
                                || !matches!(
                                    a.source,
                                    ArgumentSource::MaterialTexture { .. }
                                        | ArgumentSource::CodeTexture { .. }
                                )
                            {
                                return Err(E::Unsupported);
                            }
                        }
                    }
                    match a.source {
                        ArgumentSource::CodeConstant { row_count: 0, .. } => {
                            return Err(E::Attributes);
                        }
                        ArgumentSource::Literal { value }
                            if !value.iter().all(|f| f.is_finite()) =>
                        {
                            return Err(E::Attributes);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    let mut material_indexes = BTreeSet::new();
    for material in &m.materials {
        if material.family >= m.families.len()
            || material.scene_material >= m.materials.len()
            || !material_indexes.insert(material.scene_material)
        {
            return Err(E::Reference);
        }
        if material.constants.len() > 64
            || material.textures.len() > 32
            || material.states.len() > 128
            || material.state_entries.len() != 36
        {
            return Err(E::Limit);
        }
        for c in &material.constants {
            if !c.value.iter().all(|f| f.is_finite()) {
                return Err(E::Attributes);
            }
        }
        for t in &material.textures {
            if !(1..=4).contains(&(t.sampler & 7)) || ((t.sampler >> 3) & 3) > 2 {
                return Err(E::Unsupported);
            }
            match &t.image {
                MaterialImage::SceneImage { index } if *index >= m.scene_image_count => {
                    return Err(E::Reference);
                }
                MaterialImage::UnresolvedBuiltin { source_sha256 }
                    if !hash_valid(source_sha256) =>
                {
                    return Err(E::Identity);
                }
                _ => {}
            }
        }
        for t in &m.families[material.family].techniques {
            let state = material.state_entries[t.index as usize];
            if state != 255 && state as usize >= material.states.len() {
                return Err(E::Reference);
            }
        }
    }
    Ok(())
}
pub fn load_shader_bank(path: &Path) -> Result<ShaderBankInput, AppearanceError> {
    let root = path.parent().ok_or(AppearanceError::Reference)?;
    let m: ShaderBankManifest = serde_json::from_slice(&read(path, 32 * 1024 * 1024)?)
        .map_err(|_| AppearanceError::Malformed)?;
    validate_bank(&m)?;
    let mut shaders = Vec::new();
    let mut retained = 0u64;
    for declaration in m.shaders {
        let bytes = read(
            &relative_dependency(root, &declaration.file)?,
            (64 * 1024 * 1024 - retained).min(1024 * 1024),
        )?;
        retained += bytes.len() as u64;
        if format!("{:x}", Sha256::digest(&bytes)) != declaration.sha256 {
            return Err(AppearanceError::Identity);
        }
        if bytes.is_empty() {
            return Err(AppearanceError::Malformed);
        }
        let wgsl = String::from_utf8(bytes).map_err(|_| AppearanceError::Malformed)?;
        shaders.push(ShaderInput { declaration, wgsl });
    }
    Ok(ShaderBankInput {
        geometry_sha256: m.geometry_sha256,
        scene_image_count: m.scene_image_count,
        shaders,
        families: m.families,
        materials: m.materials,
    })
}
