//! Retained per-sample attachments enter Bevy's main pass through a GPU copy.
use bevy::{
    asset::RenderAssetUsages,
    core_pipeline::{
        Core3dSystems,
        core_3d::{main_opaque_pass_3d, main_transparent_pass_3d},
        schedule::Core3d,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderContext, RenderDevice, ViewQuery},
        texture::GpuImage,
        view::{ViewDepthTexture, ViewTarget},
    },
    shader::Shader,
};
use std::sync::Arc;

const COPY: &str = "
@group(0) @binding(0) var colors: texture_2d_array<f32>;
@group(0) @binding(1) var depths: texture_2d_array<f32>;
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p=array<vec2<f32>,3>(vec2(-1.0,-1.0),vec2(3.0,-1.0),vec2(-1.0,3.0));
    return vec4(p[i],0.0,1.0);
}
struct Result { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 };
@fragment fn fragment(@builtin(position) p: vec4<f32>, @builtin(sample_index) s: u32) -> Result {
    var out: Result;
    out.color=textureLoad(colors,vec2<i32>(p.xy),i32(s),0);
    out.depth=textureLoad(depths,vec2<i32>(p.xy),i32(s),0).r;
    return out;
}
";

#[derive(Resource, Clone, ExtractResource)]
struct InitialSamples {
    color: Handle<Image>,
    depth: Handle<Image>,
    shader: Handle<Shader>,
}
#[derive(Resource, Default)]
struct InitialPipeline(Option<CachedRenderPipelineId>);

pub struct FrameInitializationPlugin;
impl Plugin for FrameInitializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<InitialSamples>::default());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<InitialPipeline>()
                .add_systems(Render, prepare.in_set(RenderSystems::PrepareResources))
                .add_systems(
                    Core3d,
                    initialize
                        .in_set(Core3dSystems::MainPass)
                        .after(main_opaque_pass_3d)
                        .before(main_transparent_pass_3d),
                );
        }
    }
}

/// Color samples are RGBA8; depth records retain the original D32S8 eight-byte layout.
/// This adapter is deliberately limited to one four-sample RGBA8/D32 view.
pub fn register(
    world: &mut World,
    width: u32,
    height: u32,
    color: &[Arc<[u8]>],
    depth: &[Arc<[u8]>],
) -> Result<Vec<Handle<Image>>, crate::DiagnosticError> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .filter(|_| width > 0 && height > 0 && width <= 4096 && height <= 4096)
        .ok_or(crate::DiagnosticError::ReadbackLayout)?;
    if color.len() != 4
        || depth.len() != 4
        || color.iter().any(|s| s.len() != pixels * 4)
        || depth.iter().any(|s| s.len() != pixels * 8)
    {
        return Err(crate::DiagnosticError::ReadbackLayout);
    }
    let mut depth_bytes = Vec::with_capacity(pixels * 16);
    for sample in depth {
        for record in sample.as_chunks::<8>().0 {
            let value = f32::from_le_bytes(
                record[..4]
                    .try_into()
                    .map_err(|_| crate::DiagnosticError::ReadbackLayout)?,
            );
            if !value.is_finite() || !(0.0..=1.0).contains(&value) || record[4..] != [0; 4] {
                return Err(crate::DiagnosticError::Preservation);
            }
            depth_bytes.extend_from_slice(&record[..4]);
        }
    }
    let mut handles = Vec::new();
    for (data, format) in [
        (
            color.iter().flat_map(|s| s.iter().copied()).collect(),
            TextureFormat::Rgba8Unorm,
        ),
        (depth_bytes, TextureFormat::R32Float),
    ] {
        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 4,
            },
            TextureDimension::D2,
            data,
            format,
            RenderAssetUsages::default(),
        );
        image.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..default()
        });
        image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
        handles.push(world.resource_mut::<Assets<Image>>().add(image));
    }
    let shader = world
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wgsl(
            COPY,
            "retained attachment initialization",
        ));
    world.insert_resource(InitialSamples {
        color: handles[0].clone(),
        depth: handles[1].clone(),
        shader,
    });
    Ok(handles)
}
fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "retained attachment initialization",
        &(0..2)
            .map(|binding| BindGroupLayoutEntry {
                binding,
                visibility: ShaderStages::FRAGMENT,
                count: None,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: false },
                    view_dimension: TextureViewDimension::D2Array,
                    multisampled: false,
                },
            })
            .collect::<Vec<_>>(),
    )
}
fn prepare(
    samples: Option<Res<InitialSamples>>,
    mut state: ResMut<InitialPipeline>,
    cache: Res<PipelineCache>,
) {
    let Some(samples) = samples else { return };
    state.0.get_or_insert_with(|| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("retained four-sample attachments".into()),
            layout: vec![layout()],
            vertex: VertexState {
                shader: samples.shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: samples.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Always),
                stencil: default(),
                bias: default(),
            }),
            multisample: MultisampleState {
                count: 4,
                ..default()
            },
            ..default()
        })
    });
}
fn initialize(
    view: ViewQuery<(&ViewTarget, &ViewDepthTexture)>,
    samples: Option<Res<InitialSamples>>,
    state: Res<InitialPipeline>,
    images: Res<RenderAssets<GpuImage>>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    mut context: RenderContext,
) {
    let Some(samples) = samples else { return };
    let Some(pipeline) = state.0.and_then(|id| cache.get_render_pipeline(id)) else {
        return;
    };
    let (Some(color), Some(depth)) = (
        images.get(samples.color.id()),
        images.get(samples.depth.id()),
    ) else {
        return;
    };
    let group = device.create_bind_group(
        "retained attachment initialization",
        &cache.get_bind_group_layout(&layout()),
        &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&color.texture_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(&depth.texture_view),
            },
        ],
    );
    let (target, depth) = view.into_inner();
    let attachments = [Some(target.get_color_attachment())];
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("retained four-sample attachments"),
        color_attachments: &attachments,
        depth_stencil_attachment: Some(depth.get_attachment(StoreOp::Store)),
        ..default()
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &group, &[]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::visibility::NoFrustumCulling;
    use bo2_rendering::original_material::*;
    #[test]
    fn initial_samples_reject_incomplete_depth_and_nonzero_stencil() {
        let mut world = World::new();
        let colors: Vec<Arc<[u8]>> = vec![Arc::from([0u8; 4]); 4];
        let depths: Vec<Arc<[u8]>> = vec![Arc::from([0u8; 8]); 4];
        assert!(register(&mut world, 1, 1, &colors[..3], &depths).is_err());
        let mut invalid = depths.clone();
        invalid[0] = Arc::from([0u8; 4]);
        assert!(register(&mut world, 1, 1, &colors, &invalid).is_err());
        invalid[0] = Arc::from([0, 0, 0, 0, 1, 0, 0, 0]);
        assert!(register(&mut world, 1, 1, &colors, &invalid).is_err());
        let mut record = [0u8; 8];
        record[..4].copy_from_slice(&f32::NAN.to_le_bytes());
        invalid[0] = Arc::from(record);
        assert!(register(&mut world, 1, 1, &colors, &invalid).is_err());
    }
    #[test]
    #[ignore = "requires GPU; synthetic attachments only"]
    fn gpu_initialization_preserves_individual_color_and_depth_samples() {
        let mut app = crate::configured_app(false, None);
        app.add_plugins((
            FrameInitializationPlugin,
            OriginalMaterialPlugin,
            bo2_rendering::captured_world::CapturedWorldPlugin,
        ));
        let report=crate::gpu_probe::render(app,|world| {
            let colors:Vec<Arc<[u8]>>=(0..4).map(|_| [100,0,0,255].repeat(4).into()).collect();
            let depths:Vec<Arc<[u8]>>=[0.25f32,0.75,0.25,0.75].into_iter().map(|d| {
                let mut record=d.to_le_bytes().to_vec(); record.extend_from_slice(&[0;4]);
                record.repeat(4).into()
            }).collect();
            let images=register(world,2,2,&colors,&depths)?;
            let vertex=world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
                "@vertex fn vertex(@location(0) p:vec3<f32>)->@builtin(position) vec4<f32>{return vec4(p,1.0);}","synthetic initialization vertex"));
            let fragment=world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
                "@fragment fn fragment()->@location(0) vec4<f32>{return vec4(0.0,0.0,200.0/255.0,1.0);}","synthetic initialization fragment"));
            let descriptor=OriginalMaterialDescriptor {
                replay:Some(OriginalReplayState {depth_bias:0,slope_bias_bits:0.0f32.to_bits(),clamp_bits:0.0f32.to_bits()}),
                vertex_shader:vertex,fragment_shader:fragment,vertex_entry:"vertex".into(),fragment_entry:"fragment".into(),
                bindings:vec![],attributes:vec![VertexInput {location:0,attribute:SourceAttribute::Position3}],
                raster:OriginalRasterState {front_face:FrontFace::Ccw,cull_mode:None,depth_write:true,
                    depth_compare:CompareFunction::GreaterEqual,blend:None,color_writes:ColorWrites::ALL},
            };
            let mut earlier=descriptor.clone();
            earlier.fragment_shader=world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
                "@fragment fn fragment()->@location(0) vec4<f32>{return vec4(0.0,200.0/255.0,0.0,1.0);}","synthetic earlier draw"));
            let earlier=OriginalMaterial::new(earlier,vec![],&world.resource::<RenderDevice>().limits())
                .map_err(|_| crate::DiagnosticError::Preservation)?;
            let earlier=world.resource_mut::<Assets<OriginalMaterial>>().add(earlier);
            let material=OriginalMaterial::new(descriptor,vec![],&world.resource::<RenderDevice>().limits())
                .map_err(|_| crate::DiagnosticError::Preservation)?;
            let material=world.resource_mut::<Assets<OriginalMaterial>>().add(material);
            let mut mesh=Mesh::new(PrimitiveTopology::TriangleList,RenderAssetUsages::default());
            let positions=vec![[-1.,-1.,0.5],[3.,-1.,0.5],[-1.,3.,0.5]];
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,positions.clone());
            mesh.insert_attribute(SourceAttribute::Position3.attribute(),positions);
            let mesh=world.resource_mut::<Assets<Mesh>>().add(mesh);
            let dispatch=|ordinal| bo2_rendering::captured_world::CapturedDraw {ordinal,event:ordinal+1,
                viewport:[0.,0.,2.,2.,0.,0.984375],scissor:[0,0,2,2],blend_constant:[0.;4]};
            // Spawn opposite receipt order; the later blue draw must win on two samples.
            world.spawn((Mesh3d(mesh.clone()),MeshMaterial3d(material),dispatch(1),Transform::IDENTITY,
                NoFrustumCulling,bevy::render::batching::NoAutomaticBatching));
            world.spawn((Mesh3d(mesh.clone()),MeshMaterial3d(earlier),dispatch(0),Transform::IDENTITY,
                NoFrustumCulling,bevy::render::batching::NoAutomaticBatching));
            Ok(crate::gpu_probe::ProbeScene {meshes:vec![mesh],images,camera:Transform::from_xyz(0.,0.,1.),
                projection:Projection::Perspective(default()),output:crate::gpu_probe::ProbeOutput {
                    width:2,height:2,format:TextureFormat::Rgba8UnormSrgb,msaa:Msaa::Sample4}})
        }).expect("per-sample GPU initialization");
        for pixel in report.pixels.as_chunks::<4>().0 {
            for (actual, expected) in pixel.iter().zip([50u8, 0, 100, 255]) {
                assert!(actual.abs_diff(expected) <= 1, "{pixel:?}");
            }
        }
    }
}
