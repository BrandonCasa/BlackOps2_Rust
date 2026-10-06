//! Bounded native draw comparison using an external RenderDoc fixture.
//! This qualifies one observed opaque draw, not complete original scene fidelity.
use super::*;
use bevy::camera::visibility::NoFrustumCulling;
use bo2_assets::full_scene::load_full_scene;
use bo2_rendering::tech4::{
    CapturedClip, Tech4Fog, Tech4Material, Tech4MaterialPlugin, Tech4SurfaceRuntime, Tech4Textures,
    prepare_captured_tech4_surface,
};
use std::io::Read;

fn bounded(path: &Path, cap: usize) -> Result<Vec<u8>, DiagnosticError> {
    let file = File::open(path).map_err(|e| io_error("captured input", e))?;
    let length = file
        .metadata()
        .map_err(|e| io_error("captured length", e))?;
    if !length.is_file() || length.len() > cap as u64 {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length.len() as usize)
        .map_err(|_| DiagnosticError::ReadbackLayout)?;
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| io_error("captured read", e))?;
    if bytes.len() as u64 != length.len() {
        return Err(DiagnosticError::ReadbackLayout);
    }
    Ok(bytes)
}
fn floats<const N: usize>(bytes: &[u8], offset: usize) -> Result<[f32; N], DiagnosticError> {
    let slice = bytes
        .get(offset..offset + N * 4)
        .ok_or(DiagnosticError::ReadbackLayout)?;
    let values = std::array::from_fn(|i| {
        f32::from_le_bytes([
            slice[i * 4],
            slice[i * 4 + 1],
            slice[i * 4 + 2],
            slice[i * 4 + 3],
        ])
    });
    if values.iter().any(|f| !f.is_finite()) {
        return Err(DiagnosticError::Preservation);
    }
    Ok(values)
}
fn vector(bytes: &[u8], offset: usize) -> Result<Vec4, DiagnosticError> {
    Ok(Vec4::from_array(floats::<4>(bytes, offset)?))
}

pub fn run(
    scene: &Path,
    fixture: &Path,
    bank: Option<&Path>,
    output: &Path,
) -> Result<(), DiagnosticError> {
    let fixture = fixture
        .canonicalize()
        .map_err(|e| io_error("fixture identity", e))?;
    let scene = scene
        .canonicalize()
        .map_err(|e| io_error("scene identity", e))?;
    outside_workspace(&fixture)?;
    outside_workspace(&scene)?;
    let input = load_full_scene(&scene).map_err(DiagnosticError::Appearance)?;
    let bank = bank
        .map(|path| {
            let path = path
                .canonicalize()
                .map_err(|e| io_error("shader bank", e))?;
            outside_workspace(&path)?;
            bo2_assets::shader_bank::load_shader_bank(&path).map_err(DiagnosticError::Appearance)
        })
        .transpose()?;
    let scene_cb = bounded(&fixture.join("4462-cb-0-0.bin"), 16 * 1024)?;
    let object_cb = bounded(&fixture.join("4462-cb-0-1.bin"), 1024)?;
    let matrix = floats::<16>(&object_cb, 0)?;
    let camera_source = Vec3::new(-matrix[3], -matrix[7], -matrix[11]);
    let fog = Tech4Fog {
        color: vector(&scene_cb, 416)?,
        constants: vector(&scene_cb, 432)?,
        constants2: vector(&scene_cb, 448)?,
        sun_direction: vector(&scene_cb, 464)?,
        sun_color: vector(&scene_cb, 480)?,
        sun_parameters: vector(&scene_cb, 496)?,
        enabled: 1.0,
    };
    let sampler =
        bo2_rendering::appearance::source_sampler(242).map_err(DiagnosticError::Appearance)?;
    let prepared = prepare_captured_tech4_surface(
        &input,
        506,
        Tech4SurfaceRuntime {
            fog,
            hdr_scale: vector(&scene_cb, 320)?.x,
            lightmap_sampler: sampler.clone(),
            probe_sampler: sampler,
        },
        bevy::image::CompressedImageFormats::BC,
    )
    .map_err(|_| DiagnosticError::Preservation)?;
    if prepared.globals.sh0 != vector(&object_cb, 128)?
        || prepared.globals.sh1 != vector(&object_cb, 144)?
        || prepared.globals.sh2 != vector(&object_cb, 160)?
    {
        return Err(DiagnosticError::Preservation);
    }
    // Independently require that the actual bound images equal the source-derived
    // adapter payloads, regardless of legacy versus DX10 DDS headers.
    for (i, image) in prepared.images.iter().enumerate() {
        let bytes = bounded(&fixture.join(format!("texture-{i}.dds")), 128 * 1024 * 1024)?;
        bo2_assets::full_scene::inspect_dds(&bytes).map_err(DiagnosticError::Appearance)?;
        let captured = Image::from_buffer(
            &bytes,
            bevy::image::ImageType::Extension("dds"),
            bevy::image::CompressedImageFormats::BC,
            false,
            bevy::image::ImageSampler::Default,
            RenderAssetUsages::MAIN_WORLD,
        )
        .map_err(|_| DiagnosticError::Preservation)?;
        if image.data != captured.data
            || image.texture_descriptor.format != captured.texture_descriptor.format
            || image.texture_descriptor.size != captured.texture_descriptor.size
            || image.texture_descriptor.mip_level_count
                != captured.texture_descriptor.mip_level_count
        {
            return Err(DiagnosticError::Preservation);
        }
    }
    let clip = CapturedClip {
        view_projection: Mat4::from_cols_array(&floats::<16>(&scene_cb, 576)?).transpose(),
        camera_source,
        enabled: 1.0,
        viewport_depth: Vec2::new(0.0, 0.984375),
    };
    let mut app = configured_app(false, None);
    app.add_plugins(Tech4MaterialPlugin);
    if bank.is_some() {
        app.add_plugins(bo2_rendering::original_material::OriginalMaterialPlugin);
    }
    let report = gpu_probe::render(app, |world| {
        let images = prepared
            .images
            .map(|image| world.resource_mut::<Assets<Image>>().add(image));
        if let Some(bank) = &bank {
            let material =
                translated(world, bank, &fixture, &scene_cb, &object_cb, &images, false)?;
            let material = world
                .resource_mut::<Assets<bo2_rendering::original_material::OriginalMaterial>>()
                .add(material);
            let mesh = bo2_rendering::original_material::original_world_mesh(&input, 506)
                .map_err(|_| DiagnosticError::Preservation)?;
            let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
            world.spawn((
                Mesh3d(mesh.clone()),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                NoFrustumCulling,
            ));
            return Ok(probe_scene(mesh, images, camera_source));
        }
        let [base_color, normal, specular_gloss, lightmap, probe] = images.clone();
        let material = Tech4Material::new_original(
            prepared.globals,
            Tech4Textures {
                base_color,
                normal,
                specular_gloss,
                lightmap,
                probe,
            },
            prepared.fog,
        )
        .and_then(|material| material.with_captured_clip(clip))
        .map_err(|_| DiagnosticError::Preservation)?;
        let material = world.resource_mut::<Assets<Tech4Material>>().add(material);
        let mesh = world.resource_mut::<Assets<Mesh>>().add(prepared.mesh);
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            NoFrustumCulling,
        ));
        Ok(probe_scene(mesh, images, camera_source))
    })?;
    let expected_len = 1920 * 1080 * 4;
    let before = bounded(&fixture.join("rt-before.bin"), expected_len)?;
    let after = bounded(&fixture.join("rt-after.bin"), expected_len)?;
    if before.len() != expected_len || after.len() != expected_len {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let sample_zero = compare(&before, &after, &report.pixels)?;
    let mut before_samples = Vec::new();
    let mut after_samples = Vec::new();
    for sample in 0..4 {
        before_samples.push(bounded(
            &fixture.join(format!("rt-before-sample{sample}.bin")),
            expected_len,
        )?);
        after_samples.push(bounded(
            &fixture.join(format!("rt-after-sample{sample}.bin")),
            expected_len,
        )?);
    }
    let (comparison, partial_pixels) =
        compare_samples(&before_samples, &after_samples, &report.pixels)?;
    let output = new_output(output)?;
    PngEncoder::new(BufWriter::new(exclusive_file(
        &output.join("captured-material.png"),
    )?))
    .write_image(&report.pixels, 1920, 1080, image::ExtendedColorType::Rgba8)
    .map_err(|_| DiagnosticError::Encoding)?;
    let summary = format!(
        "Single original TECH4 draw comparison: translated={}, GPU updates={}, fully covered original MSAA4 pixels={}, channels within2/255={}, mean absolute channel error={:.6}, maximum channel error={}; partial/occluded sample pixels excluded={}. Original sample0 versus reconstructed resolve: pixels={}, maximum error={}. Source506/event4462, exact source attributes/textures/SH, captured VP/fog/HDR, linear target. Fully covered draw response only; boundary coverage, earlier depth occlusion and complete scene fidelity remain unverified.\n",
        bank.is_some(),
        report.updates,
        comparison.pixels,
        comparison.within_two,
        comparison.mean_error,
        comparison.maximum,
        partial_pixels,
        sample_zero.pixels,
        sample_zero.maximum
    );
    exclusive_file(&output.join("comparison.txt"))?
        .write_all(summary.as_bytes())
        .map_err(|e| io_error("comparison write", e))?;
    print!("{summary}");
    if comparison.maximum > 2 {
        return Err(DiagnosticError::Preservation);
    }
    Ok(())
}

fn probe_scene(
    mesh: Handle<Mesh>,
    images: [Handle<Image>; 5],
    camera_source: Vec3,
) -> gpu_probe::ProbeScene {
    gpu_probe::ProbeScene {
        meshes: vec![mesh],
        images: images.into(),
        camera: Transform::from_translation(Vec3::from_array(
            bo2_rendering::coordinates::source_to_bevy(camera_source.to_array()),
        )),
        projection: Projection::Perspective(PerspectiveProjection {
            near: 0.1,
            far: 100_000.0,
            ..default()
        }),
        output: gpu_probe::ProbeOutput {
            width: 1920,
            height: 1080,
            format: TextureFormat::Rgba8Unorm,
            msaa: Msaa::Sample4,
        },
    }
}

/// Second qualified family: native sun-shadow draw on source2546/event4633.
/// The captured atlas is an explicit frozen runtime input, not a generated shadow.
pub fn run_shadow(
    scene: &Path,
    fixture: &Path,
    bank: &Path,
    output: &Path,
) -> Result<(), DiagnosticError> {
    let scene = scene
        .canonicalize()
        .map_err(|e| io_error("scene identity", e))?;
    let fixture = fixture
        .canonicalize()
        .map_err(|e| io_error("fixture identity", e))?;
    let bank = bank
        .canonicalize()
        .map_err(|e| io_error("bank identity", e))?;
    for path in [&scene, &fixture, &bank] {
        outside_workspace(path)?;
    }
    let input = load_full_scene(&scene).map_err(DiagnosticError::Appearance)?;
    let bank =
        bo2_assets::shader_bank::load_shader_bank(&bank).map_err(DiagnosticError::Appearance)?;
    let scene_cb = bounded(&fixture.join("4633-cb-0-0.bin"), 16384)?;
    let object_cb = bounded(&fixture.join("4633-cb-0-1.bin"), 1024)?;
    let matrix = floats::<16>(&object_cb, 0)?;
    let camera_source = Vec3::new(-matrix[3], -matrix[7], -matrix[11]);
    let corners = bounded(&fixture.join("corners.bin"), 216)?;
    let surface = input
        .geometry
        .surfaces()
        .get(2546)
        .ok_or(DiagnosticError::Preservation)?;
    let range = surface.index_range();
    if range.len() != 6 || corners.len() != 216 || input.surfaces[2546].source_vertex_format != 0 {
        return Err(DiagnosticError::Preservation);
    }
    for (corner, index) in input.geometry.indices()[range].iter().enumerate() {
        if floats::<3>(&corners, corner * 36)? != input.geometry.positions()[*index as usize] {
            return Err(DiagnosticError::Preservation);
        }
    }
    let mut loaded = Vec::new();
    for index in 0..6 {
        let bytes = bounded(
            &fixture.join(format!("texture-{index}.dds")),
            256 * 1024 * 1024,
        )?;
        let mut image = Image::from_buffer(
            &bytes,
            bevy::image::ImageType::Extension("dds"),
            bevy::image::CompressedImageFormats::BC,
            false,
            bevy::image::ImageSampler::Default,
            RenderAssetUsages::default(),
        )
        .map_err(|_| DiagnosticError::Preservation)?;
        if index == 3 {
            // Native comparison SRV is R32_FLOAT over a typeless depth atlas.
            // WebGPU requires the corresponding depth texture view class.
            if image.texture_descriptor.format != TextureFormat::R32Float
                || image.texture_descriptor.size.width != 4096
                || image.texture_descriptor.size.height != 8192
                || image.texture_descriptor.mip_level_count != 1
            {
                return Err(DiagnosticError::Preservation);
            }
            image.texture_descriptor.format = TextureFormat::Depth32Float;
        } else {
            bo2_assets::full_scene::inspect_dds(&bytes).map_err(DiagnosticError::Appearance)?;
        }
        loaded.push(image);
    }
    let mut app = configured_app(false, None);
    app.add_plugins(bo2_rendering::original_material::OriginalMaterialPlugin);
    let report = gpu_probe::render(app, |world| {
        let images: Vec<_> = loaded
            .into_iter()
            .map(|image| world.resource_mut::<Assets<Image>>().add(image))
            .collect();
        let material = translated(world, &bank, &fixture, &scene_cb, &object_cb, &images, true)?;
        let material = world
            .resource_mut::<Assets<bo2_rendering::original_material::OriginalMaterial>>()
            .add(material);
        let mesh = bo2_rendering::original_material::original_world_mesh(&input, 2546)
            .map_err(|_| DiagnosticError::Preservation)?;
        let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
        world.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(material),
            Transform::IDENTITY,
            NoFrustumCulling,
        ));
        let mut probe = probe_scene(
            mesh,
            std::array::from_fn(|i| images[i].clone()),
            camera_source,
        );
        probe.images = images;
        Ok(probe)
    })?;
    let mut before = Vec::new();
    let mut after = Vec::new();
    for sample in 0..4 {
        before.push(bounded(
            &fixture.join(format!("rt-before-sample{sample}.bin")),
            1920 * 1080 * 4,
        )?);
        after.push(bounded(
            &fixture.join(format!("rt-after-sample{sample}.bin")),
            1920 * 1080 * 4,
        )?);
    }
    let (comparison, partial) = compare_samples(&before, &after, &report.pixels)?;
    let output = new_output(output)?;
    PngEncoder::new(BufWriter::new(exclusive_file(
        &output.join("captured-shadow.png"),
    )?))
    .write_image(&report.pixels, 1920, 1080, image::ExtendedColorType::Rgba8)
    .map_err(|_| DiagnosticError::Encoding)?;
    let receipt = format!(
        "Native translated shadow draw source2546/event4633: updates={}, fullycovered={}, channelswithin2={}, meanerror={:.6}, maxerror={}, partialexcluded={}; frozen captured depth atlas, original constants/samplers, MSAA4 resolve. Shadow generation and complete scene remain unverified.\n",
        report.updates,
        comparison.pixels,
        comparison.within_two,
        comparison.mean_error,
        comparison.maximum,
        partial
    );
    exclusive_file(&output.join("comparison.txt"))?
        .write_all(receipt.as_bytes())
        .map_err(|e| io_error("comparison write", e))?;
    print!("{receipt}");
    if comparison.maximum > 2 {
        return Err(DiagnosticError::Preservation);
    }
    Ok(())
}

fn translated(
    world: &mut World,
    bank: &bo2_assets::shader_bank::ShaderBankInput,
    fixture: &Path,
    scene_cb: &[u8],
    object_cb: &[u8],
    images: &[Handle<Image>],
    shadow: bool,
) -> Result<bo2_rendering::original_material::OriginalMaterial, DiagnosticError> {
    use bevy::render::render_resource::{
        ColorWrites, CompareFunction, Face, FrontFace, SamplerBindingType, ShaderStages,
        TextureSampleType, TextureViewDimension,
    };
    use bo2_assets::shader_bank::{ResourceKind, ShaderStage, TextureDimension, TextureSample};
    use bo2_rendering::original_material::*;
    use sha2::{Digest, Sha256};
    let locate = |file: &str, stage| {
        let bytes = bounded(&fixture.join(file), 1024 * 1024)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        bank.shaders
            .iter()
            .find(|shader| {
                shader.declaration.source_sha256 == hash && shader.declaration.stage == stage
            })
            .ok_or(DiagnosticError::Preservation)
    };
    let event = if shadow { 4633 } else { 4462 };
    let vs = locate(&format!("{event}-0.dxbc"), ShaderStage::Vertex)?;
    let ps = locate(&format!("{event}-4.dxbc"), ShaderStage::Fragment)?;
    let vertex_shader =
        world
            .resource_mut::<Assets<bevy::shader::Shader>>()
            .add(bevy::shader::Shader::from_wgsl(
                vs.wgsl.clone(),
                "external captured vertex",
            ));
    let fragment_shader =
        world
            .resource_mut::<Assets<bevy::shader::Shader>>()
            .add(bevy::shader::Shader::from_wgsl(
                ps.wgsl.clone(),
                "external captured fragment",
            ));
    let mut bindings = Vec::new();
    let mut resources = Vec::new();
    for shader in [vs, ps] {
        let visibility = if shader.declaration.stage == ShaderStage::Vertex {
            ShaderStages::VERTEX
        } else {
            ShaderStages::FRAGMENT
        };
        for resource in &shader.declaration.resources {
            let binding = resource.binding;
            let kind = match resource.kind {
                ResourceKind::Uniform { size } => {
                    let bytes = match binding {
                        0 | 16 => scene_cb,
                        3 => object_cb,
                        _ => return Err(DiagnosticError::Preservation),
                    };
                    resources.push(OriginalResource::Uniform {
                        binding,
                        bytes: bytes.to_vec(),
                    });
                    BindingKind::Uniform {
                        minimum_size: u64::from(size),
                    }
                }
                ResourceKind::Texture { dimension, sample } => {
                    let index = match binding {
                        32 => 0,
                        33 => 1,
                        34 => 2,
                        41 if shadow => 3,
                        45 => {
                            if shadow {
                                4
                            } else {
                                3
                            }
                        }
                        47 => {
                            if shadow {
                                5
                            } else {
                                4
                            }
                        }
                        _ => return Err(DiagnosticError::Preservation),
                    };
                    resources.push(OriginalResource::Image {
                        binding,
                        image: images[index].clone(),
                    });
                    BindingKind::Texture {
                        dimension: if dimension == TextureDimension::Cube {
                            TextureViewDimension::Cube
                        } else {
                            TextureViewDimension::D2
                        },
                        sample_type: if sample == TextureSample::Depth {
                            TextureSampleType::Depth
                        } else {
                            TextureSampleType::Float { filterable: true }
                        },
                    }
                }
                ResourceKind::Sampler { comparison } => {
                    let source = match binding {
                        64..=66 => 20,
                        77 | 79 => 242,
                        73 if shadow && comparison => 242,
                        _ => return Err(DiagnosticError::Preservation),
                    };
                    let mut descriptor = bo2_rendering::appearance::source_sampler(source)
                        .map_err(DiagnosticError::Appearance)?;
                    if comparison {
                        descriptor.compare = Some(CompareFunction::Greater.into());
                        descriptor.address_mode_w = bevy::image::ImageAddressMode::Repeat;
                    }
                    resources.push(OriginalResource::Sampler {
                        binding,
                        descriptor,
                    });
                    BindingKind::Sampler {
                        kind: if comparison {
                            SamplerBindingType::Comparison
                        } else {
                            SamplerBindingType::Filtering
                        },
                    }
                }
            };
            bindings.push(BindingSpec {
                binding,
                visibility,
                kind,
            });
        }
    }
    let attributes = [
        SourceAttribute::Position4,
        SourceAttribute::Color4,
        SourceAttribute::Uv0,
        SourceAttribute::Normal3,
        SourceAttribute::Tangent3,
        SourceAttribute::Uv1,
    ];
    if vs
        .declaration
        .inputs
        .iter()
        .map(|v| (v.location, v.components))
        .collect::<Vec<_>>()
        != vec![(0, 4), (1, 4), (2, 2), (3, 3), (4, 3), (5, 2)]
    {
        return Err(DiagnosticError::Preservation);
    }
    let descriptor = OriginalMaterialDescriptor {
        vertex_shader,
        fragment_shader,
        vertex_entry: vs.declaration.entry_point.clone(),
        fragment_entry: ps.declaration.entry_point.clone(),
        bindings,
        attributes: attributes
            .into_iter()
            .enumerate()
            .map(|(location, attribute)| VertexInput {
                attribute,
                location: location as u32,
            })
            .collect(),
        raster: OriginalRasterState {
            front_face: FrontFace::Cw,
            cull_mode: Some(Face::Back),
            depth_write: true,
            depth_compare: CompareFunction::GreaterEqual,
            blend: None,
            color_writes: ColorWrites::ALL,
        },
    };
    use bo2_assets::shader_bank::{ArgumentDestination, ArgumentSource, MaterialImage};
    use bo2_rendering::pass_resources::{
        PassResourceContext, QualifiedImplicitBindings, RuntimeTexture, assemble_pass_resources,
    };
    let material_contract = bank
        .materials
        .iter()
        .find(|m| m.scene_material == if shadow { 102 } else { 99 })
        .ok_or(DiagnosticError::Preservation)?;
    let family = bank
        .families
        .get(material_contract.family)
        .ok_or(DiagnosticError::Preservation)?;
    let pass = family
        .techniques
        .iter()
        .filter(|t| t.index == if shadow { 6 } else { 4 })
        .flat_map(|t| &t.passes)
        .find(|p| {
            bank.shaders[p.vertex_shader].declaration.source_sha256 == vs.declaration.source_sha256
                && bank.shaders[p.pixel_shader].declaration.source_sha256
                    == ps.declaration.source_sha256
        })
        .ok_or(DiagnosticError::Preservation)?;
    let mut context = PassResourceContext::default();
    context.initial_uniforms.insert(0, scene_cb.to_vec());
    context.initial_uniforms.insert(16, scene_cb.to_vec());
    context.initial_uniforms.insert(3, object_cb.to_vec());
    context.code_constants.insert(
        213,
        (0..4)
            .map(|row| floats::<4>(object_cb, row * 16))
            .collect::<Result<Vec<_>, _>>()?,
    );
    context.code_constants.insert(
        229,
        (0..4)
            .map(|row| floats::<4>(scene_cb, 576 + row * 16))
            .collect::<Result<Vec<_>, _>>()?,
    );
    for arg in &pass.args {
        if let (
            ArgumentSource::CodeConstant {
                index,
                first_row,
                row_count,
            },
            ArgumentDestination::Uniform { buffer, offset, .. },
        ) = (&arg.source, &arg.destination)
        {
            let binding = u32::from(*buffer)
                + if arg.stage == ShaderStage::Fragment {
                    16
                } else {
                    0
                };
            if descriptor.bindings.iter().any(|b| b.binding == binding) {
                if *first_row != 0 {
                    return Err(DiagnosticError::Preservation);
                }
                let seed = context
                    .initial_uniforms
                    .get(&binding)
                    .ok_or(DiagnosticError::Preservation)?;
                let rows = (0..usize::from(*row_count))
                    .map(|r| floats::<4>(seed, usize::from(*offset) + r * 16))
                    .collect::<Result<Vec<_>, _>>()?;
                if context
                    .code_constants
                    .get(index)
                    .is_some_and(|existing| existing != &rows)
                {
                    return Err(DiagnosticError::Preservation);
                }
                context.code_constants.insert(*index, rows);
            }
        }
        if let (
            ArgumentSource::MaterialTexture { name_hash },
            ArgumentDestination::TextureSampler { texture, .. },
        ) = (&arg.source, &arg.destination)
        {
            let declaration = material_contract
                .textures
                .iter()
                .find(|t| t.name_hash == *name_hash)
                .ok_or(DiagnosticError::Preservation)?;
            let MaterialImage::SceneImage { index } = declaration.image else {
                return Err(DiagnosticError::Preservation);
            };
            let image = images
                .get(usize::from(*texture))
                .ok_or(DiagnosticError::Preservation)?;
            context.scene_images.insert(index, image.clone());
        }
    }
    let lighting_sampler =
        bo2_rendering::appearance::source_sampler(242).map_err(DiagnosticError::Appearance)?;
    if shadow {
        let mut sampler = lighting_sampler.clone();
        sampler.compare = Some(CompareFunction::Greater.into());
        sampler.address_mode_w = bevy::image::ImageAddressMode::Repeat;
        context.code_textures.insert(
            6,
            RuntimeTexture {
                image: images[3].clone(),
                sampler,
            },
        );
    }
    context.qualified_implicit = Some(
        QualifiedImplicitBindings::world_lighting(
            pass,
            Some(RuntimeTexture {
                image: images[if shadow { 4 } else { 3 }].clone(),
                sampler: lighting_sampler.clone(),
            }),
            Some(RuntimeTexture {
                image: images[if shadow { 5 } else { 4 }].clone(),
                sampler: lighting_sampler,
            }),
        )
        .map_err(|_| DiagnosticError::Preservation)?,
    );
    let assembled = assemble_pass_resources(&descriptor, pass, material_contract, &context)
        .map_err(|_| DiagnosticError::Preservation)?;
    // Independently require source-argument assembly to reproduce the captured
    // resource map exactly before allowing the generic runtime boundary.
    if assembled.len() != resources.len()
        || assembled.iter().any(|a| {
            !resources.iter().any(|b| match (a, b) {
                (
                    OriginalResource::Uniform {
                        binding: a,
                        bytes: av,
                    },
                    OriginalResource::Uniform {
                        binding: b,
                        bytes: bv,
                    },
                ) => a == b && av == bv,
                (
                    OriginalResource::Image {
                        binding: a,
                        image: av,
                    },
                    OriginalResource::Image {
                        binding: b,
                        image: bv,
                    },
                ) => a == b && av == bv,
                (
                    OriginalResource::Sampler {
                        binding: a,
                        descriptor: av,
                    },
                    OriginalResource::Sampler {
                        binding: b,
                        descriptor: bv,
                    },
                ) => {
                    a == b
                        && av.address_mode_u == bv.address_mode_u
                        && av.address_mode_v == bv.address_mode_v
                        && av.min_filter == bv.min_filter
                        && av.anisotropy_clamp == bv.anisotropy_clamp
                }
                _ => false,
            })
        })
    {
        return Err(DiagnosticError::Preservation);
    }
    let material = OriginalMaterial::new(
        descriptor,
        assembled,
        &world
            .resource::<bevy::render::renderer::RenderDevice>()
            .limits(),
    )
    .map_err(|_| DiagnosticError::Preservation)?;
    material
        .validate_images(world.resource::<Assets<Image>>())
        .map_err(|_| DiagnosticError::Preservation)?;
    Ok(material)
}

struct Comparison {
    pixels: usize,
    within_two: usize,
    mean_error: f64,
    maximum: u8,
}
// Compare matching MSAA resolves only where this draw changes all four native
// samples. Partially covered/occluded samples retain earlier scene content absent
// from the isolated probe; excluding them is an explicit scope restriction.
fn compare_samples(
    before: &[Vec<u8>],
    after: &[Vec<u8>],
    actual: &[u8],
) -> Result<(Comparison, usize), DiagnosticError> {
    if before.len() != 4
        || after.len() != 4
        || !actual.len().is_multiple_of(4)
        || before
            .iter()
            .chain(after)
            .any(|sample| sample.len() != actual.len())
    {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let mut result = Comparison {
        pixels: 0,
        within_two: 0,
        mean_error: 0.0,
        maximum: 0,
    };
    let mut partial = 0;
    for offset in (0..actual.len()).step_by(4) {
        let changed = (0..4)
            .filter(|&s| before[s][offset..offset + 4] != after[s][offset..offset + 4])
            .count();
        if changed != 4 {
            partial += usize::from(changed != 0);
            continue;
        }
        result.pixels += 1;
        for channel in 0..4 {
            let sum: u16 = after.iter().map(|s| u16::from(s[offset + channel])).sum();
            let resolved = ((sum + 2) / 4) as u8;
            let difference = actual[offset + channel].abs_diff(resolved);
            result.maximum = result.maximum.max(difference);
            result.mean_error += f64::from(difference);
            result.within_two += usize::from(difference <= 2);
        }
    }
    if result.pixels == 0 {
        return Err(DiagnosticError::BackgroundOnly);
    }
    result.mean_error /= (result.pixels * 4) as f64;
    Ok((result, partial))
}
fn compare(before: &[u8], after: &[u8], actual: &[u8]) -> Result<Comparison, DiagnosticError> {
    if before.len() != after.len() || actual.len() != after.len() || !actual.len().is_multiple_of(4)
    {
        return Err(DiagnosticError::ReadbackLayout);
    }
    let mut result = Comparison {
        pixels: 0,
        within_two: 0,
        mean_error: 0.0,
        maximum: 0,
    };
    for ((before, after), actual) in before
        .as_chunks::<4>()
        .0
        .iter()
        .zip(after.as_chunks::<4>().0)
        .zip(actual.as_chunks::<4>().0)
    {
        if before == after {
            continue;
        }
        result.pixels += 1;
        for channel in 0..4 {
            let difference = actual[channel].abs_diff(after[channel]);
            result.maximum = result.maximum.max(difference);
            result.mean_error += f64::from(difference);
            if difference <= 2 {
                result.within_two += 1;
            }
        }
    }
    if result.pixels == 0 {
        return Err(DiagnosticError::BackgroundOnly);
    }
    result.mean_error /= (result.pixels * 4) as f64;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_resolve_requires_all_samples_and_reports_partial_coverage() {
        let before = vec![vec![0, 0, 0, 255, 0, 0, 0, 255]; 4];
        let mut after = vec![vec![8, 12, 16, 255, 8, 12, 16, 255]; 4];
        after[0][4..8].copy_from_slice(&before[0][4..8]);
        let actual = [8, 12, 16, 255, 0, 0, 0, 255];
        let (result, partial) = compare_samples(&before, &after, &actual).unwrap();
        assert_eq!(result.pixels, 1);
        assert_eq!(partial, 1);
        assert_eq!(result.maximum, 0);
        assert!(compare_samples(&before[..3], &after, &actual).is_err());
        assert!(compare_samples(&before, &before, &actual).is_err());
        let mut mismatch = actual;
        mismatch[0] = 20;
        assert_eq!(
            compare_samples(&before, &after, &mismatch)
                .unwrap()
                .0
                .maximum,
            12
        );
    }
    #[test]
    fn comparison_measures_only_native_changed_pixels_and_keeps_mismatches_visible() {
        let before = [1, 2, 3, 255, 4, 5, 6, 255];
        let after = [1, 2, 3, 255, 8, 9, 10, 255];
        let actual = [0, 0, 0, 0, 8, 11, 13, 255];
        let result = compare(&before, &after, &actual).unwrap();
        assert_eq!(result.pixels, 1);
        assert_eq!(result.within_two, 3);
        assert_eq!(result.maximum, 3);
        assert_eq!(result.mean_error, 1.25);
        assert!(compare(&before, &before, &actual).is_err());
    }
}
