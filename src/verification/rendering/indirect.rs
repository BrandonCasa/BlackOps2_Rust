use bevy::{
    prelude::*,
    remote::{BrpError, BrpResult},
    render::renderer::{RenderAdapter, RenderDevice, RenderQueue},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    indexed: bool,
    capacity: u32,
    gpu_count: u32,
    live_command: Option<u32>,
    #[serde(default)]
    argument_offset: u64,
    #[serde(default)]
    invalid: bool,
    #[serde(default)]
    direct_after: bool,
}

fn error(message: impl ToString) -> BrpError {
    BrpError {
        code: -32602,
        message: message.to_string(),
        data: None,
    }
}

pub(super) fn probe(
    In(params): In<Option<Value>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    adapter: Res<RenderAdapter>,
) -> BrpResult {
    let device = device.wgpu_device();
    let limits = device.limits();
    let Some(params) = params else {
        return Ok(json!({"max_buffer_size": limits.max_buffer_size,
            "source_window_bytes": adapter.limits().max_storage_buffer_binding_size,
            "counted": device.features().contains(wgpu::Features::MULTI_DRAW_INDIRECT_COUNT)}));
    };
    let request: Request = serde_json::from_value(params).map_err(error)?;
    let stride = if request.indexed { 20 } else { 16 };
    let size = request
        .argument_offset
        .checked_add(u64::from(request.capacity) * stride)
        .ok_or_else(|| error("Argument size overflow"))?
        .max(20);
    if size > limits.max_buffer_size
        || request.capacity > 16_777_216
        || !request.argument_offset.is_multiple_of(4)
        || request
            .live_command
            .is_some_and(|index| index >= request.capacity)
    {
        return Err(error("Indirect probe exceeds device limits"));
    }
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let buffer = |label, size, usage| {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage,
            mapped_at_creation: false,
        })
    };
    let arguments = buffer(
        "indirect probe arguments",
        size,
        wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::COPY_DST,
    );
    let count = buffer(
        "indirect probe GPU count",
        8,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
    );
    let parameter = buffer(
        "indirect probe requested count",
        4,
        wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    );
    queue.write_buffer(&parameter, 0, &request.gpu_count.to_le_bytes());
    if let Some(index) = request.live_command {
        let values = if request.indexed {
            vec![
                3u32,
                1,
                if request.invalid { 3 } else { 1 },
                (-1i32) as u32,
                11,
            ]
        } else {
            vec![if request.invalid { 8 } else { 3 }, 1, 7, 11]
        };
        let bytes: Vec<u8> = values
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        queue.write_buffer(
            &arguments,
            request.argument_offset + u64::from(index) * stride,
            &bytes,
        );
    }
    let index_buffer = buffer(
        "indirect probe indices",
        16,
        wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
    );
    let indices: Vec<u8> = [0u32, 8, 9, 10]
        .into_iter()
        .flat_map(u32::to_le_bytes)
        .collect();
    queue.write_buffer(&index_buffer, 0, &indices);
    let vertices = buffer(
        "indirect probe vertex bounds",
        160,
        wgpu::BufferUsages::VERTEX,
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("indirect probe"), source: wgpu::ShaderSource::Wgsl(r#"
struct Out { @builtin(position) position: vec4f, @location(0) color: vec4f }
@vertex fn vertex(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> Out {
    let positions = array(vec2f(-1., -1.), vec2f(3., -1.), vec2f(-1., 3.));
    var result: Out;
    result.position = vec4f(positions[(vertex - 7u) % 3u], 0., 1.);
    result.color = select(vec4f(1., 0., 0., 1.), vec4f(0., 1., 0., 1.),
        vertex >= 7u && vertex < 10u && instance == 11u);
    return result;
}
@fragment fn fragment(input: Out) -> @location(0) vec4f { return input.color; }
@group(0) @binding(0) var<storage, read> requested: array<u32>;
@group(0) @binding(1) var<storage, read_write> count: array<u32>;
@compute @workgroup_size(1) fn generate_count() { count[1] = requested[0]; }
"#.into()),
    });
    let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("indirect probe"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: 16,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[],
            })],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("indirect probe GPU count"),
        layout: None,
        module: &shader,
        entry_point: Some("generate_count"),
        compilation_options: Default::default(),
        cache: None,
    });
    let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("indirect probe GPU count"),
        layout: &compute.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: parameter.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: count.as_entire_binding(),
            },
        ],
    });
    let extent = wgpu::Extent3d {
        width: 1,
        height: 1,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("indirect probe result"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let readback = buffer(
        "indirect probe readback",
        256,
        wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    );
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&compute);
        pass.set_bind_group(0, &bindings, &[]);
        pass.dispatch_workgroups(1, 1, 1);
    }
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&render);
        pass.set_vertex_buffer(0, vertices.slice(..));
        if request.direct_after {
            pass.draw(7..10, 11..12);
        }
        if request.indexed {
            pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.multi_draw_indexed_indirect_count(
                &arguments,
                request.argument_offset,
                &count,
                4,
                request.capacity,
            );
        } else {
            pass.multi_draw_indirect_count(
                &arguments,
                request.argument_offset,
                &count,
                4,
                request.capacity,
            );
        }
        if request.direct_after {
            pass.draw(7..10, 11..12);
        }
    }
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        extent,
    );
    queue.submit([encoder.finish()]);
    let (sender, receiver) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(60)),
        })
        .map_err(error)?;
    receiver
        .recv_timeout(Duration::from_secs(5))
        .map_err(error)?
        .map_err(error)?;
    if let Some(failure) = bevy::tasks::block_on(scope.pop()) {
        return Err(error(failure));
    }
    let pixel = readback.slice(..).get_mapped_range().map_err(error)?[..4].to_vec();
    readback.unmap();
    Ok(
        json!({"pixel": pixel, "argument_bytes": size, "capacity": request.capacity,
        "gpu_count": request.gpu_count, "indexed": request.indexed, "gpu_count_generation": true}),
    )
}
