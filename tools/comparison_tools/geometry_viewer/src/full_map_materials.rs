//! Full-map inspection assembles source materials with explicit frozen capture inputs.
use super::*;
use bo2_assets::{captured_frame::*, full_scene::FullSceneInput, shader_bank::*};
use bo2_rendering::{captured_world::*, original_material::*, pass_resources::*, vertex_routes::*};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct MapDraw {
    pub mesh: Mesh,
    pub material: OriginalMaterial,
    pub dispatch: CapturedDraw,
    pub template_event: u32,
    pub uniforms: BTreeMap<u32, Vec<u8>>,
}
pub(super) struct MapScene {
    pub draws: Vec<MapDraw>,
    pub admitted: usize,
    pub omitted: BTreeMap<String, usize>,
}
struct Recipe<'a> {
    draw: &'a WorldDraw,
    loaded: &'a LoadedWorldDraw,
    pass: &'a ShaderPass,
    family: usize,
    state: [u32; 2],
    technique: usize,
    material: &'a MaterialContract,
    event: u32,
    source_probe: Option<usize>,
    required: BTreeSet<(u8, u32)>,
}
fn cb(stage: ShaderStage, register: u16) -> u32 {
    u32::from(register)
        + if stage == ShaderStage::Fragment {
            16
        } else {
            0
        }
}
fn image_key(binding: &TextureBinding) -> (String, String) {
    (binding.bytes.sha256.clone(), binding.view_format.clone())
}
fn native_texture(
    recipe: &Recipe<'_>,
    texture: u8,
    sampler: u8,
    images: &BTreeMap<(String, String), Handle<Image>>,
) -> Result<RuntimeTexture, String> {
    let binding = recipe
        .draw
        .textures
        .iter()
        .find(|t| t.stage == ShaderStage::Fragment && t.register == texture)
        .ok_or_else(|| "missing captured texture".to_string())?;
    let sampler = recipe
        .draw
        .samplers
        .iter()
        .find(|s| s.stage == ShaderStage::Fragment && s.register == sampler)
        .ok_or_else(|| "missing captured sampler".to_string())?;
    Ok(RuntimeTexture {
        image: images
            .get(&image_key(binding))
            .ok_or_else(|| "missing captured image view".to_string())?
            .clone(),
        sampler: native_sampler(sampler).map_err(|e| format!("{e:?}"))?,
    })
}
fn context(
    recipe: &Recipe<'_>,
    images: &BTreeMap<(String, String), Handle<Image>>,
    scene_images: &BTreeMap<usize, Handle<Image>>,
) -> Result<PassResourceContext, String> {
    let mut context = PassResourceContext {
        scene_images: scene_images.clone(),
        ..default()
    };
    for (binding, bytes) in recipe.draw.uniforms.iter().zip(&recipe.loaded.uniforms) {
        context.initial_uniforms.insert(
            cb(binding.stage, u16::from(binding.register)),
            bytes.to_vec(),
        );
    }
    for argument in &recipe.pass.args {
        let needed = match argument.destination {
            ArgumentDestination::Uniform { buffer, .. } => recipe
                .required
                .contains(&(argument.stage as u8, cb(argument.stage, buffer))),
            ArgumentDestination::TextureSampler { texture, sampler } => {
                recipe
                    .required
                    .contains(&(argument.stage as u8, 32 + u32::from(texture)))
                    || recipe
                        .required
                        .contains(&(argument.stage as u8, 64 + u32::from(sampler)))
            }
        };
        if !needed {
            continue;
        }
        match (&argument.source, &argument.destination) {
            (
                ArgumentSource::CodeConstant {
                    index,
                    first_row,
                    row_count,
                },
                ArgumentDestination::Uniform {
                    buffer,
                    offset,
                    size,
                },
            ) => {
                if *first_row != 0 || usize::from(*size) != usize::from(*row_count) * 16 {
                    return Err("unsupported code row span".into());
                }
                let bytes = context
                    .initial_uniforms
                    .get(&cb(argument.stage, *buffer))
                    .ok_or_else(|| "missing captured code buffer".to_string())?;
                let values = bytes
                    .get(usize::from(*offset)..usize::from(*offset) + usize::from(*size))
                    .ok_or_else(|| "invalid captured code range".to_string())?;
                let rows: Vec<[f32; 4]> = values
                    .as_chunks::<16>()
                    .0
                    .iter()
                    .map(|row| {
                        std::array::from_fn(|i| {
                            f32::from_le_bytes([
                                row[i * 4],
                                row[i * 4 + 1],
                                row[i * 4 + 2],
                                row[i * 4 + 3],
                            ])
                        })
                    })
                    .collect();
                if let Some(prior) = context.code_constants.get(index)
                    && *prior != rows
                {
                    return Err("conflicting captured code constant".into());
                }
                context.code_constants.insert(*index, rows);
            }
            (
                ArgumentSource::CodeTexture { index },
                ArgumentDestination::TextureSampler { texture, sampler },
            ) => {
                context
                    .code_textures
                    .insert(*index, native_texture(recipe, *texture, *sampler, images)?);
            }
            (
                ArgumentSource::MaterialTexture { name_hash },
                ArgumentDestination::TextureSampler { texture, sampler },
            ) => {
                if let Some(source) = recipe
                    .material
                    .textures
                    .iter()
                    .find(|t| t.name_hash == *name_hash)
                    && let MaterialImage::UnresolvedBuiltin { source_sha256 } = &source.image
                {
                    context.builtin_images.insert(
                        source_sha256.clone(),
                        native_texture(recipe, *texture, *sampler, images)?.image,
                    );
                }
            }
            _ => {}
        }
    }
    Ok(context)
}

/// Missing source contracts remain omitted and counted; they never become an unlit material.
pub(super) fn prepare(
    frame: &CapturedFrameInput,
    scene: &FullSceneInput,
    secondary: &[bo2_assets::secondary_stream::SourceSecondaryCorner],
    bank: &ShaderBankInput,
    handles: CapturedHandles<'_>,
    scene_images: &BTreeMap<usize, Handle<Image>>,
    limits: &bevy::render::render_resource::WgpuLimits,
) -> Result<MapScene, DiagnosticError> {
    let shaders = handles.shaders;
    let images = handles.images;
    let mut recipes = Vec::new();
    for receipt in &frame.manifest.draws {
        let Dispatch::World { draw } = &receipt.dispatch else {
            continue;
        };
        let source = draw
            .slices
            .first()
            .and_then(|s| scene.surfaces.get(s.surface))
            .ok_or(DiagnosticError::Preservation)?;
        let material = bank
            .materials
            .iter()
            .find(|m| m.scene_material == source.material)
            .ok_or(DiagnosticError::Preservation)?;
        // The full-map source stream already contains world positions. Captured submodel poses
        // do not establish placement for unrelated source surfaces.
        let expected = [
            1.,
            0.,
            0.,
            -frame.manifest.camera_origin[0],
            0.,
            1.,
            0.,
            -frame.manifest.camera_origin[1],
            0.,
            0.,
            1.,
            -frame.manifest.camera_origin[2],
            0.,
            0.,
            0.,
            1.,
        ];
        if draw.object_matrix != expected {
            continue;
        }
        let vi = bank
            .shaders
            .iter()
            .position(|s| {
                s.declaration.source_sha256 == draw.vertex_shader_sha256
                    && s.declaration.stage == ShaderStage::Vertex
            })
            .ok_or(DiagnosticError::Preservation)?;
        let fi = bank
            .shaders
            .iter()
            .position(|s| {
                s.declaration.source_sha256 == draw.fragment_shader_sha256
                    && s.declaration.stage == ShaderStage::Fragment
            })
            .ok_or(DiagnosticError::Preservation)?;
        let Some((technique, pass)) =
            bank.families[material.family]
                .techniques
                .iter()
                .find_map(|t| {
                    t.passes
                        .iter()
                        .find(|p| p.vertex_shader == vi && p.pixel_shader == fi)
                        .map(|p| (usize::from(t.index), p))
                })
        else {
            continue;
        };
        let Some(state) = material
            .state_entries
            .get(technique)
            .and_then(|entry| material.states.get(usize::from(*entry)))
            .copied()
        else {
            continue;
        };
        let loaded = frame
            .draws
            .iter()
            .find(|d| d.event == receipt.event)
            .ok_or(DiagnosticError::Preservation)?;
        recipes.push(Recipe {
            draw,
            loaded,
            pass,
            family: material.family,
            state,
            technique,
            material,
            event: receipt.event,
            source_probe: source.probe,
            required: [(ShaderStage::Vertex, vi), (ShaderStage::Fragment, fi)]
                .into_iter()
                .flat_map(|(stage, index)| {
                    bank.shaders[index]
                        .declaration
                        .resources
                        .iter()
                        .map(move |r| (stage as u8, r.binding))
                })
                .collect(),
        });
    }
    let mut result = MapScene {
        draws: Vec::new(),
        admitted: 0,
        omitted: BTreeMap::new(),
    };
    type GroupKey = (usize, Option<usize>, Option<usize>, u32, u32);
    let mut groups: BTreeMap<GroupKey, usize> = BTreeMap::new();
    for (surface, source) in scene.surfaces.iter().enumerate() {
        let target = bank
            .materials
            .iter()
            .find(|m| m.scene_material == source.material)
            .ok_or(DiagnosticError::Preservation)?;
        let candidate = recipes.iter().find(|r| {
            r.family == target.family
                && target
                    .state_entries
                    .get(r.technique)
                    .and_then(|entry| target.states.get(usize::from(*entry)))
                    == Some(&r.state)
                && world_vertex_route(bank, r.pass, source.source_vertex_format).is_ok()
        });
        let Some(recipe) = candidate else {
            *result
                .omitted
                .entry("no captured family/state/vertex contract".into())
                .or_default() += 1;
            continue;
        };
        let key = (
            source.material,
            source.lightmap,
            source.probe,
            source.source_vertex_format,
            recipe.event,
        );
        if let Some(group) = groups.get(&key).copied() {
            let route = world_vertex_route(bank, recipe.pass, source.source_vertex_format)
                .map_err(|_| DiagnosticError::Preservation)?;
            let mesh = routed_world_mesh(scene, surface, Some(secondary), &route)
                .map_err(|_| DiagnosticError::Preservation)?;
            result.draws[group]
                .mesh
                .merge(&mesh)
                .map_err(|_| DiagnosticError::Preservation)?;
            result.admitted += 1;
            continue;
        }
        let prepared = (|| -> Result<MapDraw, String> {
            let mut context = context(recipe, images, scene_images)?;
            let secondary_lightmap = if recipe.pass.custom_sampler_flags & 2 != 0 {
                let index = source
                    .lightmap
                    .and_then(|i| scene.lightmaps.get(i))
                    .and_then(|l| l.secondary)
                    .ok_or_else(|| "missing per-surface secondary lightmap".to_string())?;
                let mut texture = native_texture(recipe, 13, 13, images)?;
                texture.image = scene_images
                    .get(&index)
                    .ok_or_else(|| "missing source lightmap image".to_string())?
                    .clone();
                Some(texture)
            } else {
                None
            };
            let reflection_probe = if recipe.pass.custom_sampler_flags & 1 != 0 {
                let index = source
                    .probe
                    .and_then(|i| scene.probes.get(i))
                    .map(|p| p.image)
                    .ok_or_else(|| "missing per-surface probe".to_string())?;
                let mut texture = native_texture(recipe, 15, 15, images)?;
                texture.image = scene_images
                    .get(&index)
                    .ok_or_else(|| "missing source probe image".to_string())?
                    .clone();
                Some(texture)
            } else {
                None
            };
            context.qualified_implicit = Some(
                QualifiedImplicitBindings::world_lighting(
                    recipe.pass,
                    secondary_lightmap,
                    reflection_probe,
                )
                .map_err(|e| format!("{e:?}"))?,
            );
            let route = world_vertex_route(bank, recipe.pass, source.source_vertex_format)
                .map_err(|e| format!("{e:?}"))?;
            let (raster, replay) =
                native_raster(&recipe.draw.pipeline).map_err(|e| format!("{e:?}"))?;
            let mut descriptor = translated_descriptor(
                &bank.shaders[recipe.pass.vertex_shader].declaration,
                &bank.shaders[recipe.pass.pixel_shader].declaration,
                shaders[recipe.pass.vertex_shader].clone(),
                shaders[recipe.pass.pixel_shader].clone(),
                route.clone(),
                raster,
            )
            .map_err(|e| format!("{e:?}"))?;
            descriptor.replay = Some(replay);
            if descriptor.bindings.iter().any(|b| {
                b.binding == 3
                    && matches!(b.kind,BindingKind::Uniform {minimum_size} if minimum_size>=176)
            }) {
                let probe = source
                    .probe
                    .and_then(|i| scene.probes.get(i))
                    .ok_or_else(|| "missing source object SH".to_string())?;
                let buffer = context
                    .initial_uniforms
                    .get_mut(&3)
                    .ok_or_else(|| "missing source object buffer".to_string())?;
                let bytes = buffer
                    .get_mut(128..176)
                    .ok_or_else(|| "missing source object SH span".to_string())?;
                if source.probe != recipe.source_probe {
                    let reference = recipe
                        .source_probe
                        .and_then(|i| scene.probes.get(i))
                        .ok_or_else(|| "unqualified per-probe SH assignment".to_string())?;
                    if bytes
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(reference.sh)
                        .any(|(b, value)| *b != value.to_le_bytes())
                    {
                        return Err("unqualified per-probe SH assignment".into());
                    }
                    for (destination, value) in
                        bytes.as_chunks_mut::<4>().0.iter_mut().zip(probe.sh)
                    {
                        *destination = value.to_le_bytes();
                    }
                }
            }
            let resources = assemble_pass_resources(&descriptor, recipe.pass, target, &context)
                .map_err(|e| format!("{e:?}"))?;
            let uniforms = resources
                .iter()
                .filter_map(|r| {
                    if let OriginalResource::Uniform { binding, bytes } = r {
                        Some((*binding, bytes.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            let material = OriginalMaterial::new(descriptor, resources, limits)
                .map_err(|e| format!("{e:?}"))?;
            let mesh = routed_world_mesh(scene, surface, Some(secondary), &route)
                .map_err(|e| format!("{e:?}"))?;
            let dispatch = native_dispatch(surface as u32, recipe.event, &recipe.draw.pipeline)
                .map_err(|e| format!("{e:?}"))?;
            Ok(MapDraw {
                mesh,
                material,
                dispatch,
                template_event: recipe.event,
                uniforms,
            })
        })();
        match prepared {
            Ok(draw) => {
                groups.insert(key, result.draws.len());
                result.draws.push(draw);
                result.admitted += 1;
            }
            Err(reason) => {
                *result.omitted.entry(reason).or_default() += 1;
            }
        }
    }
    Ok(result)
}
