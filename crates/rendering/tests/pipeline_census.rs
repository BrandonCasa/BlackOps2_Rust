//! Opt-in external shader-bank pipeline acceptance, not a visual fidelity test.
use bevy::{
    app::PluginsState,
    asset::Assets,
    mesh::VertexBufferLayout,
    prelude::*,
    render::{
        RenderApp, RenderPlugin,
        render_resource::*,
        renderer::{RenderAdapter, RenderDevice},
    },
    shader::Shader,
};
use bo2_assets::shader_bank::{
    ResourceKind, ShaderDeclaration, ShaderStage as SourceStage,
    TextureDimension as SourceDimension, TextureSample,
};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};

fn targets(source: &str, entry: &str) -> Result<Vec<Option<ColorTargetState>>, String> {
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    let stage = module
        .entry_points
        .iter()
        .find(|e| e.name == entry && e.stage == naga::ShaderStage::Fragment)
        .ok_or("missing fragment entry")?;
    let Some(result) = stage.function.result.as_ref() else {
        return Ok(Vec::new());
    };
    let mut outputs = Vec::new();
    if let Some(naga::Binding::Location { location, .. }) = &result.binding {
        outputs.push((*location, result.ty));
    } else if let naga::TypeInner::Struct { members, .. } = &module.types[result.ty].inner {
        for member in members {
            if let Some(naga::Binding::Location { location, .. }) = &member.binding {
                outputs.push((*location, member.ty));
            }
        }
    }
    if outputs.iter().any(|(location, _)| *location >= 8) {
        return Err("unsupported color location >=8".into());
    }
    let mut targets = vec![
        None;
        outputs
            .iter()
            .map(|(l, _)| *l as usize + 1)
            .max()
            .unwrap_or(0)
    ];
    for (location, ty) in outputs {
        let kind = match module.types[ty].inner {
            naga::TypeInner::Scalar(s) => s.kind,
            naga::TypeInner::Vector { scalar, .. } => scalar.kind,
            _ => return Err("unsupported color output type".into()),
        };
        let format = match kind {
            naga::ScalarKind::Float => TextureFormat::Rgba8Unorm,
            naga::ScalarKind::Uint => TextureFormat::Rgba32Uint,
            naga::ScalarKind::Sint => TextureFormat::Rgba32Sint,
            _ => return Err("unsupported color scalar".into()),
        };
        targets[location as usize] = Some(ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        });
    }
    Ok(targets)
}
fn layout(
    vs: &ShaderDeclaration,
    ps: &ShaderDeclaration,
) -> Result<BindGroupLayoutDescriptor, String> {
    let mut entries: BTreeMap<u32, BindGroupLayoutEntry> = BTreeMap::new();
    for (shader, visibility) in [(vs, ShaderStages::VERTEX), (ps, ShaderStages::FRAGMENT)] {
        for r in &shader.resources {
            let ty = match r.kind {
                ResourceKind::Uniform { size } => BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(u64::from(size)),
                },
                ResourceKind::Texture { dimension, sample } => BindingType::Texture {
                    sample_type: match sample {
                        TextureSample::Float => TextureSampleType::Float { filterable: true },
                        TextureSample::Depth => TextureSampleType::Depth,
                    },
                    view_dimension: match dimension {
                        SourceDimension::D2 => TextureViewDimension::D2,
                        SourceDimension::Cube => TextureViewDimension::Cube,
                    },
                    multisampled: false,
                },
                ResourceKind::Sampler { comparison } => BindingType::Sampler(if comparison {
                    SamplerBindingType::Comparison
                } else {
                    SamplerBindingType::Filtering
                }),
            };
            if let Some(existing) = entries.get_mut(&r.binding) {
                if existing.ty != ty {
                    return Err(format!("binding {} stage type collision", r.binding));
                }
                existing.visibility |= visibility;
            } else {
                entries.insert(
                    r.binding,
                    BindGroupLayoutEntry {
                        binding: r.binding,
                        visibility,
                        ty,
                        count: None,
                    },
                );
            }
        }
    }
    Ok(BindGroupLayoutDescriptor {
        label: "shader census exact material signature".into(),
        entries: entries.into_values().collect(),
    })
}
fn vertex_buffer(shader: &ShaderDeclaration) -> Result<VertexBufferLayout, String> {
    let mut stride = 0;
    let mut attributes = Vec::new();
    for i in &shader.inputs {
        let format = match i.components {
            1 => VertexFormat::Float32,
            2 => VertexFormat::Float32x2,
            3 => VertexFormat::Float32x3,
            4 => VertexFormat::Float32x4,
            _ => return Err("unsupported vertex component count".into()),
        };
        attributes.push(VertexAttribute {
            format,
            offset: stride,
            shader_location: i.location,
        });
        stride += format.size();
    }
    Ok(VertexBufferLayout {
        array_stride: stride,
        step_mode: VertexStepMode::Vertex,
        attributes,
    })
}
fn synthetic_bindings(
    device: &RenderDevice,
    cache: &PipelineCache,
    signature: &BindGroupLayoutDescriptor,
) -> Result<PreparedBindGroup, String> {
    let mut resources = Vec::new();
    for entry in &signature.entries {
        let resource = match entry.ty {
            BindingType::Buffer {
                min_binding_size: Some(size),
                ..
            } => OwnedBindingResource::Buffer(device.create_buffer_with_data(
                &BufferInitDescriptor {
                    label: Some("synthetic census constants"),
                    contents: &vec![0; size.get().next_multiple_of(16) as usize],
                    usage: BufferUsages::UNIFORM,
                },
            )),
            BindingType::Texture {
                view_dimension,
                sample_type,
                ..
            } => {
                let format = if sample_type == TextureSampleType::Depth {
                    TextureFormat::Depth32Float
                } else {
                    TextureFormat::Rgba8Unorm
                };
                let texture = device.create_texture(&TextureDescriptor {
                    label: Some("synthetic census typed image"),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: if view_dimension == TextureViewDimension::Cube {
                            6
                        } else {
                            1
                        },
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let view = texture.create_view(&TextureViewDescriptor {
                    dimension: Some(view_dimension),
                    ..default()
                });
                OwnedBindingResource::TextureView(view_dimension, view)
            }
            BindingType::Sampler(kind) => OwnedBindingResource::Sampler(
                kind,
                device.create_sampler(&SamplerDescriptor {
                    label: Some("synthetic census sampler"),
                    compare: if kind == SamplerBindingType::Comparison {
                        Some(CompareFunction::Greater)
                    } else {
                        None
                    },
                    mag_filter: FilterMode::Linear,
                    min_filter: FilterMode::Linear,
                    mipmap_filter: MipmapFilterMode::Linear,
                    ..default()
                }),
            ),
            _ => return Err(format!("unsupported binding{}", entry.binding)),
        };
        resources.push((entry.binding, resource));
    }
    let entries = resources
        .iter()
        .map(|(binding, r)| BindGroupEntry {
            binding: *binding,
            resource: r.get_binding(),
        })
        .collect::<Vec<_>>();
    let bind_group = device.create_bind_group(
        "synthetic census",
        &cache.get_bind_group_layout(signature),
        &entries,
    );
    Ok(PreparedBindGroup {
        bindings: BindingResources(resources),
        bind_group,
    })
}

#[test]
fn synthetic_outputs_and_vertex_signature_are_exact_and_malformed_inputs_fail() {
    let source = "struct Out { @location(0) a:vec4<f32>, @location(2) b:vec4<u32> }; @fragment fn main()->Out {return Out(vec4(0.0),vec4(0u));}";
    let output = targets(source, "main").unwrap();
    assert_eq!(output.len(), 3);
    assert!(output[1].is_none());
    assert_eq!(
        output[2].as_ref().unwrap().format,
        TextureFormat::Rgba32Uint
    );
    assert!(targets("invalid", "main").is_err());
    assert!(targets(source, "missing").is_err());
    assert!(
        targets("@fragment fn main() {}", "main")
            .unwrap()
            .is_empty()
    );
    let mut vs = ShaderDeclaration {
        source_sha256: "0".repeat(64),
        file: "synthetic.wgsl".into(),
        sha256: "1".repeat(64),
        stage: SourceStage::Vertex,
        entry_point: "main".into(),
        inputs: vec![
            bo2_assets::shader_bank::VertexInput {
                location: 3,
                components: 2,
            },
            bo2_assets::shader_bank::VertexInput {
                location: 7,
                components: 4,
            },
        ],
        resources: vec![bo2_assets::shader_bank::ShaderResource {
            binding: 0,
            kind: ResourceKind::Uniform { size: 16 },
        }],
    };
    let buffer = vertex_buffer(&vs).unwrap();
    assert_eq!(buffer.array_stride, 24);
    assert_eq!(buffer.attributes[1].offset, 8);
    assert_eq!(buffer.attributes[1].shader_location, 7);
    let mut ps = vs.clone();
    ps.stage = SourceStage::Fragment;
    assert_eq!(
        layout(&vs, &ps).unwrap().entries[0].visibility,
        ShaderStages::VERTEX_FRAGMENT
    );
    ps.resources[0].kind = ResourceKind::Uniform { size: 32 };
    assert!(layout(&vs, &ps).is_err());
    vs.inputs[0].components = 5;
    assert!(vertex_buffer(&vs).is_err());
}

fn gpu_error_path(cache: &mut PipelineCache) {
    let mut assets = Assets::<Shader>::default();
    let vs = Shader::from_wgsl(
        "@vertex fn main()->@builtin(position) vec4<f32>{return vec4(0.0,0.0,0.0,1.0);}",
        "synthetic-census-vertex",
    );
    let vertex = assets.add(vs.clone());
    cache.set_shader(vertex.id(), vs);
    for (source, valid) in [
        (
            "@fragment fn main()->@location(0) vec4<f32>{return vec4(1.0);}",
            true,
        ),
        (
            "@fragment fn main()->@location(0) vec4<f32>{return unknown_identifier;}",
            false,
        ),
    ] {
        let shader = Shader::from_wgsl(source, "synthetic-census-fragment");
        let fragment = assets.add(shader.clone());
        cache.set_shader(fragment.id(), shader);
        let id = cache.queue_render_pipeline(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: vertex.clone(),
                entry_point: Some("main".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: fragment,
                entry_point: Some("main".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        cache.process_queue();
        assert!(
            if valid {
                matches!(
                    cache.get_render_pipeline_state(id),
                    CachedPipelineState::Ok(_)
                )
            } else {
                matches!(
                    cache.get_render_pipeline_state(id),
                    CachedPipelineState::Err(_)
                )
            },
            "synthetic pipeline/error-path classification valid={valid}: {:?}",
            cache.get_render_pipeline_state(id)
        );
    }
}

#[test]
#[ignore = "requires external hash-admitted original shader bank and GPU"]
fn external_original_pipeline_census() {
    let path = std::env::var_os("BO2_SHADER_BANK").expect("BO2_SHADER_BANK");
    let output = std::env::var_os("BO2_PIPELINE_CENSUS_RECEIPT")
        .expect("BO2_PIPELINE_CENSUS_RECEIPT outside repository");
    let output = Path::new(&output);
    let parent = output.parent().unwrap().canonicalize().unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    assert!(!parent.starts_with(workspace), "receipt must stay external");
    let bank = bo2_assets::shader_bank::load_shader_bank(Path::new(&path))
        .expect("external bank admission");
    let started = Instant::now();
    let deadline = Duration::from_secs(600);
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(bevy::window::WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            }),
    );
    while app.plugins_state() == PluginsState::Adding {
        assert!(
            started.elapsed() < deadline,
            "renderer initialization deadline"
        );
        bevy::tasks::tick_global_task_pools_on_main_thread();
        std::thread::yield_now();
    }
    app.finish();
    app.cleanup();
    let render = app.get_sub_app_mut(RenderApp).unwrap().world_mut();
    let device = render.resource::<RenderDevice>().clone();
    let adapter = render.resource::<RenderAdapter>().clone();
    let mut cache = PipelineCache::new(device.clone(), adapter, true);
    gpu_error_path(&mut cache);
    let mut shader_assets = Assets::<Shader>::default();
    let mut handles = Vec::new();
    for shader in &bank.shaders {
        let asset = Shader::from_wgsl(shader.wgsl.clone(), shader.declaration.file.clone());
        let handle = shader_assets.add(asset.clone());
        cache.set_shader(handle.id(), asset);
        handles.push(handle);
    }
    let mut receipt =
        String::from("family\ttechnique\tpass\tvertex\tfragment\tcolor_targets\tstatus\n");
    let mut total = 0;
    let mut ready = 0;
    let mut failed = 0;
    let mut multiple_targets = 0;
    let mut pairs: BTreeMap<(usize, usize), Result<(usize, String), String>> = BTreeMap::new();
    for (family_id, family) in bank.families.iter().enumerate() {
        for technique in &family.techniques {
            for (pass_id, pass) in technique.passes.iter().enumerate() {
                total += 1;
                let vs = &bank.shaders[pass.vertex_shader];
                let ps = &bank.shaders[pass.pixel_shader];
                assert_eq!(vs.declaration.stage, SourceStage::Vertex);
                assert_eq!(ps.declaration.stage, SourceStage::Fragment);
                let pair = (pass.vertex_shader, pass.pixel_shader);
                let result = if let Some(existing) = pairs.get(&pair) {
                    existing.clone()
                } else {
                    (|| -> Result<(usize, String), String> {
                        if started.elapsed() >= deadline {
                            return Err("census deadline: pair not attempted".into());
                        }
                        let signature = layout(&vs.declaration, &ps.declaration)?;
                        let _bound = synthetic_bindings(&device, &cache, &signature)?;
                        let targets = targets(&ps.wgsl, &ps.declaration.entry_point)?;
                        let target_count = targets.iter().flatten().count();
                        let mut descriptor = RenderPipelineDescriptor {
                            label: Some("external original pair census".into()),
                            vertex: VertexState {
                                shader: handles[pass.vertex_shader].clone(),
                                entry_point: Some(vs.declaration.entry_point.clone().into()),
                                buffers: vec![vertex_buffer(&vs.declaration)?],
                                ..default()
                            },
                            fragment: Some(FragmentState {
                                shader: handles[pass.pixel_shader].clone(),
                                entry_point: Some(ps.declaration.entry_point.clone().into()),
                                targets,
                                ..default()
                            }),
                            depth_stencil: Some(DepthStencilState {
                                format: TextureFormat::Depth32Float,
                                depth_write_enabled: Some(true),
                                depth_compare: Some(CompareFunction::GreaterEqual),
                                stencil: default(),
                                bias: default(),
                            }),
                            ..default()
                        };
                        descriptor.set_layout(3, signature);
                        let id = cache.queue_render_pipeline(descriptor);
                        cache.process_queue();
                        loop {
                            match cache.get_render_pipeline_state(id) {
                                CachedPipelineState::Ok(_) => {
                                    return Ok((target_count, "READY".into()));
                                }
                                CachedPipelineState::Err(error) => {
                                    return Err(format!("{error:?}"));
                                }
                                _ => {
                                    if started.elapsed() >= deadline {
                                        return Err("pipeline deadline".into());
                                    }
                                    cache.process_queue();
                                    std::thread::yield_now();
                                }
                            }
                        }
                    })()
                };
                pairs.insert(pair, result.clone());
                match result {
                    Ok((count, status)) => {
                        ready += 1;
                        if count > 1 {
                            multiple_targets += 1;
                        }
                        receipt.push_str(&format!(
                            "{family_id}\t{}\t{pass_id}\t{}\t{}\t{count}\t{status}\n",
                            technique.index, pass.vertex_shader, pass.pixel_shader
                        ));
                    }
                    Err(error) => {
                        failed += 1;
                        receipt.push_str(&format!(
                            "{family_id}\t{}\t{pass_id}\t{}\t{}\t?\tFAILED {}\n",
                            technique.index,
                            pass.vertex_shader,
                            pass.pixel_shader,
                            error.replace(['\n', '\r', '\t'], " ")
                        ));
                    }
                }
            }
        }
        std::fs::write(output, &receipt).unwrap();
        println!("census family {family_id}: {ready}/{total} ready, {failed} failures");
    }
    receipt.push_str(&format!("TOTAL {total} READY {ready} FAILED {failed} FAMILIES {} MRT {multiple_targets} ELAPSED {:?}\n",bank.families.len(),started.elapsed()));
    std::fs::write(output, receipt).unwrap();
    println!(
        "GPU pipeline census: {ready}/{total} ready; {failed} failures; {} families; {multiple_targets} MRT passes",
        bank.families.len()
    );
    assert_eq!(failed, 0, "exact errors saved in external receipt");
}
