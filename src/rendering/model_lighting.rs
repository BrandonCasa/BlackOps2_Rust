//! Native model-lighting atlas residency, owned by the render world.
//!
//! 00457f40/00458150 allocate the 512x256x4 atlas. Static handles occupy its
//! first 4096 blocks; the remaining 4096 belong to dynamic model lighting.
//! 004577f0 assigns/touches static handles, and 00457b70 permits reuse only
//! after more than three untouched frames. 00457c30 builds newly assigned
//! blocks. Region uploads use Bevy's RenderQueue instead of D3D staging copies.
use super::lighting::{LightingCoefficients, grid_coordinates};
use bevy::{
    platform::collections::HashMap,
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::{
            AddressMode, Extent3d, FilterMode, MipmapFilterMode, Origin3d, Sampler,
            SamplerDescriptor, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect,
            TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
            TextureViewDescriptor, TextureViewDimension,
        },
        renderer::{RenderDevice, RenderQueue},
        sync_world::MainEntity,
    },
};
use std::{fmt, sync::Arc};

const STATIC_SLOTS: usize = 4096;

#[derive(Resource, Clone, ExtractResource)]
#[extract_app(RenderApp)]
pub(crate) struct ModelLightingSource {
    pub coefficients: Arc<LightingCoefficients>,
    pub grid: Arc<super::light_grid::LightGrid>,
}

pub(crate) struct ModelLightingPlugin;

impl Plugin for ModelLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractResourcePlugin::<ModelLightingSource>::default());
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(Render, prepare_atlas.in_set(RenderSystems::PrepareViews));
        }
    }
}

fn prepare_atlas(
    mut commands: Commands,
    source: Option<Res<ModelLightingSource>>,
    atlas: Option<Res<ModelLightingAtlas>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    let Some(source) = source else {
        if atlas.is_some() {
            commands.remove_resource::<ModelLightingAtlas>();
        }
        return;
    };
    if source.is_changed() || atlas.is_none() {
        commands.insert_resource(ModelLightingAtlas::new(
            &device,
            &queue,
            source.coefficients.clone(),
            source.grid.clone(),
        ));
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ModelLighting {
    pub grid_coords_and_vis: Vec4,
    pub sh: [Vec4; 3],
}

/// Native 004577f0 rejects allocation when every static slot is still in use.
/// Its caller, 0045ac40, rejects the draw rather than using invented lighting.
#[derive(Debug, Clone, Copy)]
pub(crate) struct StaticLightingAtlasFull;

impl fmt::Display for StaticLightingAtlasFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("all 4096 original static model-lighting slots are still in use")
    }
}

impl std::error::Error for StaticLightingAtlasFull {}

struct StaticSlot {
    owner: MainEntity,
    colors_index: usize,
    last_used: u32,
    sh: [Vec4; 3],
}

#[derive(Resource)]
pub(crate) struct ModelLightingAtlas {
    texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    queue: RenderQueue,
    coefficients: Arc<LightingCoefficients>,
    grid: Arc<super::light_grid::LightGrid>,
    dynamic_usage: DynamicSlots,
    dynamic_handles: HashMap<MainEntity, u16>,
    dynamic_slots: Vec<Option<DynamicSlot>>,
    /// Match the original one-based handle; zero remains unassigned.
    handles: HashMap<MainEntity, u16>,
    slots: Vec<StaticSlot>,
    frame: Option<u32>,
    reusable: Vec<u16>,
    reusable_built: bool,
}

impl ModelLightingAtlas {
    pub(crate) fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        coefficients: Arc<LightingCoefficients>,
        grid: Arc<super::light_grid::LightGrid>,
    ) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("Origins original model-lighting atlas"),
            size: Extent3d {
                width: 512,
                height: 256,
                depth_or_array_layers: 4,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&TextureViewDescriptor {
            label: Some("Origins model-lighting 3D view"),
            dimension: Some(TextureViewDimension::D3),
            ..Default::default()
        });
        // Code sampler 0xE2: clamp on all axes, linear min/mag, no comparison,
        // zero mip bias. 0043d230 selects E2; 00432ba0 creates that sampler.
        // The source atlas has exactly one mip, so mip filtering cannot blend.
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("Origins model-lighting E2 sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: f32::MAX,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            queue: queue.clone(),
            coefficients,
            grid,
            dynamic_usage: DynamicSlots::default(),
            dynamic_handles: HashMap::with_capacity(4096),
            dynamic_slots: (0..4096).map(|_| None).collect(),
            handles: HashMap::with_capacity(STATIC_SLOTS),
            slots: Vec::with_capacity(STATIC_SLOTS),
            frame: None,
            reusable: Vec::with_capacity(STATIC_SLOTS),
            reusable_built: false,
        }
    }

    /// Call for visible static placements using the render frame counter.
    /// A resident placement only touches its last-used frame; its coefficients
    /// and GPU texels remain unchanged. Visibility belongs to the draw constants.
    pub(crate) fn touch(
        &mut self,
        owner: MainEntity,
        colors_index: usize,
        visibility: u8,
        frame: u32,
    ) -> Result<ModelLighting, StaticLightingAtlasFull> {
        if self.frame != Some(frame) {
            self.frame = Some(frame);
            self.reusable.clear();
            self.reusable_built = false;
        }

        if let Some(&handle) = self.handles.get(&owner) {
            let slot = handle - 1;
            let resident = &mut self.slots[usize::from(slot)];
            assert_eq!(
                resident.colors_index, colors_index,
                "A static model's authored colorsIndex changed while resident"
            );
            resident.last_used = frame;
            return Ok(ModelLighting {
                grid_coords_and_vis: grid_coordinates(slot, visibility),
                sh: resident.sh,
            });
        }

        let slot = self.allocate_slot(frame)?;
        let lighting = self.coefficients.static_lighting(colors_index);
        self.upload(slot, &lighting.texels);

        let resident = StaticSlot {
            owner,
            colors_index,
            last_used: frame,
            sh: lighting.sh,
        };
        let index = usize::from(slot);
        if index == self.slots.len() {
            self.slots.push(resident);
        } else {
            self.handles.remove(&self.slots[index].owner);
            self.slots[index] = resident;
        }
        self.handles.insert(owner, slot + 1);
        Ok(ModelLighting {
            grid_coords_and_vis: grid_coordinates(slot, visibility),
            sh: lighting.sh,
        })
    }

    /// Base light-grid contribution only; the native hero-light stage must be
    /// applied before claiming complete dynamic lighting parity.
    pub(crate) fn touch_dynamic_grid(
        &mut self,
        owner: MainEntity,
        origin: Vec3,
        frame: u32,
    ) -> Option<(ModelLighting, u8)> {
        self.dynamic_usage.begin_frame(frame);
        if let Some(&slot) = self.dynamic_handles.get(&owner) {
            let resident = self.dynamic_slots[usize::from(slot)]
                .as_ref()
                .expect("resident dynamic light slot");
            if resident.origin == origin {
                self.dynamic_usage.touch(slot);
                return Some((resident.lighting, resident.primary_light));
            }
        }
        let slot = self.dynamic_usage.allocate()?;
        let sample = self.grid.sample(origin);
        let lighting = self
            .coefficients
            .grid_lighting(&sample.coefficients[..sample.count]);
        let atlas_slot = slot + STATIC_SLOTS as u16;
        self.upload(atlas_slot, &lighting.texels);
        let mut coords = grid_coordinates(atlas_slot, 0);
        coords.w = sample.visibility;
        let lighting = ModelLighting {
            grid_coords_and_vis: coords,
            sh: lighting.sh,
        };
        let resident = &mut self.dynamic_slots[usize::from(slot)];
        if let Some(old) = resident {
            if self.dynamic_handles.get(&old.owner) == Some(&slot) {
                self.dynamic_handles.remove(&old.owner);
            }
        }
        *resident = Some(DynamicSlot {
            owner,
            origin,
            lighting,
            primary_light: sample.primary_light,
        });
        self.dynamic_handles.insert(owner, slot);
        Some((lighting, sample.primary_light))
    }

    fn upload(&self, slot: u16, texels: &[u8; 256]) {
        self.queue.write_texture(
            TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: Origin3d {
                    x: u32::from(slot & 127) * 4,
                    y: u32::from(slot >> 7) * 4,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            texels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(16),
                rows_per_image: Some(4),
            },
            Extent3d {
                width: 4,
                height: 4,
                depth_or_array_layers: 4,
            },
        );
    }

    fn allocate_slot(&mut self, frame: u32) -> Result<u16, StaticLightingAtlasFull> {
        if self.slots.len() < STATIC_SLOTS {
            return Ok(self.slots.len() as u16);
        }
        // Native 00457b70 collects candidates once per frame, then 004577f0
        // consumes them from the end. Defer this bounded scan until full and
        // needed; resident draws do no scans or allocations.
        if !self.reusable_built {
            self.reusable.extend(
                self.slots
                    .iter()
                    .enumerate()
                    .filter(|(_, slot)| frame.wrapping_sub(slot.last_used) > 3)
                    .map(|(index, _)| index as u16),
            );
            self.reusable_built = true;
        }
        while let Some(slot) = self.reusable.pop() {
            // A candidate can have been touched by another visible draw since
            // collection. Preserve that placement's existing handle.
            if frame.wrapping_sub(self.slots[usize::from(slot)].last_used) > 3 {
                return Ok(slot);
            }
        }
        Err(StaticLightingAtlasFull)
    }
}
// 004578d0 intersects current/previous-two usage masks. A moving instance gets
// a fresh block; it must not overwrite a block referenced by an in-flight frame.
struct DynamicSlots {
    unused: [[u32; 128]; 3],
    frame: Option<u32>,
    cursor: usize,
    current: usize,
    exhausted: bool,
}

impl Default for DynamicSlots {
    fn default() -> Self {
        Self {
            unused: [[u32::MAX; 128]; 3],
            frame: None,
            cursor: 0,
            current: 0,
            exhausted: false,
        }
    }
}

impl DynamicSlots {
    fn begin_frame(&mut self, frame: u32) {
        if self.frame == Some(frame) {
            return;
        }
        let elapsed = self
            .frame
            .map_or(3, |previous| frame.wrapping_sub(previous).min(3));
        for _ in 0..elapsed {
            self.current = (self.current + 1) % 3;
            self.unused[self.current].fill(u32::MAX);
        }
        self.frame = Some(frame);
        self.exhausted = false;
    }

    fn touch(&mut self, slot: u16) {
        let word = usize::from(slot) / 32;
        self.unused[self.current][word] &= !(0x8000_0000 >> (slot % 32));
    }

    fn allocate(&mut self) -> Option<u16> {
        if self.exhausted {
            return None;
        }
        for offset in 0..128 {
            let word = (self.cursor + offset) % 128;
            let available = self.unused[0][word] & self.unused[1][word] & self.unused[2][word];
            if available != 0 {
                let slot = (word * 32 + available.leading_zeros() as usize) as u16;
                self.cursor = word;
                self.touch(slot);
                return Some(slot);
            }
        }
        self.exhausted = true;
        None
    }
}

struct DynamicSlot {
    owner: MainEntity,
    origin: Vec3,
    lighting: ModelLighting,
    primary_light: u8,
}

#[cfg(test)]
mod dynamic_slot_tests {
    use super::*;

    #[test]
    fn preserves_current_and_previous_two_frames() {
        let mut slots = DynamicSlots::default();
        slots.begin_frame(0);
        for expected in 0..4096 {
            assert_eq!(slots.allocate(), Some(expected));
        }
        assert_eq!(slots.allocate(), None);
        for frame in [1, 2] {
            slots.begin_frame(frame);
            assert_eq!(slots.allocate(), None);
        }
        slots.begin_frame(3);
        assert!(slots.allocate().is_some());
    }

    #[test]
    fn a_touched_block_remains_unavailable_across_frame_wrap() {
        let mut slots = DynamicSlots::default();
        slots.begin_frame(u32::MAX - 1);
        let retained = slots.allocate().unwrap();
        slots.begin_frame(u32::MAX);
        slots.touch(retained);
        slots.begin_frame(0);
        slots.touch(retained);
        slots.begin_frame(1);
        for _ in 0..4095 {
            assert_ne!(slots.allocate().unwrap(), retained);
        }
        assert_eq!(slots.allocate(), None);
    }
}
