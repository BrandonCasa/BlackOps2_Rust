//! Native late HDR rendering with depth owned by the first-person view.
use crate::{assets::LaunchOptions, presentation::ViewModelCamera};
use bevy::{
    core_pipeline::{
        core_3d::{CORE_3D_DEPTH_FORMAT, prepare_core_3d_depth_textures},
        prepass::DepthPrepass,
        schedule::camera_driver,
    },
    ecs::entity::EntityHashMap,
    image::ToExtents,
    prelude::*,
    render::{
        GpuResourceAppExt, Render, RenderApp, RenderSystems,
        camera::ExtractedCamera,
        extract_component::ExtractComponentPlugin,
        render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureUsages},
        renderer::{RenderDevice, RenderGraph},
        texture::CachedTexture,
        view::{ViewDepthStencilTexture, ViewTarget},
    },
};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Resource, Clone, Default)]
pub(crate) struct FirstPersonDepthStats(Option<Arc<DepthCounters>>);

#[derive(Default)]
struct DepthCounters {
    allocations: AtomicU64,
    reuses: AtomicU64,
    checked_views: AtomicU64,
    depth_aliases: AtomicU64,
    hdr_mismatches: AtomicU64,
    retained_bytes: AtomicU64,
}

impl FirstPersonDepthStats {
    pub(crate) fn report(&self) -> serde_json::Value {
        let Some(counters) = &self.0 else {
            return serde_json::Value::Null;
        };
        let load = |value: &AtomicU64| value.load(Ordering::Relaxed);
        serde_json::json!({
            "allocations": load(&counters.allocations),
            "reuses": load(&counters.reuses),
            "checked_views": load(&counters.checked_views),
            "depth_aliases": load(&counters.depth_aliases),
            "hdr_mismatches": load(&counters.hdr_mismatches),
            "retained_bytes": load(&counters.retained_bytes),
            "depth_format": "Depth32Float",
            "clear_depth": 0.0,
        })
    }
}

#[derive(Resource, Default)]
struct FirstPersonDepths(EntityHashMap<OwnedDepth>);

struct OwnedDepth {
    size: Extent3d,
    samples: u32,
    usage: TextureUsages,
    texture: CachedTexture,
}

pub(super) struct FirstPersonRenderPlugin;

impl Plugin for FirstPersonRenderPlugin {
    fn build(&self, app: &mut App) {
        let stats = FirstPersonDepthStats(
            app.world()
                .resource::<LaunchOptions>()
                .verify
                .as_ref()
                .filter(|_| cfg!(debug_assertions))
                .map(|_| Arc::default()),
        );
        app.insert_resource(stats.clone())
            .add_plugins(ExtractComponentPlugin::<ViewModelCamera>::default());
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .insert_resource(stats)
                // RenderStartup resets handles after device recovery.
                .init_gpu_resource::<FirstPersonDepths>()
                .add_systems(
                    Render,
                    prepare_depth
                        .after(prepare_core_3d_depth_textures)
                        .in_set(RenderSystems::PrepareResources),
                )
                .add_systems(RenderGraph, record_depth_contract.before(camera_driver));
        }
    }
}

fn prepare_depth(
    mut commands: Commands,
    device: Res<RenderDevice>,
    mut depths: ResMut<FirstPersonDepths>,
    stats: Res<FirstPersonDepthStats>,
    views: Query<
        (
            Entity,
            &ExtractedCamera,
            &Camera3d,
            &Msaa,
            Has<DepthPrepass>,
        ),
        With<ViewModelCamera>,
    >,
) {
    // Removing a camera or suspending its target retires its storage. Ordinary
    // camera motion, ADS and animation preserve allocation and depth identity.
    depths.0.retain(|entity, _| {
        views
            .get(*entity)
            .is_ok_and(|(_, camera, ..)| camera.physical_target_size.is_some())
    });
    for (entity, camera, camera_3d, msaa, prepass) in &views {
        let Some(size) = camera.physical_target_size.map(|size| size.to_extents()) else {
            continue;
        };
        let samples = msaa.samples();
        let mut usage: TextureUsages = camera_3d.depth_texture_usages.into();
        if prepass {
            usage |= TextureUsages::COPY_SRC;
        }
        let compatible = depths.0.get(&entity).is_some_and(|depth| {
            depth.size == size && depth.samples == samples && depth.usage == usage
        });
        if !compatible {
            let texture = device.create_texture(&TextureDescriptor {
                label: Some("first_person_owned_depth"),
                size,
                mip_level_count: 1,
                sample_count: samples,
                dimension: TextureDimension::D2,
                format: CORE_3D_DEPTH_FORMAT,
                usage,
                view_formats: &[],
            });
            let default_view = texture.create_view(&default());
            depths.0.insert(
                entity,
                OwnedDepth {
                    size,
                    samples,
                    usage,
                    texture: CachedTexture {
                        texture,
                        default_view,
                    },
                },
            );
        }
        if let Some(counters) = &stats.0 {
            if compatible {
                counters.reuses.fetch_add(1, Ordering::Relaxed);
            } else {
                counters.allocations.fetch_add(1, Ordering::Relaxed);
            }
        }
        commands.entity(entity).insert(ViewDepthStencilTexture::new(
            depths.0[&entity].texture.clone(),
            // Bevy's reverse-Z clear preserves normal self-occlusion. This
            // attachment never participates in the world camera's depth.
            Some(0.0),
            None,
        ));
    }
    if let Some(counters) = &stats.0 {
        counters.retained_bytes.store(
            depths
                .0
                .values()
                .map(|depth| {
                    u64::from(depth.size.width)
                        * u64::from(depth.size.height)
                        * u64::from(depth.size.depth_or_array_layers)
                        * u64::from(depth.samples)
                        * 4
                })
                .sum(),
            Ordering::Relaxed,
        );
    }
}

fn record_depth_contract(
    stats: Res<FirstPersonDepthStats>,
    first_person: Query<
        (&ExtractedCamera, &ViewDepthStencilTexture, &ViewTarget),
        With<ViewModelCamera>,
    >,
    world: Query<
        (&ExtractedCamera, &ViewDepthStencilTexture, &ViewTarget),
        (With<Camera3d>, Without<ViewModelCamera>),
    >,
) {
    let Some(counters) = &stats.0 else {
        return;
    };
    for (camera, depth, hdr) in &first_person {
        for (world_camera, world_depth, world_hdr) in &world {
            if camera.target != world_camera.target {
                continue;
            }
            counters.checked_views.fetch_add(1, Ordering::Relaxed);
            if depth.texture().id() == world_depth.texture().id() {
                counters.depth_aliases.fetch_add(1, Ordering::Relaxed);
            }
            if hdr.main_texture().id() != world_hdr.main_texture().id() {
                counters.hdr_mismatches.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
