//! Original static-model placements, packed LOD streams and distance selection.
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use bevy::{camera::primitives::Aabb, prelude::*};
use serde::Deserialize;

use super::{
    material::MaterialDefinition,
    native::{NativeWorldGeometry, NativeWorldPass, NativeWorldState},
    native_model::{ModelVertexLighting, NativeModelMaterial, NativeModelSurface},
    world::{WorldSurfaceLighting, source_frame},
};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReflectionProbe {
    pub origin: [f32; 3],
    pub probe_volumes: Vec<[[f32; 4]; 6]>,
    pub reflection_image: String,
    #[serde(rename = "lightingSH")]
    pub lighting_sh: [f32; 12],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StaticModelDefinition {
    model_vertex_lighting_file: String,
    static_models: Vec<Placement>,
    model_geometry: BTreeMap<String, ModelDefinition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Placement {
    origin: SourcePosition,
    axis: [[f32; 3]; 3],
    scale: f32,
    model: String,
    colors_index: u16,
    visibility: u8,
    reflection_probe_index: u8,
    primary_light_index: u8,
    cull_dist: f32,
    vertex_lighting: [VertexLightingRange; 4],
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VertexLightingRange {
    byte_offset: u64,
    vertex_count: u32,
}

#[derive(Deserialize)]
struct SourcePosition {
    x: f32,
    y: f32,
    z: f32,
}

impl SourcePosition {
    fn vector(&self) -> Vec3 {
        Vec3::new(self.x, self.y, self.z)
    }
}

impl Placement {
    fn transform(&self) -> Transform {
        let frame = Mat3::from_cols(Vec3::X, Vec3::NEG_Z, Vec3::Y);
        let rotation = frame * Mat3::from_cols_array_2d(&self.axis) * frame.transpose();
        // Draw children apply source_frame to PACKED32 vertices exactly once.
        // Their placement owner therefore keeps the original unitless scale.
        Transform {
            translation: source_frame().transform_point3(self.origin.vector()),
            rotation: Quat::from_mat3(&rotation),
            scale: Vec3::splat(self.scale),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelDefinition {
    bounds: [[f32; 3]; 2],
    vertex_file: String,
    index_file: String,
    lods: Vec<LodDefinition>,
}

#[derive(Deserialize)]
struct LodDefinition {
    dist: f32,
    surfaces: Vec<SurfaceDefinition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceDefinition {
    material_draw_surf: u64,
    base_vert_index: u32,
    vertex_byte_offset: u64,
    first_index: u32,
    vertex_count: u32,
    tri_count: u32,
    flags: u32,
    material: String,
    material_file: String,
}

pub(crate) struct LoadedModels {
    vertex_lighting: ModelVertexLighting,
    definition: StaticModelDefinition,
    geometry: BTreeMap<String, NativeWorldGeometry>,
    materials: BTreeMap<String, MaterialDefinition>,
    probes: Vec<ReflectionProbe>,
    unsupported: BTreeMap<String, String>,
}

impl StaticModelDefinition {
    /// Run on the existing loading IO task. Model bytes have one render-asset
    /// owner per model; placements share them rather than cloning vertex data.
    pub(crate) fn load(self, root: &Path, probes: &[ReflectionProbe]) -> LoadedModels {
        let vertex_lighting = std::fs::read(root.join(&self.model_vertex_lighting_file))
            .expect("Read original per-placement model vertex lighting");
        assert!(vertex_lighting.len().is_multiple_of(4));
        let mut geometry = BTreeMap::new();
        let mut materials = BTreeMap::new();
        let mut unsupported = BTreeMap::new();
        for (name, model) in &self.model_geometry {
            assert!(
                !model.lods.is_empty()
                    && model.lods.len() <= 4
                    && model
                        .lods
                        .iter()
                        .all(|lod| lod.dist.is_finite() && lod.dist > 0.0)
                    && model
                        .lods
                        .windows(2)
                        .all(|pair| pair[0].dist <= pair[1].dist),
                "Original model {name} requires its ordered source LOD distances"
            );
            assert!(
                model.bounds.iter().flatten().all(|value| value.is_finite())
                    && (0..3).all(|axis| model.bounds[0][axis] <= model.bounds[1][axis]),
                "Original model {name} bounds"
            );
            let read = |path: &str| {
                std::fs::read(root.join(path)).unwrap_or_else(|error| {
                    panic!("Read original model {name} stream {path}: {error}")
                })
            };
            let packed = NativeWorldGeometry {
                primary: read(&model.vertex_file).into(),
                auxiliary: Vec::new(),
                indices: read(&model.index_file).into(),
                vertex_stride: 32,
            };
            assert!(
                !packed.primary.is_empty()
                    && packed.primary.len().is_multiple_of(32)
                    && !packed.indices.is_empty()
                    && packed.indices.len().is_multiple_of(2),
                "Original model {name} requires complete PACKED32 vertices and Uint16 indices"
            );
            for (lod_index, lod) in model.lods.iter().enumerate() {
                assert!(
                    !lod.surfaces.is_empty(),
                    "Original model {name} LOD {lod_index} has no surfaces"
                );
                for surface in &lod.surfaces {
                    let first = surface.first_index as usize * 2;
                    let end = first + surface.tri_count as usize * 6;
                    assert!(
                        surface.vertex_byte_offset.is_multiple_of(32)
                            && surface.vertex_byte_offset + u64::from(surface.vertex_count) * 32
                                <= packed.primary.len() as u64
                            && end <= packed.indices.len(),
                        "Original model {name} LOD {lod_index} surface stream range"
                    );
                    assert!(
                        packed.indices[first..end].chunks_exact(2).all(|bytes| {
                            u32::from(u16::from_le_bytes([bytes[0], bytes[1]]))
                                < surface.vertex_count
                        }),
                        "Original model {name} LOD {lod_index} requires surface-local Uint16 indices"
                    );
                    // Static draw type4 dispatches through 004751d0 to
                    // 004784f0/004785f0. Both upload verts0 unchanged via
                    // 00476950 and use the placement transform, including
                    // SKINNED|DEFORMED (0x82) surfaces. Animated script models
                    // have a separate pose owner and draw path.
                    if !matches!(surface.flags, 0 | 0x82) {
                        unsupported.insert(
                            format!("{name} LOD {lod_index} {}", surface.material),
                            format!(
                                "Unsupported original static surface vertex encoding (flags {:#x})",
                                surface.flags
                            ),
                        );
                    }
                    materials
                        .entry(surface.material_file.clone())
                        .or_insert_with(|| {
                            MaterialDefinition::read(&root.join(&surface.material_file))
                                .unwrap_or_else(|error| panic!("Original model {name}: {error}"))
                        });
                }
            }
            geometry.insert(name.clone(), packed);
        }
        assert!(
            !self.static_models.is_empty(),
            "Origins requires its original static model placements"
        );
        for placement in &self.static_models {
            assert!(
                self.model_geometry.contains_key(&placement.model),
                "Missing original model {}",
                placement.model
            );
            assert!(
                placement.origin.vector().is_finite()
                    && placement
                        .axis
                        .iter()
                        .flatten()
                        .all(|value| value.is_finite())
                    && placement.scale.is_finite()
                    && placement.scale > 0.0
                    && placement.cull_dist.is_finite()
                    && placement.cull_dist > 0.0,
                "Original static placement {} requires its finite source pose and cullDist",
                placement.model
            );
            assert!(
                usize::from(placement.reflection_probe_index) < probes.len(),
                "Original static placement {} reflectionProbeIndex",
                placement.model
            );
        }
        for placement in &self.static_models {
            let model = &self.model_geometry[&placement.model];
            for (lod_index, range) in placement.vertex_lighting.iter().enumerate() {
                assert!(
                    range.byte_offset.is_multiple_of(4)
                        && range.byte_offset + u64::from(range.vertex_count) * 4
                            <= vertex_lighting.len() as u64,
                    "Original model {} vertex lighting range",
                    placement.model
                );
                if range.vertex_count > 0 {
                    assert!(
                        lod_index < model.lods.len(),
                        "Lighting references missing LOD"
                    );
                }
                if let Some(lod) = model.lods.get(lod_index) {
                    for surface in &lod.surfaces {
                        if materials[&surface.material_file]
                            .technique_set()
                            .starts_with("mlv_")
                        {
                            assert!(
                                u64::from(surface.base_vert_index)
                                    + u64::from(surface.vertex_count)
                                    <= u64::from(range.vertex_count),
                                "Original model {} LOD {lod_index} lacks authored vertex lighting",
                                placement.model
                            );
                        }
                    }
                }
            }
        }
        for probe in probes {
            assert!(
                probe.lighting_sh.iter().all(|value| value.is_finite()),
                "Original reflection probe SH"
            );
        }
        LoadedModels {
            vertex_lighting: ModelVertexLighting(vertex_lighting),
            definition: self,
            geometry,
            materials,
            probes: probes.to_vec(),
            unsupported,
        }
    }
}

#[derive(Component)]
pub(crate) struct NativeModelsRoot {
    images: Vec<Handle<Image>>,
}

impl NativeModelsRoot {
    pub(crate) fn images_ready(&self, server: &AssetServer) -> bool {
        self.images
            .iter()
            .all(|image| server.is_loaded_with_dependencies(image.id()))
    }
}

struct BoundSurface {
    pass: NativeWorldPass,
    material_draw_surf: u64,
    base_vert_index: u32,
    material: Handle<NativeModelMaterial>,
    state: NativeWorldState,
    vertex_byte_offset: u64,
    first_index: u32,
    index_count: u32,
}

struct BoundLod {
    draw_type: u64,
    distance: f32,
    surfaces: Vec<BoundSurface>,
}

struct ModelDraws {
    geometry: Handle<NativeWorldGeometry>,
    lods: Vec<BoundLod>,
}

#[derive(Component)]
pub(crate) struct StaticModelInstance {
    pub(crate) source_index: u32,
    primary_light_index: u8,
    vertex_lighting: Handle<ModelVertexLighting>,
    lighting_ranges: [VertexLightingRange; 4],
    model: Arc<ModelDraws>,
    origin: Vec3,
    scale: f32,
    cull_distance: f32,
    colors_index: usize,
    visibility: u8,
    surfaces: Vec<Entity>,
    selected: Option<usize>,
}

impl StaticModelInstance {
    fn surface(&self, placement: Entity, lod: usize, source: &BoundSurface) -> NativeModelSurface {
        let lighting = self.lighting_ranges[lod];
        NativeModelSurface {
            pass: source.pass,
            // 0045a470 sets primary light and model type, 004307d0 replaces
            // low16 with a batch pointer. Keep original placement order for
            // equivalent keys; dynamic-light classification is not yet supplied.
            sort_key: (
                (source.material_draw_surf & 0xff00_0fff_3fff_0000)
                    | (u64::from(self.primary_light_index) << 44)
                    | (self.model.lods[lod].draw_type << 52),
                self.source_index,
            ),
            vertex_lighting: (lighting.vertex_count > 0).then(|| {
                (
                    self.vertex_lighting.clone(),
                    lighting.byte_offset + u64::from(source.base_vert_index) * 4,
                )
            }),
            geometry: super::native_model::ModelGeometry::Static(self.model.geometry.clone()),
            material: source.material.clone(),
            state: source.state,
            primary_byte_offset: source.vertex_byte_offset,
            first_index: source.first_index,
            index_count: source.index_count,
            placement,
            colors_index: self.colors_index,
            visibility: self.visibility,
        }
    }

    fn select(&self, eye: Vec3, fov_scale: f32) -> Option<usize> {
        // 0045ac40: source-origin distance, FOV scale, cullDist, then the
        // original max(placement.scale,1) divisor. No metre-scaled thresholds.
        let distance = eye.distance(self.origin);
        let adjusted = distance * fov_scale;
        if adjusted >= self.cull_distance {
            return None;
        }
        let inverse_scale = self.scale.max(1.0).recip();
        let adjusted = inverse_scale * adjusted;
        let distance = inverse_scale * distance;
        // 00792c70: first strict upper distance threshold; retry raw distance
        // only if adjusted selection ran out and the two distances differ.
        source_lod(
            self.model.lods.iter().map(|lod| lod.distance),
            adjusted,
            distance,
        )
    }
}

/// Original 00792c70, shared by static and scene-model submission.
pub(crate) fn source_lod(
    mut distances: impl Iterator<Item = f32> + Clone,
    adjusted: f32,
    raw: f32,
) -> Option<usize> {
    let selected = distances.clone().position(|limit| adjusted < limit);
    if selected.is_some() || adjusted == raw {
        selected
    } else {
        distances.position(|limit| raw < limit)
    }
}

impl LoadedModels {
    pub(crate) fn spawn(
        self,
        commands: &mut Commands,
        server: &AssetServer,
        geometries: &mut Assets<NativeWorldGeometry>,
        materials: &mut Assets<NativeModelMaterial>,
        lighting: &mut Assets<ModelVertexLighting>,
    ) {
        let mut missing = self.unsupported;
        let probes: Vec<_> = self
            .probes
            .iter()
            .map(|probe| super::material::image(server, &probe.reflection_image))
            .collect();
        let mut bound = BTreeMap::new();
        let mut images = BTreeMap::new();
        let requested: BTreeSet<_> = self
            .definition
            .static_models
            .iter()
            .map(|placement| (placement.model.as_str(), placement.reflection_probe_index))
            .collect();
        for &(model, probe_index) in &requested {
            for lod in &self.definition.model_geometry[model].lods {
                for surface in &lod.surfaces {
                    let key = (surface.material_file.as_str(), probe_index);
                    if bound.contains_key(&key) || missing.contains_key(&surface.material_file) {
                        continue;
                    }
                    let source = &self.materials[&surface.material_file];
                    if !source.has_camera_pass() {
                        continue;
                    }
                    match source.model(server, probes[usize::from(probe_index)].clone()) {
                        Ok((material, state)) => {
                            for image in material.images() {
                                images.entry(image.id()).or_insert_with(|| image.clone());
                            }
                            bound.insert(key, (materials.add(material), state));
                        }
                        Err(error) => {
                            missing.insert(
                                surface.material_file.clone(),
                                format!(
                                    "{} ({}): {error}",
                                    surface.material,
                                    source.technique_set()
                                ),
                            );
                        }
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "Original full-map static model rendering is incomplete:\n{}",
            missing
                .iter()
                .map(|(path, error)| format!("{path}: {error}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let vertex_lighting = lighting.add(self.vertex_lighting);
        let geometry: BTreeMap<_, _> = self
            .geometry
            .into_iter()
            .map(|(name, geometry)| (name, geometries.add(geometry)))
            .collect();
        let mut models = BTreeMap::<String, BTreeMap<u8, Arc<ModelDraws>>>::new();
        for &(model, probe_index) in &requested {
            let source = &self.definition.model_geometry[model];
            models.entry(model.to_owned()).or_default().insert(
                probe_index,
                Arc::new(ModelDraws {
                    geometry: geometry[model].clone(),
                    lods: source
                        .lods
                        .iter()
                        .map(|lod| BoundLod {
                            draw_type: if lod.surfaces.iter().any(|s| s.flags & 0x80 != 0) {
                                4
                            } else {
                                3
                            },
                            distance: lod.dist,
                            surfaces: lod
                                .surfaces
                                .iter()
                                .filter(|surface| {
                                    self.materials[&surface.material_file].has_camera_pass()
                                })
                                .map(|surface| {
                                    let (material, state) =
                                        &bound[&(surface.material_file.as_str(), probe_index)];
                                    BoundSurface {
                                        pass: self.materials[&surface.material_file].world_pass(),
                                        material_draw_surf: surface.material_draw_surf,
                                        base_vert_index: surface.base_vert_index,
                                        material: material.clone(),
                                        state: *state,
                                        vertex_byte_offset: surface.vertex_byte_offset,
                                        first_index: surface.first_index,
                                        index_count: surface.tri_count * 3,
                                    }
                                })
                                .collect(),
                        })
                        .collect(),
                }),
            );
        }
        drop(requested);
        let root = commands
            .spawn((
                NativeModelsRoot {
                    images: images.into_values().collect(),
                },
                Transform::IDENTITY,
                Visibility::Hidden,
                Name::new("Origins original static models"),
            ))
            .id();
        let frame = Transform::from_matrix(source_frame());
        for (source_index, placement) in self.definition.static_models.into_iter().enumerate() {
            let model = models[placement.model.as_str()][&placement.reflection_probe_index].clone();
            let bounds = self.definition.model_geometry[&placement.model].bounds;
            let pose = placement.transform();
            let owner = commands
                .spawn((
                    ChildOf(root),
                    crate::assets::StaticPropRoot,
                    pose,
                    Visibility::Inherited,
                    Name::new(placement.model),
                ))
                .id();
            let mut instance = StaticModelInstance {
                source_index: source_index as u32,
                primary_light_index: placement.primary_light_index,
                vertex_lighting: vertex_lighting.clone(),
                lighting_ranges: placement.vertex_lighting,
                model,
                origin: placement.origin.vector(),
                scale: placement.scale,
                cull_distance: placement.cull_dist,
                colors_index: usize::from(placement.colors_index),
                visibility: placement.visibility,
                surfaces: Vec::new(),
                selected: Some(0),
            };
            let sh = &self.probes[usize::from(placement.reflection_probe_index)].lighting_sh;
            let lighting = std::array::from_fn(|i| Vec4::from_slice(&sh[i * 4..i * 4 + 4]));
            let largest = instance
                .model
                .lods
                .iter()
                .max_by_key(|lod| lod.surfaces.len())
                .expect("validated original model LODs");
            // Keep only enough draw entities for the largest source LOD. Every
            // slot retains real source geometry; index_count 0 marks an inactive
            // source draw. LOD switches retarget slots without entity churn.
            for (index, maximum) in largest.surfaces.iter().enumerate() {
                let source_surface = instance.model.lods[0].surfaces.get(index);
                let mut draw = instance.surface(owner, 0, source_surface.unwrap_or(maximum));
                if source_surface.is_none() {
                    draw.index_count = 0;
                }
                instance.surfaces.push(
                    commands
                        .spawn((
                            ChildOf(owner),
                            frame,
                            Visibility::Inherited,
                            Aabb::from_min_max(
                                Vec3::from_array(bounds[0]),
                                Vec3::from_array(bounds[1]),
                            ),
                            WorldSurfaceLighting(lighting),
                            draw,
                        ))
                        .id(),
                );
            }
            commands.entity(owner).insert(instance);
        }
    }
}

#[derive(Default)]
pub(crate) struct LodView {
    eye: Option<Vec3>,
    fov_scale: Option<f32>,
}

/// After camera/transform updates and before visibility checks/extraction.
/// Defaults recovered from 0041f2b0: rigid LOD scale 1, bias 0, auto-LOD disabled.
pub(crate) fn select_lods(
    cameras: Query<(&Camera, &GlobalTransform), With<super::GameCamera>>,
    roots: Query<Ref<NativeModelsRoot>>,
    mut instances: Query<(Entity, &mut StaticModelInstance)>,
    mut surfaces: Query<&mut NativeModelSurface>,
    mut previous: Local<LodView>,
    visibility: Res<super::culling::SourceVisibility>,
) {
    let Ok((camera, camera_transform)) = cameras.single() else {
        return;
    };
    if roots.is_empty() {
        return;
    }
    let eye = source_frame()
        .inverse()
        .transform_point3(camera_transform.translation());
    // 0041b820 multiplies tanHalfFovX by 2.118673. Its asymmetric hysteresis
    // follows increases immediately and decreases only after a 1.25 ratio.
    let requested = camera.clip_from_view().x_axis.x.recip() * 2.118673;
    assert!(
        requested.is_finite() && requested > 0.0,
        "Original rigid LOD requires a positive perspective FOV"
    );
    let mut fov_scale = previous.fov_scale.unwrap_or(requested);
    if fov_scale <= requested || fov_scale / requested > 1.25 {
        fov_scale = requested;
    }
    let changed = previous.eye != Some(eye)
        || previous.fov_scale != Some(fov_scale)
        || visibility.is_changed()
        || roots.iter().any(|root| root.is_added());
    previous.eye = Some(eye);
    previous.fov_scale = Some(fov_scale);
    if !changed {
        return;
    }
    for (owner, mut instance) in &mut instances {
        if visibility.models.get(instance.source_index as usize) == Some(&false) {
            continue;
        }
        let selected = instance.select(eye, fov_scale);
        if instance.selected == selected {
            continue;
        }
        for (index, entity) in instance.surfaces.iter().enumerate() {
            let mut draw = surfaces
                .get_mut(*entity)
                .expect("original static model draw owner");
            if let Some(source) =
                selected.and_then(|lod| instance.model.lods[lod].surfaces.get(index))
            {
                *draw = instance.surface(owner, selected.expect("selected source LOD"), source);
            } else {
                draw.index_count = 0;
            }
        }
        instance.selected = selected;
    }
}
