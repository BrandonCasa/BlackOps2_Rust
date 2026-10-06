//! Bounded version two appearance exchange. Original contracts remain explicit.
use crate::{
    appearance::{AppearanceError, decode_corner_uv, relative_dependency},
    geometry_snapshot::{GeometrySnapshot, read_geometry_snapshot},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Read},
    path::Path,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullSceneManifest {
    pub version: u32,
    pub geometry: String,
    pub geometry_sha256: String,
    pub corner_uv: String,
    pub corner_uv_sha256: String,
    pub uv_encoding: String,
    /// Optional proven D3D11 world stream, source corner order (36 bytes each).
    pub corner_vertices: Option<SceneImage>,
    pub images: Vec<SceneImage>,
    pub materials: Vec<SceneMaterial>,
    #[serde(default)]
    pub lightmaps: Vec<SceneLightmap>,
    #[serde(default)]
    pub probes: Vec<SceneProbe>,
    pub surfaces: Vec<SceneSurface>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneImage {
    pub file: String,
    pub sha256: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneLightmap {
    pub primary: Option<usize>,
    pub secondary: Option<usize>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneProbe {
    pub image: usize,
    pub origin: [f32; 3],
    pub mip_bias: f32,
    pub sh: [f32; 12],
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneBinding {
    pub semantic: u8,
    pub name_hash: u32,
    pub image: usize,
    pub sampler: u8,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneMaterial {
    pub source_technique_set: String,
    pub contract: String,
    pub bindings: Vec<SceneBinding>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneSurface {
    pub material: usize,
    pub source_vertex_format: u32,
    /// Original lighting slots, not indices into the shared DDS image table.
    pub lightmap: Option<usize>,
    pub probe: Option<usize>,
}
pub struct SceneImageInput {
    pub dds: Vec<u8>,
    pub metadata: DdsMetadata,
}
pub struct FullSceneInput {
    pub geometry: GeometrySnapshot,
    /// In snapshot surface/index order; source coordinates and UVs are unchanged.
    pub corner_uv: Vec<[f32; 2]>,
    pub corner_attributes: Option<Vec<SourceCornerAttributes>>,
    pub images: Vec<SceneImageInput>,
    pub materials: Vec<SceneMaterial>,
    pub lightmaps: Vec<SceneLightmap>,
    pub probes: Vec<SceneProbe>,
    pub surfaces: Vec<SceneSurface>,
}
/// Exact normalized vertex-input channels, before shader-specific unpacking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceCornerAttributes {
    pub position_w: f32,
    pub color: [f32; 4],
    pub normal_unorm: [f32; 4],
    pub tangent_unorm: [f32; 4],
    pub lightmap_uv: [f32; 2],
}
pub fn decode_world_corner_attributes(
    bytes: &[u8],
    geometry: &GeometrySnapshot,
) -> Result<Vec<SourceCornerAttributes>, AppearanceError> {
    if geometry.indices().len().checked_mul(36) != Some(bytes.len()) {
        return Err(AppearanceError::Attributes);
    }
    let mut attributes = Vec::new();
    attributes
        .try_reserve_exact(geometry.indices().len())
        .map_err(|_| AppearanceError::Limit)?;
    for (b, index) in bytes.as_chunks::<36>().0.iter().zip(geometry.indices()) {
        let word = |o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        let p = [
            f32::from_bits(word(0)),
            f32::from_bits(word(4)),
            f32::from_bits(word(8)),
        ];
        if p != geometry.positions()[*index as usize] {
            return Err(AppearanceError::Attributes);
        }
        let packed = |v: u32| {
            [
                (v & 1023) as f32 / 1023.,
                ((v >> 10) & 1023) as f32 / 1023.,
                ((v >> 20) & 1023) as f32 / 1023.,
                (v >> 30) as f32 / 3.,
            ]
        };
        let half = |o: usize| u16::from_le_bytes([b[o], b[o + 1]]) as f32 / 65535.;
        let position_w = f32::from_bits(word(12));
        if !position_w.is_finite() {
            return Err(AppearanceError::Attributes);
        }
        attributes.push(SourceCornerAttributes {
            position_w,
            color: [
                b[16] as f32 / 255.,
                b[17] as f32 / 255.,
                b[18] as f32 / 255.,
                b[19] as f32 / 255.,
            ],
            normal_unorm: packed(word(24)),
            tangent_unorm: packed(word(28)),
            lightmap_uv: [half(32), half(34)],
        });
    }
    Ok(attributes)
}
/// Cross-check independently exported UVs against the captured hardware half input.
pub fn validate_world_corner_uv(bytes: &[u8], uv: &[[f32; 2]]) -> Result<(), AppearanceError> {
    if uv.len().checked_mul(36) != Some(bytes.len()) {
        return Err(AppearanceError::Attributes);
    }
    for (record, uv) in bytes.as_chunks::<36>().0.iter().zip(uv) {
        let actual = [20, 22].map(|offset| {
            crate::appearance::decode_ieee_half(u16::from_le_bytes([
                record[offset],
                record[offset + 1],
            ]))
        });
        if actual
            .into_iter()
            .zip(uv)
            .any(|(a, b)| a.to_bits() != b.to_bits())
        {
            return Err(AppearanceError::Attributes);
        }
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DdsFormat {
    Bc1,
    Bc2,
    Bc3,
    Bc5,
    Rgba8,
    Bgra8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DdsMetadata {
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub cube: bool,
    pub format: DdsFormat,
}

/// Admit complete legacy mip payloads, including all six faces of a cube.
pub fn inspect_dds(bytes: &[u8]) -> Result<DdsMetadata, AppearanceError> {
    use AppearanceError as E;
    if bytes.len() < 128 || &bytes[..4] != b"DDS " {
        return Err(E::Malformed);
    }
    let word = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    if word(4) != 124 || word(76) != 32 {
        return Err(E::Malformed);
    }
    let (width, height) = (word(16), word(12));
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(E::Limit);
    }
    let mips = word(28).max(1);
    if mips > 32 - width.max(height).leading_zeros() {
        return Err(E::Malformed);
    }
    let caps = word(112);
    let cube = caps & 0x200 != 0;
    if cube && width != height {
        return Err(E::Malformed);
    }
    if word(24) > 1
        || caps & 0x200000 != 0
        || caps & !0xfe00 != 0
        || (cube && caps & 0xfc00 != 0xfc00)
        || (!cube && caps != 0)
    {
        return Err(E::Unsupported);
    }
    let mut header = 128;
    let (format, block) = match &bytes[84..88] {
        b"DX10" if bytes.len() >= 148 => {
            header = 148;
            if word(132) != 3
                || word(140) != 1
                || word(136) & !4 != 0
                || ((word(136) & 4 != 0) != cube)
            {
                return Err(E::Unsupported);
            }
            match word(128) {
                71 => (DdsFormat::Bc1, 8usize),
                74 => (DdsFormat::Bc2, 16),
                77 => (DdsFormat::Bc3, 16),
                83 => (DdsFormat::Bc5, 16),
                28 => (DdsFormat::Rgba8, 0),
                _ => return Err(E::Unsupported),
            }
        }
        b"DXT1" => (DdsFormat::Bc1, 8usize),
        b"DXT3" => (DdsFormat::Bc2, 16),
        b"DXT5" => (DdsFormat::Bc3, 16),
        b"ATI2" | b"BC5U" => (DdsFormat::Bc5, 16),
        [0, 0, 0, 0]
            if word(80) & 0x40 != 0
                && word(88) == 32
                && word(100) == 0xff0000
                && word(104) == 0xff000000
                && word(92) == 0xff
                && word(96) == 0xff00 =>
        {
            (DdsFormat::Rgba8, 0)
        }
        [0, 0, 0, 0]
            if word(80) & 0x40 != 0
                && word(88) == 32
                && word(92) == 0xff0000
                && word(96) == 0xff00
                && word(100) == 0xff
                && word(104) == 0xff000000 =>
        {
            (DdsFormat::Bgra8, 0)
        }
        _ => return Err(E::Unsupported),
    };
    let mut size = 0usize;
    for mip in 0..mips {
        let w = (width >> mip).max(1) as usize;
        let h = (height >> mip).max(1) as usize;
        size = size
            .checked_add(if block == 0 {
                w * h * 4
            } else {
                w.div_ceil(4) * h.div_ceil(4) * block
            })
            .ok_or(E::Limit)?;
    }
    size = size
        .checked_mul(if cube { 6 } else { 1 })
        .and_then(|n| n.checked_add(header))
        .ok_or(E::Limit)?;
    if size != bytes.len() {
        return Err(E::Malformed);
    }
    Ok(DdsMetadata {
        width,
        height,
        mips,
        cube,
        format,
    })
}
fn read(path: &Path, cap: u64) -> Result<Vec<u8>, AppearanceError> {
    let f = fs::File::open(path)?;
    let n = f.metadata()?;
    if !n.is_file() || n.len() > cap {
        return Err(AppearanceError::Limit);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(n.len() as usize)
        .map_err(|_| AppearanceError::Limit)?;
    f.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != n.len() {
        return Err(AppearanceError::Limit);
    }
    Ok(bytes)
}
pub(crate) fn identified(
    root: &Path,
    file: &str,
    hash: &str,
    cap: u64,
) -> Result<Vec<u8>, AppearanceError> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(AppearanceError::Identity);
    }
    let b = read(&relative_dependency(root, file)?, cap)?;
    if format!("{:x}", Sha256::digest(&b)) != hash {
        return Err(AppearanceError::Identity);
    }
    Ok(b)
}
pub fn load_full_scene(path: &Path) -> Result<FullSceneInput, AppearanceError> {
    use AppearanceError as E;
    let root = path.parent().ok_or(E::Reference)?;
    let m: FullSceneManifest =
        serde_json::from_slice(&read(path, 16 * 1024 * 1024)?).map_err(|_| E::Malformed)?;
    if m.version != 2 || m.uv_encoding != "f32-le-corner-original-order" {
        return Err(E::Unsupported);
    }
    if m.surfaces.len() > 100_000 || m.images.len() > 20_000 || m.materials.len() > 20_000 {
        return Err(E::Limit);
    }
    if m.lightmaps.len() > 256 || m.probes.len() > 256 {
        return Err(E::Limit);
    }
    for l in &m.lightmaps {
        if l.primary.is_some_and(|n| n >= m.images.len())
            || l.secondary.is_some_and(|n| n >= m.images.len())
        {
            return Err(E::Reference);
        }
    }
    for p in &m.probes {
        if p.image >= m.images.len() {
            return Err(E::Reference);
        }
        if !p
            .origin
            .iter()
            .chain(&p.sh)
            .chain(std::iter::once(&p.mip_bias))
            .all(|f| f.is_finite())
        {
            return Err(E::Attributes);
        }
    }
    let b = identified(root, &m.geometry, &m.geometry_sha256, 64 * 1024 * 1024)?;
    let geometry =
        read_geometry_snapshot(&mut Cursor::new(&b), b.len() as u64).map_err(|_| E::Geometry)?;
    if m.surfaces.len() != geometry.surfaces().len() {
        return Err(E::Attributes);
    }
    for (s, g) in m.surfaces.iter().zip(geometry.surfaces()) {
        if s.material >= m.materials.len()
            || s.lightmap.is_some_and(|n| n >= 256)
            || s.probe.is_some_and(|n| n >= 256)
        {
            return Err(E::Reference);
        }
        if s.source_vertex_format != g.source_format_category() {
            return Err(E::Attributes);
        }
    }
    for material in &m.materials {
        if material.source_technique_set.is_empty()
            || material.source_technique_set.len() > 256
            || material.bindings.len() > 32
        {
            return Err(E::Limit);
        }
        // Original shaded contract admission awaits runtime attributes/globals.
        if material.contract != "diagnostic-unlit-color" {
            return Err(E::Unsupported);
        }
        for binding in &material.bindings {
            if binding.image >= m.images.len() {
                return Err(E::Reference);
            }
            if !(1..=4).contains(&(binding.sampler & 7)) || ((binding.sampler >> 3) & 3) > 2 {
                return Err(E::Unsupported);
            }
        }
    }
    let count = geometry.indices().len();
    let corner_uv = decode_corner_uv(
        &identified(root, &m.corner_uv, &m.corner_uv_sha256, 8 * 1024 * 1024)?,
        count,
    )?;
    let corner_attributes = if let Some(vertices) = &m.corner_vertices {
        let bytes = identified(root, &vertices.file, &vertices.sha256, 36 * 1_000_000)?;
        validate_world_corner_uv(&bytes, &corner_uv)?;
        Some(decode_world_corner_attributes(&bytes, &geometry)?)
    } else {
        None
    };
    let mut images = Vec::new();
    let mut retained = 0usize;
    for image in &m.images {
        let cap = (1024usize * 1024 * 1024 - retained).min(128 * 1024 * 1024) as u64;
        let dds = identified(root, &image.file, &image.sha256, cap)?;
        retained = retained.checked_add(dds.len()).ok_or(E::Limit)?;
        let metadata = inspect_dds(&dds)?;
        images.push(SceneImageInput { dds, metadata });
    }
    Ok(FullSceneInput {
        geometry,
        corner_uv,
        corner_attributes,
        images,
        materials: m.materials,
        lightmaps: m.lightmaps,
        probes: m.probes,
        surfaces: m.surfaces,
    })
}
