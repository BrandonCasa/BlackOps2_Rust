//! Hash-bound replay inputs. Admission proves binding identity, not final-frame fidelity.
use crate::{
    appearance::AppearanceError as E,
    full_scene::{FullSceneInput, identified},
    shader_bank::{ResourceKind, ShaderBankInput, ShaderStage},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payload {
    pub file: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameManifest {
    pub version: u32,
    pub capture_sha256: String,
    pub geometry_sha256: String,
    pub shader_bank_sha256: String,
    pub source_corners: Payload,
    pub source_secondary: Payload,
    pub draws: Vec<DrawReceipt>,
    pub initial_attachments: Vec<InitialAttachment>,
    pub camera_origin: [f32; 3],
    pub camera_view_projection: [f32; 16],
    /// Capture event IDs which produce required attachments or sampled images.
    pub producer_dependencies: Vec<u32>,
    /// Non-draw operations remain outside the world-dispatch admission.
    pub excluded_operations: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InitialAttachment {
    pub identity: String,
    pub before_event: u32,
    pub width: u32,
    pub height: u32,
    pub view_format: String,
    pub samples: Vec<Payload>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DrawReceipt {
    pub event: u32,
    pub dispatch: Dispatch,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Dispatch {
    World { draw: Box<WorldDraw> },
    WorldUnsupported { reason: String },
    NonWorld { reason: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurfaceSlice {
    pub surface: usize,
    pub first_corner: usize,
    pub corners: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UniformBinding {
    pub stage: ShaderStage,
    pub register: u8,
    pub bytes: Payload,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureBinding {
    pub stage: ShaderStage,
    pub register: u8,
    pub bytes: Payload,
    pub width: u32,
    pub height: u32,
    pub mips: u32,
    pub cube: bool,
    /// Original typed view format, independent of the backing resource format.
    pub view_format: String,
    pub first_mip: u32,
    pub view_mips: u32,
    pub first_slice: u32,
    pub slices: u32,
    /// Zero means immutable; otherwise records the last producer event.
    pub producer_event: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplerBinding {
    pub stage: ShaderStage,
    pub register: u8,
    pub address: [u32; 3],
    pub filter: [u32; 4],
    pub comparison: u32,
    pub max_anisotropy: f32,
    pub min_lod: f32,
    pub max_lod: f32,
    pub mip_bias: f32,
    pub border: [f32; 4],
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub identity: String,
    pub view_format: String,
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    pub producer_event: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineState {
    pub viewport: [f32; 6],
    pub scissor: [i32; 4],
    pub scissor_enabled: bool,
    pub cull: u32,
    pub front_ccw: bool,
    pub depth_clip: bool,
    pub depth_bias: i32,
    pub slope_bias: f32,
    pub bias_clamp: f32,
    pub alpha_to_coverage: bool,
    pub independent_blend: bool,
    pub conservative_raster: u32,
    pub forced_samples: u32,
    pub multisample: bool,
    pub antialiased_lines: bool,
    pub sample_mask: u32,
    pub depth_enabled: bool,
    pub depth_write: bool,
    pub depth_compare: u32,
    pub stencil_enabled: bool,
    pub stencil_reference: u32,
    pub stencil_read_mask: u32,
    pub stencil_write_mask: u32,
    /// fail, depth-fail, pass, comparison for front and back faces.
    pub stencil: [[u32; 4]; 2],
    pub blend_enabled: bool,
    pub blend_color: [u32; 3],
    pub blend_alpha: [u32; 3],
    pub blend_constant: [f32; 4],
    pub write_mask: u8,
    pub color: Attachment,
    pub depth: Option<Attachment>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldDraw {
    pub slices: Vec<SurfaceSlice>,
    pub runtime_corners: Payload,
    pub runtime_secondary: Option<Payload>,
    pub vertex_shader_sha256: String,
    pub fragment_shader_sha256: String,
    pub uniforms: Vec<UniformBinding>,
    pub textures: Vec<TextureBinding>,
    pub samplers: Vec<SamplerBinding>,
    pub pipeline: PipelineState,
    /// Row-major source object matrix, before coordinate adaptation.
    pub object_matrix: [f32; 16],
    /// Physical source b-register and byte offset from which this matrix was read.
    pub object_register: u8,
    pub object_offset: usize,
}
pub struct CapturedFrameInput {
    pub manifest: FrameManifest,
    /// Payloads ordered by draw, then uniforms/textures. Engine-independent bytes.
    pub draws: Vec<LoadedWorldDraw>,
    pub initial_attachments: Vec<Vec<Arc<[u8]>>>,
}
pub struct LoadedWorldDraw {
    pub event: u32,
    pub uniforms: Vec<Arc<[u8]>>,
    pub textures: Vec<Arc<[u8]>>,
}

fn hash(h: &str) -> bool {
    h.len() == 64
        && h.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn payload(root: &Path, p: &Payload, cap: u64) -> Result<Vec<u8>, E> {
    identified(root, &p.file, &p.sha256, cap)
}
fn cached(
    root: &Path,
    p: &Payload,
    cap: u64,
    cache: &mut BTreeMap<String, (String, Arc<[u8]>)>,
    retained: &mut usize,
) -> Result<Arc<[u8]>, E> {
    // Check each declared path even when its content hash has already been loaded.
    crate::appearance::relative_dependency(root, &p.file)?;
    if let Some((file, bytes)) = cache.get(&p.sha256) {
        if file != &p.file {
            payload(root, p, cap)?;
        }
        if bytes.len() as u64 > cap {
            return Err(E::Limit);
        }
        return Ok(Arc::clone(bytes));
    }
    let bytes: Arc<[u8]> = payload(root, p, cap)?.into();
    *retained = retained.checked_add(bytes.len()).ok_or(E::Limit)?;
    if *retained > 1024 * 1024 * 1024 {
        return Err(E::Limit);
    }
    cache.insert(p.sha256.clone(), (p.file.clone(), Arc::clone(&bytes)));
    Ok(bytes)
}
fn mapped(stage: ShaderStage, register: u8, base: u32) -> u32 {
    base + u32::from(register)
        + if base == 0 && stage == ShaderStage::Fragment {
            16
        } else {
            0
        }
}

/// The supplied bank hash binds the already validated bank to its external manifest.
pub fn load_captured_frame(
    path: &Path,
    scene: &FullSceneInput,
    bank: &ShaderBankInput,
    bank_sha256: &str,
) -> Result<CapturedFrameInput, E> {
    let root = path.parent().ok_or(E::Reference)?;
    let size = std::fs::metadata(path)?.len();
    if size > 32 * 1024 * 1024 {
        return Err(E::Limit);
    }
    let bytes = std::fs::read(path)?;
    let m: FrameManifest = serde_json::from_slice(&bytes).map_err(|_| E::Malformed)?;
    if m.version != 1 {
        return Err(E::Unsupported);
    }
    if !hash(&m.capture_sha256)
        || !hash(&m.shader_bank_sha256)
        || m.geometry_sha256 != bank.geometry_sha256
        || m.shader_bank_sha256 != bank_sha256
    {
        return Err(E::Identity);
    }
    if m.draws.len() > 20000
        || m.producer_dependencies.len() > 20000
        || m.excluded_operations.len() > 20000
    {
        return Err(E::Limit);
    }
    let source = payload(root, &m.source_corners, 128 * 1024 * 1024)?;
    if source.len() != scene.geometry.indices().len() * 36 {
        return Err(E::Attributes);
    }
    if m.camera_origin
        .iter()
        .chain(m.camera_view_projection.iter())
        .any(|f| !f.is_finite())
        || m.initial_attachments.len() > 8
    {
        return Err(E::Malformed);
    }
    let mut cache = BTreeMap::<String, (String, Arc<[u8]>)>::new();
    let mut retained = source.len();
    let mut initial_attachments = Vec::new();
    let mut initial_ids = BTreeSet::new();
    for a in &m.initial_attachments {
        if !initial_ids.insert(&a.identity)
            || ![1, 2, 4, 8].contains(&a.samples.len())
            || a.width == 0
            || a.height == 0
            || a.width > 16384
            || a.height > 16384
        {
            return Err(E::Reference);
        }
        let pixel_bytes = match a.view_format.as_str() {
            "R8G8B8A8_UNORM" => 4,
            "D32S8" => 8,
            _ => return Err(E::Unsupported),
        };
        let expected = (a.width as usize)
            .checked_mul(a.height as usize)
            .and_then(|n| n.checked_mul(pixel_bytes))
            .ok_or(E::Limit)?;
        let mut samples = Vec::new();
        for p in &a.samples {
            let b = cached(root, p, 256 * 1024 * 1024, &mut cache, &mut retained)?;
            if b.len() != expected {
                return Err(E::Attributes);
            }
            samples.push(b);
        }
        initial_attachments.push(samples);
    }
    let mut previous = 0;
    let mut loaded = Vec::new();
    let decoded = crate::full_scene::decode_world_corner_attributes(&source, &scene.geometry)?;
    if scene.corner_attributes.as_ref() != Some(&decoded) {
        return Err(E::Identity);
    }
    crate::full_scene::validate_world_corner_uv(&source, &scene.corner_uv)?;
    let secondary_path = crate::appearance::relative_dependency(root, &m.source_secondary.file)?;
    payload(root, &m.source_secondary, 16 * 1024 * 1024)?;
    let secondary = crate::secondary_stream::load_secondary_corners(
        &secondary_path,
        &scene.geometry,
        &m.geometry_sha256,
    )?;
    let producers: BTreeSet<_> = m.producer_dependencies.iter().copied().collect();
    if producers.len() != m.producer_dependencies.len() || producers.contains(&0) {
        return Err(E::Reference);
    }
    for receipt in &m.draws {
        if receipt.event <= previous {
            return Err(E::Reference);
        }
        previous = receipt.event;
        let Dispatch::World { draw: d } = &receipt.dispatch else {
            continue;
        };
        if d.slices.is_empty()
            || d.slices.len() > 6469
            || d.uniforms.len() > 28
            || d.textures.len() > 256
            || d.samplers.len() > 32
        {
            return Err(E::Limit);
        }
        if d.object_matrix
            .iter()
            .chain(d.pipeline.viewport.iter())
            .any(|f| !f.is_finite())
            || d.pipeline.viewport[2] <= 0.0
            || d.pipeline.viewport[3] <= 0.0
            || d.pipeline.viewport[4] > d.pipeline.viewport[5]
        {
            return Err(E::Malformed);
        }
        for attachment in std::iter::once(&d.pipeline.color).chain(d.pipeline.depth.iter()) {
            let initial = m
                .initial_attachments
                .iter()
                .find(|a| a.identity == attachment.identity)
                .ok_or(E::Reference)?;
            if initial.before_event > receipt.event
                || (
                    initial.width,
                    initial.height,
                    initial.samples.len(),
                    &initial.view_format,
                ) != (
                    attachment.width,
                    attachment.height,
                    attachment.samples as usize,
                    &attachment.view_format,
                )
            {
                return Err(E::Attributes);
            }
            if attachment.producer_event > 0
                && (attachment.producer_event >= receipt.event
                    || !producers.contains(&attachment.producer_event))
            {
                return Err(E::Reference);
            }
        }
        if d.pipeline.blend_constant.iter().any(|v| !v.is_finite())
            || !d.pipeline.slope_bias.is_finite()
            || !d.pipeline.bias_clamp.is_finite()
        {
            return Err(E::Malformed);
        }
        if ![1, 2, 4, 8].contains(&d.pipeline.color.samples)
            || d.pipeline.color.width == 0
            || d.pipeline.color.height == 0
        {
            return Err(E::Unsupported);
        }
        let runtime = payload(root, &d.runtime_corners, 64 * 1024 * 1024)?;
        let mut cursor: usize = 0;
        let secondary_raw = d
            .runtime_secondary
            .as_ref()
            .map(|p| payload(root, p, 16 * 1024 * 1024))
            .transpose()?;
        let mut secondary_cursor: usize = 0;
        for slice in &d.slices {
            let range = scene
                .geometry
                .surfaces()
                .get(slice.surface)
                .ok_or(E::Reference)?
                .index_range();
            let end = slice
                .first_corner
                .checked_add(slice.corners)
                .ok_or(E::Limit)?;
            if slice.corners == 0
                || slice.corners % 3 != 0
                || slice.first_corner < range.start
                || end > range.end
            {
                return Err(E::Reference);
            }
            let n = slice.corners.checked_mul(36).ok_or(E::Limit)?;
            let stop = cursor.checked_add(n).ok_or(E::Limit)?;
            if runtime.get(cursor..stop) != source.get(slice.first_corner * 36..end * 36) {
                return Err(E::Identity);
            }
            let format = scene
                .surfaces
                .get(slice.surface)
                .ok_or(E::Reference)?
                .source_vertex_format;
            let stride = crate::secondary_stream::secondary_stride(format)?;
            for expected in &secondary[slice.first_corner..end] {
                if stride > 0 {
                    let end = secondary_cursor.checked_add(stride).ok_or(E::Limit)?;
                    let raw = secondary_raw
                        .as_ref()
                        .and_then(|b| b.get(secondary_cursor..end))
                        .ok_or(E::Attributes)?;
                    if crate::secondary_stream::decode_secondary_record(format, raw)? != *expected {
                        return Err(E::Identity);
                    }
                    secondary_cursor = end;
                }
            }
            cursor = stop;
        }
        if secondary_raw.as_ref().map_or(0, Vec::len) != secondary_cursor {
            return Err(E::Attributes);
        }
        if cursor != runtime.len() {
            return Err(E::Identity);
        }
        let mut uniform_bytes = Vec::new();
        let mut texture_bytes = Vec::new();
        let mut bindings = BTreeSet::new();
        for u in &d.uniforms {
            if u.register >= 14 || !bindings.insert((u.stage as u8, mapped(u.stage, u.register, 0)))
            {
                return Err(E::Reference);
            }
            uniform_bytes.push(cached(root, &u.bytes, 65536, &mut cache, &mut retained)?);
        }
        for t in &d.textures {
            if t.register >= 32
                || !bindings.insert((t.stage as u8, mapped(t.stage, t.register, 32)))
                || t.width == 0
                || t.height == 0
                || t.width > 16384
                || t.height > 16384
                || t.mips == 0
                || t.mips > 15
                || t.slices == 0
            {
                return Err(E::Reference);
            }
            if t.producer_event > 0
                && (t.producer_event >= receipt.event || !producers.contains(&t.producer_event))
            {
                return Err(E::Reference);
            }
            let bytes = cached(root, &t.bytes, 256 * 1024 * 1024, &mut cache, &mut retained)?;
            let (width, height, mips, cube, format) = inspect_captured_dds(&bytes)?;
            if (width, height, mips, cube) != (t.width, t.height, t.mips, t.cube)
                || format != t.view_format
                || t.first_mip != 0
                || t.view_mips != t.mips
                || t.first_slice != 0
                || t.slices != if cube { 6 } else { 1 }
            {
                return Err(E::Attributes);
            }
            texture_bytes.push(bytes);
        }
        for s in &d.samplers {
            if s.register >= 16
                || !bindings.insert((s.stage as u8, mapped(s.stage, s.register, 64)))
                || !s.max_anisotropy.is_finite()
                || !s.min_lod.is_finite()
                || !s.max_lod.is_finite()
                || s.min_lod > s.max_lod
                || !s.mip_bias.is_finite()
                || s.border.iter().any(|v| !v.is_finite())
            {
                return Err(E::Reference);
            }
        }
        for (stage, sha) in [
            (ShaderStage::Vertex, &d.vertex_shader_sha256),
            (ShaderStage::Fragment, &d.fragment_shader_sha256),
        ] {
            let shader = bank
                .shaders
                .iter()
                .find(|s| s.declaration.stage == stage && s.declaration.source_sha256 == *sha)
                .ok_or(E::Identity)?;
            for resource in &shader.declaration.resources {
                if !bindings.contains(&(stage as u8, resource.binding)) {
                    return Err(E::Reference);
                }
                if let ResourceKind::Texture { dimension, sample } = resource.kind {
                    let t = d
                        .textures
                        .iter()
                        .find(|t| {
                            t.stage == stage && mapped(stage, t.register, 32) == resource.binding
                        })
                        .ok_or(E::Reference)?;
                    if sample == crate::shader_bank::TextureSample::Depth
                        && t.view_format != "R32_FLOAT"
                    {
                        return Err(E::Unsupported);
                    }
                    if t.cube != (dimension == crate::shader_bank::TextureDimension::Cube) {
                        return Err(E::Attributes);
                    }
                }
                if let ResourceKind::Sampler { comparison } = resource.kind {
                    let s = d
                        .samplers
                        .iter()
                        .find(|s| {
                            s.stage == stage && mapped(stage, s.register, 64) == resource.binding
                        })
                        .ok_or(E::Reference)?;
                    // RenderDoc Never=0 denotes a non-comparison sampler.
                    if comparison != (s.filter[3] == 1) {
                        return Err(E::Attributes);
                    }
                }
                if let ResourceKind::Uniform { size } = resource.kind {
                    let i = d
                        .uniforms
                        .iter()
                        .position(|u| {
                            u.stage == stage && mapped(stage, u.register, 0) == resource.binding
                        })
                        .ok_or(E::Reference)?;
                    if uniform_bytes[i].len() < size as usize {
                        return Err(E::Attributes);
                    }
                }
            }
        }
        let object = d
            .uniforms
            .iter()
            .position(|u| u.stage == ShaderStage::Vertex && u.register == d.object_register)
            .ok_or(E::Reference)?;
        let raw = uniform_bytes[object]
            .get(d.object_offset..d.object_offset.checked_add(64).ok_or(E::Limit)?)
            .ok_or(E::Attributes)?;
        if raw
            .as_chunks::<4>()
            .0
            .iter()
            .zip(d.object_matrix)
            .any(|(b, f)| *b != f.to_le_bytes())
        {
            return Err(E::Identity);
        }
        loaded.push(LoadedWorldDraw {
            event: receipt.event,
            uniforms: uniform_bytes,
            textures: texture_bytes,
        });
    }
    Ok(CapturedFrameInput {
        manifest: m,
        draws: loaded,
        initial_attachments,
    })
}

/// Only qualified sampled DDS formats; float views retain float payloads even for comparison sampling.
pub fn inspect_captured_dds(bytes: &[u8]) -> Result<(u32, u32, u32, bool, String), E> {
    if bytes.len() >= 148
        && &bytes[..4] == b"DDS "
        && &bytes[84..88] == b"DX10"
        && bytes[128..132] == 41u32.to_le_bytes()
    {
        let word = |o: usize| {
            u32::from_le_bytes(bytes[o..o + 4].try_into().expect("fixed DDS header bounds"))
        };
        let (width, height, mips) = (word(16), word(12), word(28).max(1));
        if word(4) != 124
            || word(76) != 32
            || word(132) != 3
            || word(136) != 0
            || word(140) != 1
            || word(112) != 0
            || word(24) > 1
            || width == 0
            || height == 0
            || width > 16384
            || height > 16384
            || mips > 32 - width.max(height).leading_zeros()
        {
            return Err(E::Unsupported);
        }
        let mut length = 148usize;
        for mip in 0..mips {
            length = length
                .checked_add((width >> mip).max(1) as usize * (height >> mip).max(1) as usize * 4)
                .ok_or(E::Limit)?;
        }
        if length != bytes.len() {
            return Err(E::Malformed);
        }
        return Ok((width, height, mips, false, "R32_FLOAT".into()));
    }
    let m = crate::full_scene::inspect_dds(bytes)?;
    use crate::full_scene::DdsFormat;
    let format = match m.format {
        DdsFormat::Bc1 => "BC1_UNORM",
        DdsFormat::Bc2 => "BC2_UNORM",
        DdsFormat::Bc3 => "BC3_UNORM",
        DdsFormat::Bc5 => "BC5_UNORM",
        DdsFormat::Rgba8 => "R8G8B8A8_UNORM",
        DdsFormat::Bgra8 => "B8G8R8A8_UNORM",
    };
    Ok((m.width, m.height, m.mips, m.cube, format.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn float_view_requires_exact_complete_payload() {
        let mut b = vec![0u8; 164];
        b[..4].copy_from_slice(b"DDS ");
        b[84..88].copy_from_slice(b"DX10");
        for (o, n) in [
            (4, 124u32),
            (76, 32),
            (12, 2),
            (16, 2),
            (28, 1),
            (128, 41),
            (132, 3),
            (140, 1),
        ] {
            b[o..o + 4].copy_from_slice(&n.to_le_bytes());
        }
        assert_eq!(
            inspect_captured_dds(&b).unwrap(),
            (2, 2, 1, false, "R32_FLOAT".into())
        );
        b.pop();
        assert_eq!(inspect_captured_dds(&b), Err(E::Malformed));
        b.push(0);
        b[140..144].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(inspect_captured_dds(&b), Err(E::Unsupported));
    }

    #[test]
    fn shared_payload_is_retained_once_and_missing_alias_is_rejected() {
        let root = std::env::temp_dir().join(format!(
            "captured-payload-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("one.bin"), [1, 2, 3, 4]).unwrap();
        let p = Payload {
            file: "one.bin".into(),
            sha256: format!("{:x}", Sha256::digest([1, 2, 3, 4])),
        };
        let mut cache = BTreeMap::new();
        let mut retained = 0;
        let a = cached(&root, &p, 4, &mut cache, &mut retained).unwrap();
        let b = cached(&root, &p, 4, &mut cache, &mut retained).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(retained, 4);
        let missing = Payload {
            file: "missing.bin".into(),
            ..p.clone()
        };
        assert!(matches!(
            cached(&root, &missing, 4, &mut cache, &mut retained),
            Err(E::Io(_))
        ));
        let traversal = Payload {
            file: "../one.bin".into(),
            ..p
        };
        assert!(cached(&root, &traversal, 4, &mut cache, &mut retained).is_err());
        let invalid = Payload {
            file: "one.bin".into(),
            sha256: "0".repeat(64),
        };
        assert!(matches!(
            cached(&root, &invalid, 4, &mut cache, &mut retained),
            Err(E::Identity)
        ));
        assert!(matches!(
            cached(&root, &invalid, 3, &mut cache, &mut retained),
            Err(E::Limit)
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn physical_register_mapping_keeps_shader_spaces_distinct() {
        assert_eq!(mapped(ShaderStage::Vertex, 3, 0), 3);
        assert_eq!(mapped(ShaderStage::Fragment, 3, 0), 19);
        assert_eq!(mapped(ShaderStage::Fragment, 13, 32), 45);
        assert_eq!(mapped(ShaderStage::Fragment, 15, 64), 79);
    }
}
