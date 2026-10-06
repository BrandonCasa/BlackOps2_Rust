//! Versioned, bounded external appearance input. This increment admits one UV/color preview.
use crate::geometry_snapshot::{GeometrySnapshot, read_geometry_snapshot};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt, fs,
    io::{self, Cursor},
    path::{Component, Path},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceError {
    Io(io::ErrorKind),
    Malformed,
    Unsupported,
    Reference,
    Identity,
    Geometry,
    Attributes,
    Limit,
}
impl fmt::Display for AppearanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "appearance {:?}", self)
    }
}
impl std::error::Error for AppearanceError {}
impl From<io::Error> for AppearanceError {
    fn from(e: io::Error) -> Self {
        Self::Io(e.kind())
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceManifest {
    pub version: u32,
    pub geometry: String,
    pub geometry_sha256: String,
    pub surface: u32,
    pub corner_uv: String,
    pub uv_encoding: String,
    pub material: ColorPreview,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ColorPreview {
    pub texture: String,
    pub color_space: String,
    pub sampler: u8,
    pub family: String,
}
pub struct MaterialSceneInput {
    pub geometry: GeometrySnapshot,
    pub surface: usize,
    pub corner_uv: Vec<[f32; 2]>,
    pub texture_dds: Vec<u8>,
    pub sampler: u8,
}
fn bounded_file(path: &Path, cap: u64) -> Result<Vec<u8>, AppearanceError> {
    let f = fs::File::open(path)?;
    let m = f.metadata()?;
    if !m.is_file() || m.len() > cap {
        return Err(AppearanceError::Limit);
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(m.len() as usize)
        .map_err(|_| AppearanceError::Limit)?;
    f.take(cap + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 != m.len() {
        return Err(AppearanceError::Limit);
    }
    Ok(bytes)
}
pub fn relative_dependency(
    root: &Path,
    reference: &str,
) -> Result<std::path::PathBuf, AppearanceError> {
    if reference.is_empty()
        || reference.contains('\\')
        || reference.contains(':')
        || Path::new(reference)
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
    {
        return Err(AppearanceError::Reference);
    }
    let path = root.join(reference).canonicalize()?;
    if !path.starts_with(root.canonicalize()?) {
        return Err(AppearanceError::Reference);
    }
    Ok(path)
}
pub fn decode_corner_uv(bytes: &[u8], count: usize) -> Result<Vec<[f32; 2]>, AppearanceError> {
    if count > 1_000_000 || count.checked_mul(8) != Some(bytes.len()) {
        return Err(AppearanceError::Attributes);
    }
    let mut out = Vec::new();
    out.try_reserve_exact(count)
        .map_err(|_| AppearanceError::Limit)?;
    for b in bytes.as_chunks::<8>().0 {
        let u = f32::from_le_bytes(
            b[0..4]
                .try_into()
                .map_err(|_| AppearanceError::Attributes)?,
        );
        let v = f32::from_le_bytes(
            b[4..8]
                .try_into()
                .map_err(|_| AppearanceError::Attributes)?,
        );
        if !u.is_finite() || !v.is_finite() {
            return Err(AppearanceError::Attributes);
        }
        out.push([u, v]);
    }
    Ok(out)
}
pub fn validate_manifest(m: &AppearanceManifest) -> Result<(), AppearanceError> {
    if m.version != 1
        || m.uv_encoding != "f32-le-corner-original-order"
        || m.material.family != "diagnostic-unlit-color"
        || m.material.color_space != "linear-diagnostic"
    {
        return Err(AppearanceError::Unsupported);
    }
    let filter = m.material.sampler & 7;
    let mip = (m.material.sampler >> 3) & 3;
    if !(1..=4).contains(&filter) || mip != 2 {
        return Err(AppearanceError::Unsupported);
    }
    if m.geometry_sha256.len() != 64
        || !m
            .geometry_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(AppearanceError::Identity);
    }
    Ok(())
}
pub fn load_material_scene(path: &Path) -> Result<MaterialSceneInput, AppearanceError> {
    let root = path.parent().ok_or(AppearanceError::Reference)?;
    let manifest: AppearanceManifest = serde_json::from_slice(&bounded_file(path, 16384)?)
        .map_err(|_| AppearanceError::Malformed)?;
    validate_manifest(&manifest)?;
    let geometry_bytes = bounded_file(
        &relative_dependency(root, &manifest.geometry)?,
        64 * 1024 * 1024,
    )?;
    let hash = format!("{:x}", Sha256::digest(&geometry_bytes));
    if hash != manifest.geometry_sha256 {
        return Err(AppearanceError::Identity);
    }
    let geometry = read_geometry_snapshot(
        &mut Cursor::new(&geometry_bytes),
        geometry_bytes.len() as u64,
    )
    .map_err(|_| AppearanceError::Geometry)?;
    let surface = manifest.surface as usize;
    let selected = geometry
        .surfaces()
        .get(surface)
        .ok_or(AppearanceError::Reference)?;
    let count = selected.index_range().len();
    if count == 0 {
        return Err(AppearanceError::Attributes);
    }
    let corner_uv = decode_corner_uv(
        &bounded_file(
            &relative_dependency(root, &manifest.corner_uv)?,
            8 * 1024 * 1024,
        )?,
        count,
    )?;
    let texture_dds = bounded_file(
        &relative_dependency(root, &manifest.material.texture)?,
        64 * 1024 * 1024,
    )?;
    validate_dds(&texture_dds)?;
    Ok(MaterialSceneInput {
        geometry,
        surface,
        corner_uv,
        texture_dds,
        sampler: manifest.material.sampler,
    })
}

/// Admit bounded 2D legacy DDS dimensions before Bevy allocates or uploads pixels.
pub fn validate_dds(bytes: &[u8]) -> Result<(), AppearanceError> {
    if bytes.len() < 128 || !bytes.starts_with(b"DDS ") {
        return Err(AppearanceError::Malformed);
    }
    let word = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    if word(4) != 124 || word(76) != 32 {
        return Err(AppearanceError::Malformed);
    }
    if word(16) == 0 || word(12) == 0 || word(16) > 4096 || word(12) > 4096 || word(28) > 13 {
        return Err(AppearanceError::Limit);
    }
    if word(24) > 1 || word(112) & 0x20fe00 != 0 || &bytes[84..88] == b"DX10" {
        return Err(AppearanceError::Unsupported);
    }
    Ok(())
}

pub(crate) fn decode_ieee_half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1. } else { -1. };
    let exponent = (bits >> 10) & 31;
    let mantissa = bits & 1023;
    if exponent == 0 {
        sign * (mantissa as f32) * 2f32.powi(-24)
    } else if exponent == 31 {
        if mantissa == 0 {
            sign * f32::INFINITY
        } else {
            f32::NAN
        }
    } else {
        sign * (1. + mantissa as f32 / 1024.) * 2f32.powi(exponent as i32 - 15)
    }
}
