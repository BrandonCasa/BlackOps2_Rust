use super::Recipe;
use crate::core::*;
use std::{future::Future, sync::{Arc, OnceLock}, task::{Context, Poll, Wake, Waker}, time::Duration};
use wgpu::util::DeviceExt;

pub(super) type Sample = [f32; 20];
pub(super) type Pixel = [[f32; 4]; 4];

fn wait<T>(future: impl Future<Output = T>) -> T {
    struct Thread(std::thread::Thread);
    impl Wake for Thread { fn wake(self: Arc<Self>) { self.0.unpark(); } }
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
}

impl Compute {
    fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN, ..wgpu::InstanceDescriptor::new_without_display_handle() });
        let adapter = wait(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default()
        }))?;
        if matches!(adapter.get_info().device_type, wgpu::DeviceType::Cpu) {
            return Err("Material baking requires a hardware GPU".into());
        }
        let (device, queue) = wait(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Origins material baker"), required_limits: adapter.limits(), ..Default::default()
        }))?;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Existing layered material equations"),
            source: wgpu::ShaderSource::Wgsl(include_str!("bake.wgsl").into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Layered material bake"), layout: None, module: &shader,
            entry_point: Some("bake"), compilation_options: Default::default(), cache: None,
        });
        if let Some(error) = wait(errors.pop()) { return Err(error.into()); }
        println!("GPU material baking: {} ({:?}); {} CPU workers", adapter.get_info().name,
            adapter.get_info().backend, std::thread::available_parallelism().map_or(1, usize::from));
        Ok(Self { device, queue, pipeline })
    }
}

pub(super) struct Baker {
    compute: &'static Compute,
    parameters: wgpu::Buffer,
    samples: wgpu::Buffer,
    output: wgpu::Buffer,
    readback: wgpu::Buffer,
    bindings: wgpu::BindGroup,
    capacity: usize,
}

impl Baker {
    pub(super) fn new(recipe: &Recipe) -> Result<Self> {
        if recipe.inputs.is_empty() || recipe.inputs.len() > 9 { return Err("Layered GPU recipe must have one to nine source images".into()); }
        static COMPUTE: OnceLock<std::result::Result<Compute, String>> = OnceLock::new();
        let compute = COMPUTE.get_or_init(|| Compute::new().map_err(|error| error.to_string()))
            .as_ref().map_err(|error| format!("GPU material baking initialization failed: {error}"))?;
        let flags = recipe.decal as u32 | (recipe.multiply as u32) << 1 | (recipe.additive as u32) << 2
            | (recipe.blend_color as u32) << 3 | (recipe.reveal as u32) << 4
            | (recipe.layer_specular as u32) << 5 | (recipe.base_alpha_response as u32) << 6
            | (recipe.base_specular as u32) << 7 | (recipe.geometric_normal as u32) << 8
            | (recipe.second_additive as u32) << 9 | (u32::from(recipe.second_normal == 1)) << 10 | (u32::from(recipe.second_normal == 2)) << 11
            | (recipe.second_reveal as u32) << 12 | (recipe.second_specular as u32) << 13
            | (recipe.third_multiply as u32) << 14 | (recipe.layer_cutout as u32) << 15;
        let mut parameters = Vec::new();
        for value in [flags, 0, recipe.inputs.len() as u32, recipe.inputs.len() as u32-1] { push32(&mut parameters, value); }
        for value in recipe.parms.into_iter().chain(recipe.second_parms) { push32(&mut parameters, value.to_bits()); }
        for index in [recipe.layer_specular_index(),recipe.second_specular_index(),recipe.second_normal_index(),recipe.second_color_index()] { push32(&mut parameters,index as u32); }
        let mut image_data = Vec::new();
        for index in 0..9 {
            let metadata = if let Some(input) = recipe.inputs.get(index) {
                let offset = u32::try_from(image_data.len() / 16)?;
                for pixel in &input.pixels { for value in pixel { push32(&mut image_data, value.to_bits()); } }
                [input.width as u32, input.height as u32, offset, input.sampler as u32]
            } else { [1, 1, 0, 20] };
            for value in metadata { push32(&mut parameters, value); }
        }
        let device = &compute.device;
        if image_data.len() as u64 > device.limits().max_storage_buffer_binding_size as u64 {
            return Err("Layered source images exceed GPU storage buffer limit".into());
        }
        let images = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Decoded native images"), contents: &image_data, usage: wgpu::BufferUsages::STORAGE,
        });
        let parameters = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Native material recipe"), contents: &parameters, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let capacity = 262_144_usize.min(device.limits().max_storage_buffer_binding_size as usize / 80)
            .min(device.limits().max_compute_workgroups_per_dimension as usize * 64);
        if capacity == 0 { return Err("GPU cannot dispatch material bake samples".into()); }
        let buffer = |label, size, usage| device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size, usage, mapped_at_creation: false });
        let samples = buffer("Bake sample coordinates", (capacity * 80) as u64, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let output = buffer("Baked PBR samples", (capacity * 64) as u64, wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let readback = buffer("Bake readback", (capacity * 64) as u64, wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST);
        let entries: Vec<_> = [&parameters, &images, &samples, &output].into_iter().enumerate()
            .map(|(binding, buffer)| wgpu::BindGroupEntry { binding: binding as u32, resource: buffer.as_entire_binding() }).collect();
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material bake inputs"), layout: &compute.pipeline.get_bind_group_layout(0), entries: &entries,
        });
        Ok(Self { compute, parameters, samples, output, readback, bindings, capacity })
    }

    pub(super) fn sample(&self, samples: &[Sample]) -> Result<Vec<Pixel>> {
        let mut pixels = Vec::with_capacity(samples.len());
        for batch in samples.chunks(self.capacity) {
            let mut bytes = Vec::with_capacity(batch.len() * 80);
            for sample in batch { for value in sample { push32(&mut bytes, value.to_bits()); } }
            self.compute.queue.write_buffer(&self.samples, 0, &bytes);
            self.compute.queue.write_buffer(&self.parameters, 4, &(batch.len() as u32).to_le_bytes());
            let mut commands = self.compute.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Material bake dispatch") });
            {
                let mut pass = commands.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("Material bake"), timestamp_writes: None });
                pass.set_pipeline(&self.compute.pipeline);
                pass.set_bind_group(0, &self.bindings, &[]);
                pass.dispatch_workgroups((batch.len() as u32).div_ceil(64), 1, 1);
            }
            let size = (batch.len() * 64) as u64;
            commands.copy_buffer_to_buffer(&self.output, 0, &self.readback, 0, size);
            let submission = self.compute.queue.submit([commands.finish()]);
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            self.readback.map_async(wgpu::MapMode::Read, 0..size, move |result| { let _ = sender.send(result); });
            self.compute.device.poll(wgpu::PollType::Wait { submission_index: Some(submission), timeout: Some(Duration::from_secs(60)) })?;
            receiver.recv_timeout(Duration::from_secs(1))??;
            let decoded = (|| -> Result<()> {
                let mapped = self.readback.get_mapped_range(0..size)?;
                for bytes in mapped.chunks_exact(64) {
                    let pixel: Pixel = std::array::from_fn(|role| std::array::from_fn(|channel| f32::from_bits(u32_at(bytes, role * 16 + channel * 4))));
                    if pixel[0][0].to_bits() == 0x7fc00001 { return Err("Layered reveal thresholds have no interval".into()); }
                    if pixel.iter().flatten().any(|value| !value.is_finite()) { return Err("GPU material bake produced a non-finite channel".into()); }
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
