//! Original world streams, surface bindings and existing gameplay brush owners.
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    sync::Arc,
};

use bevy::{camera::primitives::Aabb, prelude::*};
use serde::Deserialize;

use super::native_water::{
    LightDefinition, NativeWaterLights, NativeWaterMaterial, NativeWaterSurface,
    PrimaryLightDefinition, SunLightDefinition, WaterTechniqueDefinition,
};
use super::{material::MaterialDefinition, models::ReflectionProbe, native::*};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorldDrawDefinition {
    vertex_data_file0: String,
    vertex_data_file1: String,
    index_file: String,
    index_count: usize,
    surfaces: Vec<Surface>,
    models: Vec<BrushModel>,
    lightmaps: Vec<Lightmap>,
    reflection_probes: Vec<ReflectionProbe>,
    outdoor_image: String,
    outdoor_lookup_matrix: [[f32; 4]; 4],
    primary_lights: Vec<PrimaryLightDefinition>,
    sun_primary_light_index: u32,
    light_definitions: BTreeMap<String, LightDefinition>,
    water_techniques: Vec<WaterTechniqueDefinition>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Surface {
    material: String,
    material_file: String,
    base_index: u32,
    tri_count: u32,
    vertex_data_offset0: u64,
    vertex_data_offset1: u64,
    lightmap_index: usize,
    reflection_probe_index: usize,
    primary_light_index: u8,
    source_surface_index: u32,
    bounds: [[f32; 3]; 2],
    #[serde(skip)]
    vertex_reach: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BrushModel {
    start_surf_index: i32,
    surface_count: usize,
}

#[derive(Deserialize)]
struct Lightmap {
    secondary: String,
}

/// IO ownership ends when the geometry is handed to Bevy's render assets.
pub(crate) struct LoadedWorld {
    definition: WorldDrawDefinition,
    geometry: NativeWorldGeometry,
    materials: BTreeMap<String, MaterialDefinition>,
}

impl WorldDrawDefinition {
    pub(crate) fn reflection_probes(&self) -> &[ReflectionProbe] {
        &self.reflection_probes
    }

    pub(crate) fn load(mut self, root: &Path) -> LoadedWorld {
        let read = |path: &str| {
            std::fs::read(root.join(path))
                .unwrap_or_else(|error| panic!("Read original world stream {path}: {error}"))
        };
        let geometry = NativeWorldGeometry {
            primary: read(&self.vertex_data_file0),
            auxiliary: read(&self.vertex_data_file1),
            indices: read(&self.index_file),
        };
        assert_eq!(geometry.indices.len(), self.index_count * 2);
        let mut materials = BTreeMap::new();
        for surface in &mut self.surfaces {
            materials
                .entry(surface.material_file.clone())
                .or_insert_with(|| {
                    MaterialDefinition::read(&root.join(&surface.material_file))
                        .unwrap_or_else(|error| panic!("{}: {error}", surface.material))
                });
            let first = surface.base_index as usize * 2;
            let end = first + surface.tri_count as usize * 6;
            assert!(end <= geometry.indices.len(), "Original world index range");
            let max_vertex = geometry.indices[first..end]
                .chunks_exact(2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as u64)
                .max();
            if let Some(max_vertex) = max_vertex {
                surface.vertex_reach = max_vertex + 1;
                assert!(
                    surface.vertex_data_offset0 + (max_vertex + 1) * 36
                        <= geometry.primary.len() as u64,
                    "Original world primary vertex range"
                );
            }
            assert!(
                surface.bounds.iter().flatten().all(|v| v.is_finite())
                    && (0..3).all(|i| surface.bounds[0][i] <= surface.bounds[1][i]),
                "Original world surface bounds"
            );
            assert!(surface.reflection_probe_index < self.reflection_probes.len());
        }
        let mut owned = vec![false; self.surfaces.len()];
        for model in &self.models {
            if model.surface_count == 0 {
                continue;
            }
            let first = usize::try_from(model.start_surf_index)
                .expect("Original world model surface start");
            let range = owned
                .get_mut(first..first + model.surface_count)
                .expect("Original world model surface range");
            assert!(
                range.iter().all(|owned| !*owned),
                "Overlapping world models"
            );
            range.fill(true);
        }
        assert!(
            owned.into_iter().all(|owned| owned),
            "Unowned world surfaces"
        );
        LoadedWorld {
            definition: self,
            geometry,
            materials,
        }
    }
}

#[derive(Component)]
pub(crate) struct NativeWorldRoot {
    images: Vec<Handle<Image>>,
}

impl NativeWorldRoot {
    pub(crate) fn images_ready(&self, server: &AssetServer) -> bool {
        self.images
            .iter()
            .all(|image| server.is_loaded_with_dependencies(image.id()))
    }
}

#[derive(Component)]
pub(crate) struct WorldSurfaceLighting(pub(crate) [Vec4; 3]);

#[derive(Resource)]
pub(crate) struct WorldWaterLighting(pub(crate) Arc<NativeWaterLights>);

/// Native inches / Z up to Bevy metres / Y up. Keep this single boundary for
/// draw positions, object constants and the manually supplied culling bounds.
pub(crate) fn source_frame() -> Mat4 {
    Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W)
        * Mat4::from_scale(Vec3::splat(crate::assets::SOURCE_UNIT_METERS))
}

impl LoadedWorld {
    pub(crate) fn spawn(
        self,
        commands: &mut Commands,
        server: &AssetServer,
        geometries: &mut Assets<NativeWorldGeometry>,
        materials: &mut Assets<NativeWorldMaterial>,
        water_materials: &mut Assets<NativeWaterMaterial>,
        sun: &SunLightDefinition,
        game: &crate::assets::GameDefinition,
    ) {
        let world = self.definition;
        let lightmaps: Vec<_> = world
            .lightmaps
            .iter()
            .map(|map| super::material::image(server, &map.secondary))
            .collect();
        let probes: Vec<_> = world
            .reflection_probes
            .iter()
            .map(|probe| super::material::image(server, &probe.reflection_image))
            .collect();
        let mut bound = BTreeMap::new();
        let mut bound_water = BTreeMap::new();
        let mut missing = BTreeMap::<String, String>::new();
        let mut images = HashMap::new();
        for surface in &world.surfaces {
            let key = (
                surface.material_file.clone(),
                surface.lightmap_index,
                surface.reflection_probe_index,
            );
            if bound.contains_key(&key)
                || bound_water.contains_key(&key)
                || missing.contains_key(&surface.material_file)
            {
                continue;
            }
            let source = &self.materials[&surface.material_file];
            let Some(lightmap) = lightmaps.get(surface.lightmap_index) else {
                missing.insert(surface.material_file.clone(), format!(
                    "Port the original {} lighting path; surface has no directional lightmap (index {})",
                    source.technique_set(), surface.lightmap_index));
                continue;
            };
            if source.technique_set() == "wpc_cod7water_we876z2f" {
                match source.water(
                    server,
                    lightmap.clone(),
                    probes[surface.reflection_probe_index].clone(),
                ) {
                    Ok((material, state)) => {
                        for image in material.images() {
                            images.entry(image.id()).or_insert_with(|| image.clone());
                        }
                        bound_water.insert(key, (water_materials.add(material), state));
                    }
                    Err(error) => {
                        missing.insert(surface.material_file.clone(), error);
                    }
                }
                continue;
            }
            match source.world(
                server,
                lightmap.clone(),
                probes[surface.reflection_probe_index].clone(),
            ) {
                Ok((material, state)) => {
                    for image in material.images() {
                        images.entry(image.id()).or_insert_with(|| image.clone());
                    }
                    bound.insert(key, (materials.add(material), state));
                }
                Err(error) => {
                    missing.insert(surface.material_file.clone(), error);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "Original full-map world rendering is incomplete:\n{}",
            missing
                .iter()
                .map(|(path, error)| format!("{path}: {error}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let water_lights = NativeWaterLights::from_source(
            &world.primary_lights,
            world.sun_primary_light_index,
            sun,
            &world.light_definitions,
            &world.water_techniques,
            world
                .surfaces
                .iter()
                .filter(|surface| {
                    self.materials[&surface.material_file].technique_set()
                        == "wpc_cod7water_we876z2f"
                })
                .map(|surface| surface.primary_light_index),
            server,
        )
        .unwrap_or_else(|error| panic!("Original water lighting: {error}"));
        for image in water_lights.images() {
            images.entry(image.id()).or_insert_with(|| image.clone());
        }
        commands.insert_resource(WorldWaterLighting(water_lights));
        let outdoor = super::material::image(server, &world.outdoor_image);
        images
            .entry(outdoor.id())
            .or_insert_with(|| outdoor.clone());
        commands.insert_resource(super::native_effect::NativeOutdoorMap {
            image: outdoor,
            lookup_rows: world.outdoor_lookup_matrix.map(Vec4::from_array),
            sampler: super::material::code_sampler(
                bevy::render::render_resource::MipmapFilterMode::Nearest,
            ),
        });
        let images = images.into_values().collect();
        let root = commands
            .spawn((
                NativeWorldRoot { images },
                Transform::IDENTITY,
                Visibility::Hidden,
                Name::new("Origins original world"),
            ))
            .id();
        let auxiliary_len = self.geometry.auxiliary.len() as u64;
        let geometry = geometries.add(self.geometry);
        let frame = Transform::from_matrix(source_frame());
        let mut brush_owners = BTreeMap::new();
        for source in &game.environment.brush_entities {
            if source
                .initial_state
                .as_ref()
                .is_some_and(|state| state.removed)
            {
                continue;
            }
            let part = source
                .source
                .get("model")
                .unwrap_or_else(|| panic!("Original brush {} has no source model", source.id));
            let owner = commands
                .spawn((
                    ChildOf(root),
                    source.transform(),
                    Visibility::Inherited,
                    Name::new(source.id.clone()),
                ))
                .id();
            crate::environment::brushes::bind(
                commands,
                owner,
                source,
                source.visual_meshes,
                game.environment
                    .ownership
                    .iter()
                    .find(|item| item.id == source.id),
            );
            assert!(
                brush_owners.insert(part.as_str(), owner).is_none(),
                "Duplicate original brush owner {part}"
            );
        }
        for (index, model) in world.models.iter().enumerate() {
            if model.surface_count == 0 {
                continue;
            }
            let owner = if index == 0 {
                root
            } else {
                let part = format!("*{index}");
                if game
                    .environment
                    .brush_entities
                    .iter()
                    .find(|source| source.source.get("model") == Some(&part))
                    .and_then(|source| source.initial_state.as_ref())
                    .is_some_and(|state| state.removed)
                {
                    continue;
                }
                *brush_owners
                    .get(part.as_str())
                    .unwrap_or_else(|| panic!("Original world brush {part} has no gameplay owner"))
            };
            let first = model.start_surf_index as usize;
            for surface in &world.surfaces[first..first + model.surface_count] {
                let key = (
                    surface.material_file.clone(),
                    surface.lightmap_index,
                    surface.reflection_probe_index,
                );
                let sh = &world.reflection_probes[surface.reflection_probe_index].lighting_sh;
                let mut entity = commands.spawn((
                    ChildOf(owner),
                    frame,
                    Visibility::Inherited,
                    Aabb::from_min_max(
                        Vec3::from_array(surface.bounds[0]),
                        Vec3::from_array(surface.bounds[1]),
                    ),
                    WorldSurfaceLighting(std::array::from_fn(|i| {
                        Vec4::from_slice(&sh[i * 4..i * 4 + 4])
                    })),
                    Name::new(surface.material.clone()),
                ));
                if let Some((material, state)) = bound_water.get(&key) {
                    entity.insert(NativeWaterSurface {
                        geometry: geometry.clone(),
                        material: material.clone(),
                        state: *state,
                        primary_light_index: surface.primary_light_index,
                        primary_byte_offset: surface.vertex_data_offset0,
                        first_index: surface.base_index,
                        index_count: surface.tri_count * 3,
                        // 00442ec0 builds the world key with primaryLightIndex
                        // at44. All water shares one material; source order is
                        // the remaining tie. This is a relative sorting key.
                        sort_key: (u64::from(surface.primary_light_index) << 44)
                            | u64::from(surface.source_surface_index),
                    });
                } else {
                    let (material, state) = &bound[&key];
                    if let Some(stride) = materials
                        .get(material)
                        .expect("bound original world material")
                        .auxiliary_stride()
                    {
                        assert!(
                            surface.vertex_data_offset1 + surface.vertex_reach * stride
                                <= auxiliary_len,
                            "Original auxiliary vertex range for {}",
                            surface.material
                        );
                    }
                    entity.insert(NativeWorldSurface {
                        geometry: geometry.clone(),
                        material: material.clone(),
                        state: *state,
                        primary_byte_offset: surface.vertex_data_offset0,
                        auxiliary_byte_offset: surface.vertex_data_offset1,
                        first_index: surface.base_index,
                        index_count: surface.tri_count * 3,
                    });
                }
            }
        }
    }
}

pub(crate) fn object_constants(
    camera: Query<Ref<GlobalTransform>, With<super::GameCamera>>,
    mut surfaces: Query<(
        Entity,
        Ref<GlobalTransform>,
        &WorldSurfaceLighting,
        Option<&mut NativeWorldObject>,
    )>,
    mut commands: Commands,
    mut previous_eye: Local<Option<Vec3>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let inverse_frame = source_frame().inverse();
    let eye = inverse_frame.transform_point3(camera.translation());
    let eye_changed = *previous_eye != Some(eye);
    *previous_eye = Some(eye);
    let to_eye = Mat4::from_translation(-eye) * inverse_frame;
    for (entity, transform, lighting, object) in &mut surfaces {
        if !eye_changed && !transform.is_changed() && object.is_some() {
            continue;
        }
        let matrix = to_eye * transform.to_matrix();
        let constants = NativeWorldObject {
            world_rows: std::array::from_fn(|i| matrix.row(i)),
            reflection_lighting_sh0: lighting.0[0],
            reflection_lighting_sh1: lighting.0[1],
            reflection_lighting_sh2: lighting.0[2],
        };
        if let Some(mut object) = object {
            *object = constants;
        } else {
            commands.entity(entity).insert(constants);
        }
    }
}
