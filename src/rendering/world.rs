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
    sky: super::native_sky::SkyDefinition,
    vertex_data_file0: String,
    vertex_data_file1: String,
    index_file: String,
    index_count: usize,
    surfaces: Vec<Surface>,
    models: Vec<BrushModel>,
    lightmaps: Vec<Lightmap>,
    #[serde(flatten)]
    reflection: super::reflection::ReflectionWorld,
    outdoor_image: String,
    outdoor_lookup_matrix: [[f32; 4]; 4],
    primary_lights: Vec<PrimaryLightDefinition>,
    sun_primary_light_index: u32,
    light_definitions: BTreeMap<String, LightDefinition>,
    water_techniques: Vec<WaterTechniqueDefinition>,
    shadow_geometry: Vec<ShadowGeometry>,
    static_model_count: usize,
    #[serde(flatten)]
    shadow_volumes: super::shadow::ShadowWorld,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShadowGeometry {
    surface_indices: Vec<u16>,
    static_model_indices: Vec<u16>,
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
    flags: u8,
    source_surface_index: u32,
    material_draw_surf: u64,
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
    sky: super::native_sky::LoadedSky,
    geometry: NativeWorldGeometry,
    materials: BTreeMap<String, MaterialDefinition>,
    visibility: super::culling::VisibilityWorld,
}

impl WorldDrawDefinition {
    pub(crate) fn reflection_probes(&self) -> &[ReflectionProbe] {
        &self.reflection.probes
    }

    pub(crate) fn load(mut self, root: &Path) -> LoadedWorld {
        self.reflection.validate();
        let visibility = super::culling::VisibilityWorld::load(
            root,
            self.surfaces.len(),
            self.static_model_count,
            self.reflection.cell_count(),
        );
        let read = |path: &str| {
            std::fs::read(root.join(path))
                .unwrap_or_else(|error| panic!("Read original world stream {path}: {error}"))
        };
        let geometry = NativeWorldGeometry {
            primary: read(&self.vertex_data_file0).into(),
            auxiliary: read(&self.vertex_data_file1),
            indices: read(&self.index_file).into(),
            vertex_stride: 36,
        };
        assert_eq!(geometry.indices.len(), self.index_count * 2);
        let mut materials = BTreeMap::new();
        for (index, surface) in self.surfaces.iter_mut().enumerate() {
            assert_eq!(
                surface.source_surface_index as usize, index,
                "Original shadow lists address source surface order"
            );
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
            assert!(surface.reflection_probe_index < self.reflection.probes.len());
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
        assert_eq!(
            self.shadow_geometry.len(),
            self.primary_lights.len(),
            "Original primary-light shadow geometry count"
        );
        for geometry in &self.shadow_geometry {
            assert!(
                geometry
                    .surface_indices
                    .iter()
                    .all(|&index| usize::from(index) < self.surfaces.len()),
                "Original primary-light shadow surface range"
            );
            assert!(
                geometry
                    .static_model_indices
                    .iter()
                    .all(|&index| usize::from(index) < self.static_model_count),
                "Original primary-light shadow static-model range"
            );
        }
        self.shadow_volumes.validate();
        let shadow_only = self
            .surfaces
            .iter()
            .filter(|surface| surface.flags & 8 != 0)
            .count();
        let sun_casters = self
            .surfaces
            .iter()
            .filter(|surface| {
                surface.flags & 1 != 0 && materials[&surface.material_file].casts_shadow()
            })
            .count();
        info!(
            shadow_only,
            sun_casters, "Validated original world shadow inputs; depth rendering remains required"
        );
        let sky = self.sky.load(root);
        LoadedWorld {
            definition: self,
            sky,
            geometry,
            materials,
            visibility,
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

#[derive(Component)]
pub(crate) struct WorldSurfaceColor(pub(crate) Vec4);

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
            .reflection
            .probes
            .iter()
            .map(|probe| super::material::image(server, &probe.reflection_image))
            .collect();
        let mut bound = BTreeMap::new();
        let mut bound_water = BTreeMap::new();
        let mut missing = BTreeMap::<String, String>::new();
        let mut images = HashMap::new();
        let sky_image = self.sky.install(commands, server, geometries);
        images.insert(sky_image.id(), sky_image);
        for surface in &world.surfaces {
            let source = &self.materials[&surface.material_file];
            if source.world_debug_only() {
                continue;
            }
            if surface.flags & 8 != 0 && source.world_shadow_only() {
                continue;
            }
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
            if source.technique_set() == "wpc_cod7water_we876z2f" {
                let Some(lightmap) = lightmaps.get(surface.lightmap_index) else {
                    missing.insert(
                        surface.material_file.clone(),
                        "Original water requires its directional lightmap".to_owned(),
                    );
                    continue;
                };
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
                lightmaps.get(surface.lightmap_index).cloned(),
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

        commands.insert_resource(world.shadow_volumes);
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
        commands.insert_resource(crate::native_fx::lighting::Source {
            primary: world.primary_lights,
        });
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
        let mut decal_world = crate::native_fx::decal::World {
            primary: self.geometry.primary.clone(),
            indices: self.geometry.indices.clone(),
            receivers: Vec::new(),
        };
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
        let mut shadow_draws = Vec::new();
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
            let mut decal_receivers = Vec::new();
            for surface in &world.surfaces[first..first + model.surface_count] {
                if surface.flags & 8 != 0
                    && self.materials[&surface.material_file].world_shadow_only()
                {
                    // These original vertices are absolute source coordinates.
                    // Moving brush casters need their owner's world transform.
                    assert_eq!(index, 0, "Shadow-only brush requires its transform owner");
                    if surface.flags & 1 != 0 {
                        shadow_draws.push(super::shadow::gpu::DepthDraw {
                            primary_offset: surface.vertex_data_offset0,
                            first_index: surface.base_index,
                            index_count: surface.tri_count * 3,
                            bounds: Aabb::from_min_max(
                                Vec3::from_array(surface.bounds[0]),
                                Vec3::from_array(surface.bounds[1]),
                            ),
                        });
                    }
                    continue;
                }
                assert!(
                    index == 0
                        || !matches!(
                            self.materials[&surface.material_file].technique_set(),
                            "wpc_sw4_3d_unlit_1layer_8691j77q"
                                | "wpc_sw4_3d_unlit_3layer_normal_script_q1z1f6w1"
                        ),
                    "Parallax entity requires its original scriptVector0 override owner"
                );
                if self.materials[&surface.material_file].world_debug_only() {
                    continue;
                }
                let key = (
                    surface.material_file.clone(),
                    surface.lightmap_index,
                    surface.reflection_probe_index,
                );
                let sh = &world.reflection.probes[surface.reflection_probe_index].lighting_sh;
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
                if index == 0 {
                    entity.insert(super::culling::BspSurface(
                        surface.source_surface_index as usize,
                    ));
                }
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
                        pass: self.materials[&surface.material_file].world_pass(),
                        // 00442ec0 retains the material key and replaces primary light.
                        // 004438b0 sorts the packed unsigned key; source index breaks ties.
                        sort_key: (surface.material_draw_surf & 0xfff0_0fff_fe00_ffff)
                            | (u64::from(surface.primary_light_index) << 44),
                        source_surface_index: surface.source_surface_index,
                        geometry: geometry.clone(),
                        material: material.clone(),
                        state: *state,
                        primary_byte_offset: surface.vertex_data_offset0,
                        auxiliary_byte_offset: surface.vertex_data_offset1,
                        first_index: surface.base_index,
                        index_count: surface.tri_count * 3,
                    });
                }
                if self.materials[&surface.material_file].receives_marks() {
                    decal_receivers.push(crate::native_fx::decal::Receiver {
                        entity: entity.id(),
                        bounds: surface.bounds.map(Vec3::from_array),
                        vertex_offset: surface.vertex_data_offset0 as usize,
                        first_index: surface.base_index as usize,
                        index_count: surface.tri_count as usize * 3,
                        surface_type_bits: self.materials[&surface.material_file].surface_type_bits,
                        lightmap_index: surface.lightmap_index,
                        probe_index: surface.reflection_probe_index,
                        primary_light_index: surface.primary_light_index,
                        lightmap: lightmaps.get(surface.lightmap_index).cloned(),
                        probe: probes[surface.reflection_probe_index].clone(),
                        lighting: std::array::from_fn(|i| Vec4::from_slice(&sh[i * 4..i * 4 + 4])),
                    });
                }
            }
            if !decal_receivers.is_empty() {
                decal_world.receivers.push(decal_receivers);
            }
        }
        if !shadow_draws.is_empty() {
            commands
                .entity(root)
                .insert(super::shadow::gpu::ShadowCasters {
                    geometry,
                    draws: shadow_draws.into(),
                });
        }
        commands.insert_resource(world.reflection);
        commands.insert_resource(self.visibility);
        commands.insert_resource(decal_world);
    }
}

pub(crate) fn object_constants(
    camera: Query<
        (&GlobalTransform, &bevy::camera::visibility::VisibleEntities),
        With<super::GameCamera>,
    >,
    mut surfaces: Query<(
        &GlobalTransform,
        &WorldSurfaceLighting,
        Option<&WorldSurfaceColor>,
        Option<&mut NativeWorldObject>,
    )>,
    mut commands: Commands,
) {
    let Ok((camera, visible)) = camera.single() else {
        return;
    };
    let inverse_frame = source_frame().inverse();
    let eye = inverse_frame.transform_point3(camera.translation());
    let to_eye = Mat4::from_translation(-eye) * inverse_frame;
    for &entity in [
        std::any::TypeId::of::<NativeWorldSurface>(),
        std::any::TypeId::of::<super::native_model::NativeModelSurface>(),
        std::any::TypeId::of::<NativeWaterSurface>(),
    ]
    .into_iter()
    .flat_map(|class| visible.iter(class))
    {
        let Ok((transform, lighting, color, object)) = surfaces.get_mut(entity) else {
            continue;
        };
        let matrix = to_eye * transform.to_matrix();
        let constants = NativeWorldObject {
            world_rows: std::array::from_fn(|i| matrix.row(i)),
            reflection_lighting_sh0: lighting.0[0],
            reflection_lighting_sh1: lighting.0[1],
            reflection_lighting_sh2: lighting.0[2],
            color: color.map_or(Vec4::ONE, |color| color.0),
        };
        if let Some(mut object) = object {
            if *object != constants {
                *object = constants;
            }
        } else {
            commands.entity(entity).insert(constants);
        }
    }
}
