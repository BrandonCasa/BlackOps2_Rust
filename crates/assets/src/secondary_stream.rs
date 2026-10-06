//! Exact secondary world inputs. Source corner order and input semantics are retained.
use crate::appearance::decode_ieee_half as half;
use crate::{
    appearance::AppearanceError,
    full_scene::{SceneImage, identified},
    geometry_snapshot::GeometrySnapshot,
};
use serde::{Deserialize, Serialize};
use std::{fs, io::Read, path::Path};

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecondaryManifest {
    pub version: u32,
    pub geometry_sha256: String,
    pub stream: SceneImage,
    /// Original uint16 triangle-local vertex indexes, one per admitted corner.
    pub corner_local_indices: SceneImage,
    pub surfaces: Vec<SecondarySurface>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecondarySurface {
    pub first_corner: usize,
    pub corners: usize,
    pub source_format: u32,
    pub byte_offset: u64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SourceTexcoord {
    Float2([f32; 2]),
    Float4([f32; 4]),
}
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct SourceSecondaryCorner {
    pub texcoord3: Option<SourceTexcoord>,
    pub texcoord4: Option<[f32; 2]>,
    pub color5: Option<[f32; 4]>,
    pub color6: Option<[f32; 4]>,
}
/// Layouts qualified by actual original D3D input-layout captures.
pub fn secondary_stride(format: u32) -> Result<usize, AppearanceError> {
    match format {
        0 => Ok(0),
        1 => Ok(4),
        2 | 3 => Ok(8),
        4 => Ok(12),
        5 | 7 => Ok(16),
        _ => Err(AppearanceError::Unsupported),
    }
}
/// Decode a qualified single source record without shader-specific interpretation.
pub fn decode_secondary_record(
    format: u32,
    bytes: &[u8],
) -> Result<SourceSecondaryCorner, AppearanceError> {
    use AppearanceError as E;
    if bytes.len() != secondary_stride(format)? {
        return Err(E::Attributes);
    }
    if format == 0 {
        return Ok(SourceSecondaryCorner::default());
    }
    let f = |o| half(u16::from_le_bytes([bytes[o], bytes[o + 1]]));
    let pair = |o| [f(o), f(o + 2)];
    let color = |o| {
        [
            bytes[o] as f32 / 255.,
            bytes[o + 1] as f32 / 255.,
            bytes[o + 2] as f32 / 255.,
            bytes[o + 3] as f32 / 255.,
        ]
    };
    let four = [
        f(0),
        f(2),
        if format >= 3 { f(4) } else { 0. },
        if format >= 3 { f(6) } else { 0. },
    ];
    if !four.iter().all(|v| v.is_finite()) {
        return Err(E::Attributes);
    }
    let mut result = SourceSecondaryCorner {
        texcoord3: Some(if format < 3 {
            SourceTexcoord::Float2(pair(0))
        } else {
            SourceTexcoord::Float4(four)
        }),
        ..Default::default()
    };
    match format {
        2 => result.color5 = Some(color(4)),
        4 => result.color5 = Some(color(8)),
        5 => {
            result.color5 = Some(color(8));
            result.color6 = Some(color(12));
        }
        7 => {
            let uv = pair(8);
            if !uv.iter().all(|v| v.is_finite()) {
                return Err(E::Attributes);
            }
            result.texcoord4 = Some(uv);
            result.color5 = Some(color(12));
        }
        _ => {}
    }
    Ok(result)
}
/// Strict admission: every surface must have a qualified secondary layout.
pub fn decode_secondary_corners(
    manifest: &SecondaryManifest,
    stream: &[u8],
    local_indices: &[u8],
    geometry: &GeometrySnapshot,
) -> Result<Vec<SourceSecondaryCorner>, AppearanceError> {
    use AppearanceError as E;
    if manifest.version != 1 {
        return Err(E::Unsupported);
    }
    if manifest.surfaces.len() != geometry.surfaces().len()
        || geometry.indices().len().checked_mul(2) != Some(local_indices.len())
    {
        return Err(E::Attributes);
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(geometry.indices().len())
        .map_err(|_| E::Limit)?;
    for (s, g) in manifest.surfaces.iter().zip(geometry.surfaces()) {
        let range = g.index_range();
        if s.first_corner != range.start
            || s.corners != range.len()
            || s.source_format != g.source_format_category()
        {
            return Err(E::Attributes);
        }
        let stride = secondary_stride(s.source_format)?;
        let offset = usize::try_from(s.byte_offset).map_err(|_| E::Limit)?;
        for corner in range {
            if stride == 0 {
                result.push(SourceSecondaryCorner::default());
                continue;
            }
            let local =
                u16::from_le_bytes([local_indices[corner * 2], local_indices[corner * 2 + 1]])
                    as usize;
            let start = offset
                .checked_add(local.checked_mul(stride).ok_or(E::Limit)?)
                .ok_or(E::Limit)?;
            let end = start.checked_add(stride).ok_or(E::Limit)?;
            result.push(decode_secondary_record(
                s.source_format,
                stream.get(start..end).ok_or(E::Attributes)?,
            )?);
        }
    }
    Ok(result)
}
/// Load only SHA256-bound external resources, tied to an already admitted snapshot identity.
pub fn load_secondary_corners(
    path: &Path,
    geometry: &GeometrySnapshot,
    geometry_sha256: &str,
) -> Result<Vec<SourceSecondaryCorner>, AppearanceError> {
    use AppearanceError as E;
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > 16 * 1024 * 1024 {
        return Err(E::Limit);
    }
    let mut bytes = Vec::new();
    file.take(16 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(E::Limit);
    }
    let m: SecondaryManifest = serde_json::from_slice(&bytes).map_err(|_| E::Malformed)?;
    if m.geometry_sha256 != geometry_sha256
        || geometry_sha256.len() != 64
        || !geometry_sha256.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(E::Identity);
    }
    let root = path.parent().ok_or(E::Malformed)?;
    let stream = identified(root, &m.stream.file, &m.stream.sha256, 64 * 1024 * 1024)?;
    let locals = identified(
        root,
        &m.corner_local_indices.file,
        &m.corner_local_indices.sha256,
        2 * 1_000_000,
    )?;
    decode_secondary_corners(&m, &stream, &locals, geometry)
}
