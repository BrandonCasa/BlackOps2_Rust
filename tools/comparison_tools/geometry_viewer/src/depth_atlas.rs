//! Upload measured float depth data using a Bevy GPU pass; CPU depth copies are forbidden.
use bevy::{
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_asset::RenderAssets,
        render_resource::*,
        renderer::{RenderDevice, RenderQueue, render_system},
        texture::GpuImage,
    },
    shader::Shader,
};
use std::collections::HashSet;

const COPY: &str = "
@group(0) @binding(0) var source: texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(p[i], 0.0, 1.0);
}
@fragment fn fragment(@builtin(position) p: vec4<f32>) -> @builtin(frag_depth) f32 {
    return textureLoad(source, vec2<i32>(p.xy), 0).r;
}
";

#[derive(Resource, Clone, ExtractResource)]
pub struct DepthAtlasCopies {
    pub shader: Handle<Shader>,
    pub images: Vec<(Handle<Image>, Handle<Image>)>,
}
#[derive(Resource, Default)]
struct CopyState {
    pipeline: Option<CachedRenderPipelineId>,
    done: HashSet<bevy::asset::AssetId<Image>>,
}
pub struct DepthAtlasPlugin;
impl Plugin for DepthAtlasPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<DepthAtlasCopies>::default());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.init_resource::<CopyState>().add_systems(
                Render,
                upload.in_set(RenderSystems::Render).before(render_system),
            );
        }
    }
}
pub fn register(world: &mut World, images: Vec<(Handle<Image>, Handle<Image>)>) {
    let shader = world
        .resource_mut::<Assets<Shader>>()
        .add(Shader::from_wgsl(COPY, "float depth upload"));
    world.insert_resource(DepthAtlasCopies { shader, images });
}
fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "float depth upload",
        &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::FRAGMENT,
            count: None,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: false },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
        }],
    )
}
fn upload(
    copies: Option<Res<DepthAtlasCopies>>,
    images: Res<RenderAssets<GpuImage>>,
    mut state: ResMut<CopyState>,
    cache: Res<PipelineCache>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let Some(copies) = copies else { return };
    let id = *state.pipeline.get_or_insert_with(|| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("float depth upload".into()),
            layout: vec![layout()],
            vertex: VertexState {
                shader: copies.shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: copies.shader.clone(),
                entry_point: Some("fragment".into()),
                targets: vec![],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::Always),
                stencil: default(),
                bias: default(),
            }),
            ..default()
        })
    });
    let Some(pipeline) = cache.get_render_pipeline(id) else {
        return;
    };
    for (source, target) in &copies.images {
        if state.done.contains(&target.id()) {
            continue;
        }
        let (Some(source), Some(target_image)) = (images.get(source.id()), images.get(target.id()))
        else {
            continue;
        };
        let group = device.create_bind_group(
            "float depth upload",
            &cache.get_bind_group_layout(&layout()),
            &[BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(&source.texture_view),
            }],
        );
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("float depth upload"),
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("float depth upload"),
                color_attachments: &[],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &target_image.texture_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &*group, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([encoder.finish()]);
        state.done.insert(target.id());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{asset::RenderAssetUsages, camera::visibility::NoFrustumCulling};
    use bo2_rendering::original_material::*;

    #[test]
    #[ignore = "requires GPU; synthetic data only"]
    fn gpu_copy_preserves_float_depth_and_texel_orientation() {
        let mut app = crate::configured_app(false, None);
        app.add_plugins((DepthAtlasPlugin, OriginalMaterialPlugin));
        let report = crate::gpu_probe::render(app, |world| {
            let values = [0.0f32, 0.25, 0.75, 1.0];
            let mut source = Image::new(
                Extent3d { width: 2, height: 2, depth_or_array_layers: 1 },
                TextureDimension::D2,
                values.into_iter().flat_map(f32::to_le_bytes).collect(),
                TextureFormat::R32Float, RenderAssetUsages::default(),
            );
            source.texture_descriptor.usage = TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING;
            let mut target = source.clone();
            target.data = None;
            target.texture_descriptor.format = TextureFormat::Depth32Float;
            target.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT;
            let source = world.resource_mut::<Assets<Image>>().add(source);
            let target = world.resource_mut::<Assets<Image>>().add(target);
            register(world, vec![(source.clone(), target.clone())]);
            let vertex = world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
                "@vertex fn vertex(@location(0) p: vec3<f32>) -> @builtin(position) vec4<f32> { return vec4(p, 1.0); }", "synthetic depth probe vertex"));
            let fragment = world.resource_mut::<Assets<Shader>>().add(Shader::from_wgsl(
                "@group(3) @binding(32) var source: texture_depth_2d; @fragment fn fragment(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> { let v=textureLoad(source, vec2<i32>(p.xy), 0); return vec4(v,v,v,1.0); }", "synthetic depth probe fragment"));
            let descriptor = OriginalMaterialDescriptor {
                replay: None,
                vertex_shader: vertex, fragment_shader: fragment,
                vertex_entry: "vertex".into(), fragment_entry: "fragment".into(),
                bindings: vec![BindingSpec { binding: 32, visibility: ShaderStages::FRAGMENT,
                    kind: BindingKind::Texture { dimension: TextureViewDimension::D2, sample_type: TextureSampleType::Depth } }],
                attributes: vec![VertexInput { location: 0, attribute: SourceAttribute::Position3 }],
                raster: OriginalRasterState { front_face: FrontFace::Ccw, cull_mode: None,
                    depth_write: false, depth_compare: CompareFunction::Always, blend: None, color_writes: ColorWrites::ALL },
            };
            let limits = world.resource::<bevy::render::renderer::RenderDevice>().limits();
            let material = OriginalMaterial::new(descriptor, vec![OriginalResource::Image { binding:32, image:target.clone() }], &limits)
                .map_err(|_| crate::DiagnosticError::Preservation)?;
            let material = world.resource_mut::<Assets<OriginalMaterial>>().add(material);
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
            let positions = vec![[-1., -1., 0.], [3., -1., 0.], [-1., 3., 0.]];
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions.clone());
            mesh.insert_attribute(SourceAttribute::Position3.attribute(), positions);
            let mesh = world.resource_mut::<Assets<Mesh>>().add(mesh);
            world.spawn((Mesh3d(mesh.clone()), MeshMaterial3d(material), Transform::IDENTITY, NoFrustumCulling));
            Ok(crate::gpu_probe::ProbeScene { meshes: vec![mesh], images: vec![source,target],
                camera: Transform::from_xyz(0.,0.,1.), projection: Projection::Perspective(default()),
                output: crate::gpu_probe::ProbeOutput { width:2,height:2,format:TextureFormat::Rgba8Unorm,msaa:Msaa::Off } })
        }).expect("synthetic GPU depth upload");
        for (pixel, value) in report
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .zip([0u8, 64, 191, 255])
        {
            assert!(pixel[..3].iter().all(|c| c.abs_diff(value) <= 1));
            assert_eq!(pixel[3], 255);
        }
    }
}
