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
pub(crate) struct ModelLightingSource(pub Arc<LightingCoefficients>);

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
        commands.insert_resource(ModelLightingAtlas::new(&device, &queue, source.0.clone()));
    }
}

#[derive(Clone, Copy)]
pub(crate) struct StaticModelLighting {
    /// Zero-based block index. The original nonzero lighting handle is slot+1.
    pub slot: u16,
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
    ) -> Result<StaticModelLighting, StaticLightingAtlasFull> {
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
            return Ok(StaticModelLighting {
                slot,
                grid_coords_and_vis: grid_coordinates(slot, visibility),
                sh: resident.sh,
            });
        }

        let slot = self.allocate_slot(frame)?;
        let lighting = self.coefficients.static_lighting(colors_index);
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
            &lighting.texels,
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
        Ok(StaticModelLighting {
            slot,
            grid_coords_and_vis: grid_coordinates(slot, visibility),
            sh: lighting.sh,
        })
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
