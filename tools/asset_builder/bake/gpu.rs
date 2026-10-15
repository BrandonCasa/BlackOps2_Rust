use super::Recipe;
use crate::core::*;
use std::{
    future::Future,
    sync::{Arc, OnceLock},
    task::{Context, Poll, Wake, Waker},
    time::Duration,
};
use wgpu::util::DeviceExt;

pub(super) type Sample = [f32; 20];
pub(super) type Pixel = [[f32; 4]; 4];

fn wait<T>(future: impl Future<Output = T>) -> T {
    struct Thread(std::thread::Thread);
    impl Wake for Thread {
        fn wake(self: Arc<Self>) {
            self.0.unpark();
        }
    }
    let waker = Waker::from(Arc::new(Thread(std::thread::current())));
    let mut context = Context::from_waker(&waker);
    let mut future = std::pin::pin!(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => return value,
            Poll::Pending => std::thread::park(),
        }
    }
}

struct Compute {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    script_pipeline: wgpu::ComputePipeline,
}

impl Compute {
    fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = wait(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))?;
        if matches!(adapter.get_info().device_type, wgpu::DeviceType::Cpu) {
            return Err("Material baking requires a hardware GPU".into());
        }
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Origins material baker"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))?;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Existing layered material equations"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bake.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Layered material bake"),
            layout: None,
            module: &shader,
            entry_point: Some("bake"),
            compilation_options: Default::default(),
            cache: None,
        });
        let script_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Precomposed animation frame"),
            layout: None,
            module: &shader,
            entry_point: Some("script_frame"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(error) = wait(errors.pop()) {
            return Err(error.into());
        }
        println!(
            "GPU material baking: {} ({:?}); {} CPU workers",
            adapter.get_info().name,
            adapter.get_info().backend,
            std::thread::available_parallelism().map_or(1, usize::from)
        );
        Ok(Self {
            device,
            queue,
            pipeline,
            script_pipeline,
        })
    }
}

pub(super) struct Baker {
    compute: &'static Compute,
    parameters: wgpu::Buffer,
    samples: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    bindings: wgpu::BindGroup,
    script_bindings: wgpu::BindGroup,
    capacity: usize,
    image_count: usize,
}

impl Baker {
    pub(super) fn new(recipe: &Recipe) -> Result<Self> {
        static COMPUTE: OnceLock<std::result::Result<Compute, String>> = OnceLock::new();
        let compute = COMPUTE
            .get_or_init(|| Compute::new().map_err(|error| error.to_string()))
            .as_ref()
            .map_err(|error| format!("GPU material baking initialization failed: {error}"))?;
        let (parameters, image_data) = recipe.gpu_data()?;
        let device = &compute.device;
        if image_data.len() as u64 > device.limits().max_storage_buffer_binding_size as u64 {
            return Err("Layered source images exceed GPU storage buffer limit".into());
        }
        let images = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Decoded native images"),
            contents: &image_data,
            usage: wgpu::BufferUsages::STORAGE,
        });
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Native material recipe"),
            contents: &parameters,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let capacity = 262_144_usize
            .min(device.limits().max_storage_buffer_binding_size as usize / 80)
            .min(device.limits().max_compute_workgroups_per_dimension as usize * 64);
        if capacity == 0 {
            return Err("GPU cannot dispatch material bake samples".into());
        }
        let buffer = |label, size, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let samples = buffer(
            "Bake sample coordinates",
            (capacity * 80) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        );
        let output = buffer(
            "Baked PBR samples",
            (capacity * 64) as u64,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = buffer(
            "Bake readback",
            (capacity * 64) as u64,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let entries: Vec<_> = [&parameters, &images, &samples, &output]
            .into_iter()
            .enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry {
                binding: binding as u32,
                resource: buffer.as_entire_binding(),
            })
            .collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material bake inputs"),
            layout: &compute.pipeline.get_bind_group_layout(0),
            entries: &entries,
        });
        let script_bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Animation frame inputs"),
            layout: &compute.script_pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: parameters.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: images.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output.as_entire_binding(),
                },
            ],
        });
        Ok(Self {
            compute,
            parameters,
            samples,
            output,
            readback,
            bindings,
            script_bindings,
            capacity,
            image_count: recipe.inputs.len(),
        })
    }

    pub(super) fn set_script_frame(&self, transforms: &[[f32; 6]], state: [f32; 4]) -> Result<()> {
        if transforms.len() > self.image_count
            || transforms
                .iter()
                .flatten()
                .chain(state.iter())
                .any(|value| !value.is_finite())
        {
            return Err("Invalid baked script-frame controls".into());
        }
        let mut bytes = Vec::with_capacity(9 * 32);
        for index in 0..9 {
            for value in transforms
                .get(index)
                .copied()
                .unwrap_or([1., 0., 0., 1., 0., 0.])
                .into_iter()
                .chain([0., 0.])
            {
                push32(&mut bytes, value.to_bits());
            }
        }
        self.compute
            .queue
            .write_buffer(&self.parameters, 256, &bytes);
        let mut bytes = Vec::with_capacity(16);
        for value in state {
            push32(&mut bytes, value.to_bits());
        }
        self.compute
            .queue
            .write_buffer(&self.parameters, 64, &bytes);
        Ok(())
    }

    pub(super) fn sample(&self, samples: &[Sample]) -> Result<Vec<Pixel>> {
        let mut pixels = Vec::with_capacity(samples.len());
        for batch in samples.chunks(self.capacity) {
            let mut bytes = Vec::with_capacity(batch.len() * 80);
            for sample in batch {
                for value in sample {
                    push32(&mut bytes, value.to_bits());
                }
            }
            self.compute.queue.write_buffer(&self.samples, 0, &bytes);
            self.compute.queue.write_buffer(
                &self.parameters,
                4,
                &(batch.len() as u32).to_le_bytes(),
            );
            let mut commands =
                self.compute
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Material bake dispatch"),
                    });
            {
                let mut pass = commands.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Material bake"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.compute.pipeline);
                pass.set_bind_group(0, &self.bindings, &[]);
                pass.dispatch_workgroups((batch.len() as u32).div_ceil(64), 1, 1);
            }
            let size = (batch.len() * 64) as u64;
            commands.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, size);
            let submission = self.compute.queue.submit([commands.finish()]);
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            self.readback
                .map_async(wgpu::MapMode::Read, 0..size, move |result| {
                    let _ = sender.send(result);
                });
            self.compute.device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(60)),
            })?;
            receiver.recv_timeout(Duration::from_secs(1))??;
            let decoded = (|| -> Result<()> {
                let mapped = self.readback.get_mapped_range(0..size)?;
                for bytes in mapped.chunks_exact(64) {
                    let pixel: Pixel = std::array::from_fn(|role| {
                        std::array::from_fn(|channel| {
                            f32::from_bits(u32_at(bytes, role * 16 + channel * 4))
                        })
                    });
                    if pixel[0][0].to_bits() == 0x7fc00001 {
                        return Err("Layered reveal thresholds have no interval".into());
                    }
                    if pixel.iter().flatten().any(|value| !value.is_finite()) {
                        return Err("GPU material bake produced a non-finite channel".into());
                    }
                    pixels.push(pixel);
                }
                Ok(())
            })();
            self.readback.unmap();
            decoded?;
        }
        Ok(pixels)
    }

    pub(super) fn script_frame(
        &self,
        width: usize,
        height: usize,
        bounds: [[f32; 2]; 2],
    ) -> Result<Vec<[f32; 4]>> {
        let count = width
            .checked_mul(height)
            .filter(|count| *count <= u32::MAX as usize)
            .ok_or("Animation frame dimensions exceed the GPU address range")?;
        if width == 0 || height == 0 || bounds.iter().flatten().any(|value| !value.is_finite()) {
            return Err("Invalid animation frame UV domain".into());
        }
        let mut domain = Vec::new();
        for value in [
            bounds[0][0],
            bounds[0][1],
            bounds[1][0] - bounds[0][0],
            bounds[1][1] - bounds[0][1],
        ] {
            push32(&mut domain, value.to_bits());
        }
        self.compute
            .queue
            .write_buffer(&self.parameters, 32, &domain);
        let mut pixels = Vec::with_capacity(count);
        for offset in (0..count).step_by(self.capacity) {
            let batch = (count - offset).min(self.capacity);
            self.compute
                .queue
                .write_buffer(&self.parameters, 4, &(batch as u32).to_le_bytes());
            let mut slots = Vec::new();
            for value in [width as u32, offset as u32, height as u32, 0] {
                push32(&mut slots, value);
            }
            self.compute
                .queue
                .write_buffer(&self.parameters, 96, &slots);
            let mut commands =
                self.compute
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Animation frame dispatch"),
                    });
            {
                let mut pass = commands.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Precomposed frame"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.compute.script_pipeline);
                pass.set_bind_group(0, &self.script_bindings, &[]);
                pass.dispatch_workgroups((batch as u32).div_ceil(64), 1, 1);
            }
            let size = (batch * 16) as u64;
            commands.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, size);
            let submission = self.compute.queue.submit([commands.finish()]);
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            self.readback
                .map_async(wgpu::MapMode::Read, 0..size, move |result| {
                    let _ = sender.send(result);
                });
            self.compute.device.poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(60)),
            })?;
            receiver.recv_timeout(Duration::from_secs(1))??;
            let decoded = (|| -> Result<()> {
                let mapped = self.readback.get_mapped_range(0..size)?;
                for bytes in mapped.chunks_exact(16) {
                    let pixel =
                        std::array::from_fn(|channel| f32::from_bits(u32_at(bytes, channel * 4)));
                    if pixel.iter().any(|value| !value.is_finite()) {
                        return Err("GPU animation frame produced a non-finite channel".into());
                    }
                    pixels.push(pixel);
                }
                Ok(())
            })();
            self.readback.unmap();
            decoded?;
        }
        Ok(pixels)
    }
}
