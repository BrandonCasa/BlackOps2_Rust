//! Camera motion at the source-shader boundary. Frozen captures stay explicit.
use bevy::prelude::*;
use bo2_assets::{captured_frame::*, shader_bank::*};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCameraError {
    InvalidPose,
    InvalidProjection,
    MissingShader,
    MissingMatrix,
    UnsupportedMatrix,
    ConflictingLayout,
    InvalidUniform,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatrixKind {
    Object,
    ViewProjection,
    Shadow,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Patch {
    binding: u32,
    offset: usize,
    kind: MatrixKind,
}
pub struct CameraUniformUpdate {
    pub event: u32,
    pub uniforms: Vec<(u32, Vec<u8>)>,
}
pub struct CameraUniformPatch {
    pub binding: u32,
    pub offset: usize,
    pub bytes: Vec<u8>,
}
pub struct CameraPatchUpdate {
    pub event: u32,
    pub patches: Vec<CameraUniformPatch>,
}
struct DrawTemplate {
    event: u32,
    uniforms: BTreeMap<u32, Arc<[u8]>>,
    patches: Vec<Patch>,
}
pub struct NativeCameraAdapter {
    initial: Transform,
    source_origin: Vec3,
    source_view_projection: Mat4,
    projection: Mat4,
    templates: Vec<DrawTemplate>,
}
fn source_from_bevy() -> Mat4 {
    Mat4::from_cols(
        Vec4::X,
        Vec4::new(0., 0., 1., 0.),
        Vec4::new(0., -1., 0., 0.),
        Vec4::W,
    )
}
fn matrix(rows: [f32; 16]) -> Mat4 {
    Mat4::from_cols_array(&rows).transpose()
}
fn rows(m: Mat4) -> [f32; 16] {
    m.transpose().to_cols_array()
}
fn read_matrix(bytes: &[u8], offset: usize) -> Result<[f32; 16], NativeCameraError> {
    let data = bytes
        .get(
            offset
                ..offset
                    .checked_add(64)
                    .ok_or(NativeCameraError::InvalidUniform)?,
        )
        .ok_or(NativeCameraError::InvalidUniform)?;
    let mut result = [0.; 16];
    for (i, word) in data.as_chunks::<4>().0.iter().enumerate() {
        result[i] = f32::from_le_bytes(*word);
    }
    if result.iter().any(|v| !v.is_finite()) {
        return Err(NativeCameraError::InvalidUniform);
    }
    Ok(result)
}
fn write_matrix(bytes: &mut [u8], offset: usize, rows: [f32; 16]) -> Result<(), NativeCameraError> {
    if rows.iter().any(|v| !v.is_finite()) {
        return Err(NativeCameraError::InvalidUniform);
    }
    let dest = bytes
        .get_mut(
            offset
                ..offset
                    .checked_add(64)
                    .ok_or(NativeCameraError::InvalidUniform)?,
        )
        .ok_or(NativeCameraError::InvalidUniform)?;
    for (word, v) in dest.as_chunks_mut::<4>().0.iter_mut().zip(rows) {
        word.copy_from_slice(&v.to_le_bytes());
    }
    Ok(())
}
fn layout(pass: &ShaderPass) -> Result<Vec<Patch>, NativeCameraError> {
    let mut patches = BTreeSet::new();
    for arg in &pass.args {
        let ArgumentSource::CodeConstant {
            index,
            first_row,
            row_count,
        } = &arg.source
        else {
            continue;
        };
        if *index < 200 {
            continue;
        }
        let kind = match index {
            213 => MatrixKind::Object,
            229 => MatrixKind::ViewProjection,
            237 => MatrixKind::Shadow,
            _ => return Err(NativeCameraError::UnsupportedMatrix),
        };
        let ArgumentDestination::Uniform {
            buffer,
            offset,
            size,
        } = arg.destination
        else {
            return Err(NativeCameraError::UnsupportedMatrix);
        };
        if arg.stage != ShaderStage::Vertex || *first_row != 0 || *row_count != 4 || size != 64 {
            return Err(NativeCameraError::UnsupportedMatrix);
        }
        if !matches!(
            (kind, buffer, offset),
            (MatrixKind::Object, 3, 0)
                | (MatrixKind::ViewProjection, 0, 576)
                | (MatrixKind::Shadow, 0, 768)
        ) {
            return Err(NativeCameraError::UnsupportedMatrix);
        }
        patches.insert(Patch {
            binding: u32::from(buffer),
            offset: usize::from(offset),
            kind,
        });
    }
    if !patches.iter().any(|p| p.kind == MatrixKind::Object)
        || !patches.iter().any(|p| p.kind == MatrixKind::ViewProjection)
    {
        return Err(NativeCameraError::MissingMatrix);
    }
    Ok(patches.into_iter().collect())
}
impl NativeCameraAdapter {
    /// Recorded row-major VP contains right/up projection scales and a forward
    /// homogeneous-w row. Only this admitted infinite reversed-Z form is accepted.
    pub fn from_frame(
        frame: &CapturedFrameInput,
        bank: &ShaderBankInput,
    ) -> Result<Self, NativeCameraError> {
        let mut adapter = Self::from_projection(
            frame.manifest.camera_origin,
            frame.manifest.camera_view_projection,
        )?;
        for receipt in &frame.manifest.draws {
            let Dispatch::World { draw } = &receipt.dispatch else {
                continue;
            };
            let loaded = frame
                .draws
                .iter()
                .find(|d| d.event == receipt.event)
                .ok_or(NativeCameraError::InvalidUniform)?;
            let vi = bank
                .shaders
                .iter()
                .position(|s| {
                    s.declaration.source_sha256 == draw.vertex_shader_sha256
                        && s.declaration.stage == ShaderStage::Vertex
                })
                .ok_or(NativeCameraError::MissingShader)?;
            let fi = bank
                .shaders
                .iter()
                .position(|s| {
                    s.declaration.source_sha256 == draw.fragment_shader_sha256
                        && s.declaration.stage == ShaderStage::Fragment
                })
                .ok_or(NativeCameraError::MissingShader)?;
            let mut passes = bank
                .families
                .iter()
                .flat_map(|f| &f.techniques)
                .flat_map(|t| &t.passes)
                .filter(|p| p.vertex_shader == vi && p.pixel_shader == fi);
            let patches = layout(passes.next().ok_or(NativeCameraError::MissingShader)?)?;
            for pass in passes {
                if layout(pass)? != patches {
                    return Err(NativeCameraError::ConflictingLayout);
                }
            }
            if loaded.uniforms.len() != draw.uniforms.len() {
                return Err(NativeCameraError::InvalidUniform);
            }
            let mut uniforms = BTreeMap::new();
            for patch in &patches {
                let i = draw
                    .uniforms
                    .iter()
                    .position(|u| {
                        u.stage == ShaderStage::Vertex && u32::from(u.register) == patch.binding
                    })
                    .ok_or(NativeCameraError::MissingMatrix)?;
                let bytes = Arc::clone(&loaded.uniforms[i]);
                let m = read_matrix(&bytes, patch.offset)?;
                match patch.kind {
                    MatrixKind::Object => {
                        if draw.object_register != 3
                            || draw.object_offset != 0
                            || m.map(f32::to_bits) != draw.object_matrix.map(f32::to_bits)
                            || m[12..] != [0., 0., 0., 1.]
                        {
                            return Err(NativeCameraError::UnsupportedMatrix);
                        }
                    }
                    MatrixKind::ViewProjection => {
                        if m.map(f32::to_bits)
                            != frame.manifest.camera_view_projection.map(f32::to_bits)
                        {
                            return Err(NativeCameraError::InvalidProjection);
                        }
                    }
                    MatrixKind::Shadow => {}
                }
                uniforms.insert(patch.binding, bytes);
            }
            adapter.templates.push(DrawTemplate {
                event: receipt.event,
                uniforms,
                patches,
            });
        }
        Ok(adapter)
    }
    pub fn from_projection(
        origin: [f32; 3],
        captured: [f32; 16],
    ) -> Result<Self, NativeCameraError> {
        if origin.iter().chain(&captured).any(|v| !v.is_finite()) {
            return Err(NativeCameraError::InvalidProjection);
        }
        let m = matrix(captured);
        let right = m.row(0).truncate();
        let up = m.row(1).truncate();
        let forward = m.row(3).truncate();
        if right.length() < 0.0001
            || up.length() < 0.0001
            || (forward.length() - 1.).abs() > 0.0001
            || captured[3] != 0.
            || captured[7] != 0.
            || captured[15] != 0.
            || captured[8..11] != [0.; 3]
            || captured[11] <= 0.
        {
            return Err(NativeCameraError::InvalidProjection);
        }
        let r = right.normalize();
        let u = up.normalize();
        let f = forward.normalize();
        if r.dot(u).abs() > 0.0001
            || r.dot(f).abs() > 0.0001
            || u.dot(f).abs() > 0.0001
            || (r.cross(u) + f).length() > 0.0001
        {
            return Err(NativeCameraError::InvalidProjection);
        }
        let convert = |v: Vec3| Vec3::from_array(crate::coordinates::source_to_bevy(v.to_array()));
        let initial = Transform::from_translation(convert(Vec3::from_array(origin))).with_rotation(
            Quat::from_mat3(&Mat3::from_cols(convert(r), convert(u), convert(-f))).normalize(),
        );
        let projection = m * source_from_bevy() * Mat4::from_quat(initial.rotation);
        Ok(Self {
            initial,
            source_origin: Vec3::from_array(origin),
            source_view_projection: m,
            projection,
            templates: vec![],
        })
    }
    pub fn initial_pose(&self) -> Transform {
        self.initial
    }
    /// Exact retained lens matrix in Bevy camera-local coordinates.
    pub fn projection(&self) -> Mat4 {
        self.projection
    }
    pub fn draw_count(&self) -> usize {
        self.templates.len()
    }
    pub fn source_view_projection(&self, pose: &Transform) -> Result<Mat4, NativeCameraError> {
        self.validate_pose(pose)?;
        if pose.rotation == self.initial.rotation || pose.rotation == -self.initial.rotation {
            return Ok(self.source_view_projection);
        }
        Ok(self.projection * (source_from_bevy() * Mat4::from_quat(pose.rotation)).transpose())
    }
    fn validate_pose(&self, pose: &Transform) -> Result<(), NativeCameraError> {
        if !pose.translation.is_finite()
            || !pose.rotation.is_finite()
            || pose.scale != Vec3::ONE
            || (pose.rotation.length_squared() - 1.).abs() > 0.0001
        {
            return Err(NativeCameraError::InvalidPose);
        }
        Ok(())
    }
    pub fn uniform_updates(
        &self,
        pose: &Transform,
    ) -> Result<Vec<CameraUniformUpdate>, NativeCameraError> {
        let vp = self.source_view_projection(pose)?;
        let source = Vec3::from_array(crate::coordinates::bevy_to_source(
            pose.translation.to_array(),
        ));
        let delta = source - self.source_origin;
        let reset = pose.translation == self.initial.translation
            && (pose.rotation == self.initial.rotation || pose.rotation == -self.initial.rotation);
        let mut updates = Vec::with_capacity(self.templates.len());
        for template in &self.templates {
            let mut uniforms: BTreeMap<_, _> = template
                .uniforms
                .iter()
                .map(|(&binding, bytes)| (binding, bytes.to_vec()))
                .collect();
            if !reset {
                for patch in &template.patches {
                    let original = read_matrix(&template.uniforms[&patch.binding], patch.offset)?;
                    let value = match patch.kind {
                        MatrixKind::ViewProjection => rows(vp),
                        MatrixKind::Object => {
                            let mut m = original;
                            for axis in 0..3 {
                                m[axis * 4 + 3] -= delta[axis];
                            }
                            m
                        }
                        MatrixKind::Shadow => {
                            let mut m = original;
                            for row in 0..4 {
                                m[row * 4 + 3] +=
                                    Vec3::new(m[row * 4], m[row * 4 + 1], m[row * 4 + 2])
                                        .dot(delta);
                            }
                            m
                        }
                    };
                    write_matrix(
                        uniforms
                            .get_mut(&patch.binding)
                            .ok_or(NativeCameraError::InvalidUniform)?,
                        patch.offset,
                        value,
                    )?;
                }
            }
            updates.push(CameraUniformUpdate {
                event: template.event,
                uniforms: uniforms.into_iter().collect(),
            });
        }
        Ok(updates)
    }
    /// Only camera-owned bytes. Applying these to per-material base buffers
    /// preserves material constants, probe SH and all untouched object coefficients.
    pub fn uniform_patches(
        &self,
        pose: &Transform,
    ) -> Result<Vec<CameraPatchUpdate>, NativeCameraError> {
        self.uniform_updates(pose)?
            .into_iter()
            .zip(&self.templates)
            .map(|(update, template)| {
                let uniforms: BTreeMap<_, _> = update.uniforms.into_iter().collect();
                let mut patches = Vec::new();
                for patch in &template.patches {
                    let bytes = uniforms
                        .get(&patch.binding)
                        .ok_or(NativeCameraError::InvalidUniform)?;
                    let ranges = if patch.kind == MatrixKind::Object {
                        vec![
                            (patch.offset + 12, 4),
                            (patch.offset + 28, 4),
                            (patch.offset + 44, 4),
                        ]
                    } else {
                        vec![(patch.offset, 64)]
                    };
                    for (offset, size) in ranges {
                        patches.push(CameraUniformPatch {
                            binding: patch.binding,
                            offset,
                            bytes: bytes
                                .get(offset..offset + size)
                                .ok_or(NativeCameraError::InvalidUniform)?
                                .to_vec(),
                        });
                    }
                }
                Ok(CameraPatchUpdate {
                    event: update.event,
                    patches,
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> NativeCameraAdapter {
        let vp = [
            0.75, 0., 0., 0., 0., 0., 1.3, 0., 0., 0., 0., 4., 0., 1., 0., 0.,
        ];
        let mut adapter = NativeCameraAdapter::from_projection([10., 20., 30.], vp).unwrap();
        for event in [1, 2] {
            let mut globals = vec![0u8; 1024];
            write_matrix(&mut globals, 576, vp).unwrap();
            write_matrix(&mut globals, 768, rows(Mat4::IDENTITY)).unwrap();
            globals[160..164].copy_from_slice(&0.375f32.to_le_bytes());
            let mut object = vec![0u8; 192];
            let translation = Vec3::new(-10., -20., -30.)
                + if event == 2 {
                    Vec3::new(5., 7., 9.)
                } else {
                    Vec3::ZERO
                };
            write_matrix(&mut object, 0, rows(Mat4::from_translation(translation))).unwrap();
            object[128..132].copy_from_slice(&0.5f32.to_le_bytes());
            adapter.templates.push(DrawTemplate {
                event,
                uniforms: BTreeMap::from([(0, globals.into()), (3, object.into())]),
                patches: vec![
                    Patch {
                        binding: 0,
                        offset: 576,
                        kind: MatrixKind::ViewProjection,
                    },
                    Patch {
                        binding: 0,
                        offset: 768,
                        kind: MatrixKind::Shadow,
                    },
                    Patch {
                        binding: 3,
                        offset: 0,
                        kind: MatrixKind::Object,
                    },
                ],
            });
        }
        adapter
    }
    #[test]
    fn reset_is_bit_exact_and_object_models_remain_placed_in_source_world() {
        let adapter = fixture();
        let initial = adapter.initial_pose();
        assert_eq!(initial.translation, Vec3::new(10., 30., -20.));
        let reset = adapter.uniform_updates(&initial).unwrap();
        for (update, template) in reset.iter().zip(&adapter.templates) {
            for (binding, bytes) in &update.uniforms {
                assert_eq!(bytes.as_slice(), template.uniforms[binding].as_ref());
            }
        }
        let pose = initial.with_translation(initial.translation + Vec3::new(1., 2., 3.));
        let updates = adapter.uniform_updates(&pose).unwrap();
        let source = Vec3::from_array(crate::coordinates::bevy_to_source(
            pose.translation.to_array(),
        ));
        for (update, template) in updates.iter().zip(&adapter.templates) {
            let buffers: BTreeMap<_, _> = update.uniforms.iter().map(|(b, v)| (*b, v)).collect();
            let old = matrix(read_matrix(&template.uniforms[&3], 0).unwrap());
            let new = matrix(read_matrix(buffers[&3], 0).unwrap());
            let p = Vec3::new(4., 7., 2.);
            assert!(
                (old.transform_point3(p) + adapter.source_origin
                    - new.transform_point3(p)
                    - source)
                    .length()
                    < 0.00001
            );
            for offset in [0, 4, 8, 16, 20, 24, 32, 36, 40, 48, 52, 56, 60, 128] {
                assert_eq!(
                    &buffers[&3][offset..offset + 4],
                    &template.uniforms[&3][offset..offset + 4]
                );
            }
            assert_eq!(&buffers[&0][160..164], &template.uniforms[&0][160..164]);
        }
        let patches = adapter.uniform_patches(&pose).unwrap();
        assert_eq!(
            patches[0]
                .patches
                .iter()
                .filter(|p| p.binding == 3)
                .map(|p| (p.offset, p.bytes.len()))
                .collect::<Vec<_>>(),
            vec![(12, 4), (28, 4), (44, 4)]
        );
    }
    #[test]
    fn yaw_pitch_retain_projection_and_frozen_shadow_lookup_is_world_anchored() {
        let adapter = fixture();
        let initial = adapter.initial_pose();
        let pose = initial.with_rotation(Quat::from_rotation_y(0.4) * Quat::from_rotation_x(-0.2));
        let source_point = Vec3::new(2., 12., 3.).extend(1.);
        assert!(
            (adapter.source_view_projection(&pose).unwrap() * source_point
                - adapter.source_view_projection(&initial).unwrap() * source_point)
                .length()
                > 0.1
        );
        let moved = pose.with_translation(initial.translation + Vec3::new(1., 2., 3.));
        let updates = adapter.uniform_updates(&moved).unwrap();
        let globals = &updates[0].uniforms.iter().find(|(b, _)| *b == 0).unwrap().1;
        let shadow = matrix(read_matrix(globals, 768).unwrap());
        let delta = Vec3::from_array(crate::coordinates::bevy_to_source(
            moved.translation.to_array(),
        )) - adapter.source_origin;
        assert!((shadow * (source_point - delta.extend(0.)) - source_point).length() < 0.00001);
        let mut invalid = initial;
        invalid.translation.x = f32::NAN;
        assert!(adapter.uniform_updates(&invalid).is_err());
        invalid = initial;
        invalid.scale.x = 2.;
        assert!(adapter.uniform_updates(&invalid).is_err());
        invalid = initial;
        invalid.rotation = Quat::from_xyzw(0., 0., 0., 2.);
        assert!(adapter.uniform_updates(&invalid).is_err());
    }
    #[test]
    fn malformed_projection_matrix_and_uniforms_are_errors() {
        assert!(NativeCameraAdapter::from_projection([0.; 3], [0.; 16]).is_err());
        assert!(read_matrix(&[0; 63], 0).is_err());
        let mut adapter = fixture();
        adapter.templates[0]
            .uniforms
            .insert(0, Arc::from(vec![0u8; 4]));
        let pose = adapter.initial_pose().with_translation(Vec3::ONE);
        assert!(adapter.uniform_updates(&pose).is_err());
        let pass = ShaderPass {
            vertex_shader: 0,
            pixel_shader: 1,
            routing: vec![],
            args: vec![],
            custom_sampler_flags: 0,
            material_type: 0,
            precompiled_index: 0,
        };
        assert_eq!(layout(&pass), Err(NativeCameraError::MissingMatrix));
    }
    #[test]
    #[ignore = "requires admitted external captured frame and shader bank; CPU camera proof"]
    fn external_all_694_camera_templates_are_admitted() {
        fn path(key: &str) -> std::path::PathBuf {
            std::env::var_os(key).expect(key).into()
        }
        let bank = load_shader_bank(&path("BO2_SHADER_BANK")).unwrap();
        let scene = bo2_assets::full_scene::load_full_scene(&path("BO2_FULL_SCENE")).unwrap();
        let frame = load_captured_frame(
            &path("BO2_CAPTURED_FRAME"),
            &scene,
            &bank,
            &std::env::var("BO2_SHADER_BANK_SHA256").unwrap(),
        )
        .unwrap();
        let adapter = NativeCameraAdapter::from_frame(&frame, &bank).unwrap();
        assert_eq!(adapter.draw_count(), 694);
        let resets = adapter.uniform_updates(&adapter.initial_pose()).unwrap();
        for (update, template) in resets.iter().zip(&adapter.templates) {
            for (binding, bytes) in &update.uniforms {
                assert_eq!(bytes.as_slice(), template.uniforms[binding].as_ref());
            }
        }
        let moved = adapter
            .initial_pose()
            .with_translation(adapter.initial_pose().translation + Vec3::new(100., 50., 25.))
            .with_rotation(adapter.initial_pose().rotation * Quat::from_rotation_y(0.15));
        assert_eq!(adapter.uniform_patches(&moved).unwrap().len(), 694);
        let mut matched = 0usize;
        let mut mismatch = 0usize;
        let mut mismatch_kinds = BTreeMap::<String, usize>::new();
        for receipt in &frame.manifest.draws {
            let Dispatch::World { draw } = &receipt.dispatch else {
                continue;
            };
            let loaded = frame
                .draws
                .iter()
                .find(|d| d.event == receipt.event)
                .unwrap();
            let Some(i) = draw
                .uniforms
                .iter()
                .position(|u| u.stage == ShaderStage::Vertex && u.register == 3)
            else {
                continue;
            };
            if loaded.uniforms[i].len() < 176 {
                continue;
            }
            let Some(probe) = scene.surfaces[draw.slices[0].surface]
                .probe
                .and_then(|p| scene.probes.get(p))
            else {
                continue;
            };
            let expected: Vec<u8> = probe.sh.iter().flat_map(|v| v.to_le_bytes()).collect();
            if loaded.uniforms[i][128..176] == expected {
                matched += 1
            } else {
                mismatch += 1;
                let shader = bank
                    .shaders
                    .iter()
                    .find(|s| s.declaration.source_sha256 == draw.vertex_shader_sha256)
                    .unwrap();
                let size = shader.declaration.resources.iter().find_map(|r| {
                    if r.binding == 3 {
                        if let ResourceKind::Uniform { size } = r.kind {
                            Some(size)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                });
                let alternatives: Vec<_> = scene
                    .probes
                    .iter()
                    .enumerate()
                    .filter_map(|(p, v)| {
                        let bytes: Vec<_> = v.sh.iter().flat_map(|f| f.to_le_bytes()).collect();
                        (loaded.uniforms[i][128..176] == bytes).then_some(p)
                    })
                    .collect();
                *mismatch_kinds
                    .entry(format!(
                        "b3min{size:?}/sourceprobe{:?}/other{alternatives:?}",
                        scene.surfaces[draw.slices[0].surface].probe
                    ))
                    .or_default() += 1;
            }
        }
        println!(
            "camera templates:694; SH source-probe joins exact:{matched} mismatched:{mismatch}; initial {:?}",
            adapter.initial_pose()
        );
        println!("SH mismatch groups:{mismatch_kinds:?}");
    }
}
