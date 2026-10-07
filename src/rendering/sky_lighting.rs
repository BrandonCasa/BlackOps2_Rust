//! Reuse Bevy's full-resolution filtered sky lighting between fixed-rate updates.
//! The visible sky, aerial perspective and moving-caster shadows remain current-frame.
use bevy::{
    light::Atmosphere,
    pbr::generate::{
        GeneratorPipelines, RenderEnvironmentMap,
        prepare_generated_environment_map_intermediate_textures,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        render_resource::PipelineCache,
    },
};
use std::time::{Duration, Instant};

#[derive(Component, Clone, ExtractComponent)]
pub(super) struct SkyEnvironment(pub Handle<Image>);

fn environment(
    mut commands: Commands,
    cameras: Query<
        (
            Entity,
            &bevy::light::EnvironmentMapLight,
            Option<&SkyEnvironment>,
        ),
        With<super::GameCamera>,
    >,
) {
    for (entity, environment, current) in &cameras {
        if current.is_none_or(|current| current.0 != environment.specular_map) {
            commands
                .entity(entity)
                .insert(SkyEnvironment(environment.specular_map.clone()));
        }
    }
}

#[derive(Resource, Clone, ExtractResource)]
struct SkyInputs {
    revision: u64,
    interval: Duration,
}

#[derive(Resource, Default)]
struct FilterCadence {
    last: Option<Instant>,
    revision: u64,
    followup: bool,
    suspended: Vec<(Entity, RenderEnvironmentMap)>,
}

pub(super) struct SkyLightingPlugin;
impl Plugin for SkyLightingPlugin {
    fn build(&self, app: &mut App) {
        let hz = app
            .world()
            .resource::<super::profile::VisualProfile>()
            .sky_lighting_hz;
        app.insert_resource(SkyInputs {
            revision: 0,
            interval: Duration::from_secs_f32(1. / hz.max(0.01)),
        })
        .add_plugins(ExtractResourcePlugin::<SkyInputs>::default())
        .add_plugins(ExtractComponentPlugin::<SkyEnvironment>::default())
        .add_systems(Update, environment)
        .add_systems(
            PostUpdate,
            inputs.after(bevy::transform::TransformSystems::Propagate),
        );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.init_resource::<FilterCadence>().add_systems(
                Render,
                suspend
                    // Skip native preparation as well as filtering: missing
                    // mip placeholders otherwise allocate and clear each frame.
                    .in_set(RenderSystems::PrepareResources)
                    .before(prepare_generated_environment_map_intermediate_textures),
            );
        }
    }
}

fn inputs(
    lights: Query<(Ref<DirectionalLight>, Ref<GlobalTransform>)>,
    atmospheres: Query<Ref<Atmosphere>>,
    camera: Query<&GlobalTransform, With<super::GameCamera>>,
    mut height: Local<Option<f32>>,
    mut medium_events: MessageReader<AssetEvent<bevy::light::atmosphere::ScatteringMedium>>,
    mut inputs: ResMut<SkyInputs>,
) {
    let y = camera.single().ok().map(|pose| pose.translation().y);
    let medium_changed = medium_events.read().count() != 0;
    let dirty = lights
        .iter()
        .any(|(light, pose)| light.is_changed() || pose.is_changed())
        || atmospheres.iter().any(|atmosphere| atmosphere.is_changed())
        || *height != y
        || medium_changed;
    *height = y;
    if dirty {
        inputs.revision = inputs.revision.wrapping_add(1);
    }
}

fn suspend(world: &mut World) {
    let Some(pipelines) = world.get_resource::<GeneratorPipelines>() else {
        return;
    };
    let cache = world.resource::<PipelineCache>();
    if [
        pipelines.copy,
        pipelines.downsample_first,
        pipelines.downsample_second,
        pipelines.radiance,
        pipelines.irradiance,
    ]
    .into_iter()
    .any(|id| cache.get_compute_pipeline(id).is_none())
    {
        return;
    }
    let inputs = world.resource::<SkyInputs>();
    let (revision, interval) = (inputs.revision, inputs.interval);
    let mut cadence = world.resource_mut::<FilterCadence>();
    let changed = cadence.revision != revision;
    let due =
        changed || cadence.followup || cadence.last.is_none_or(|last| last.elapsed() >= interval);
    if due {
        cadence.last = Some(Instant::now());
        cadence.revision = revision;
        // Atmosphere generation runs after filtering. Refresh again next frame
        // so a changed sun/medium is never held at the previous sky for a second.
        cadence.followup = changed;
        return;
    }
    let entities: Vec<_> = world
        .query_filtered::<Entity, (
            With<RenderEnvironmentMap>,
            Without<super::static_gi::LocalReflection>,
        )>()
        .iter(world)
        .collect();
    let mut suspended = Vec::with_capacity(entities.len());
    for entity in entities {
        if let Some(map) = world.entity_mut(entity).take::<RenderEnvironmentMap>() {
            suspended.push((entity, map));
        }
    }
    world.resource_mut::<FilterCadence>().suspended = suspended;
}

pub(super) fn restore(world: &mut World) {
    let suspended = std::mem::take(&mut world.resource_mut::<FilterCadence>().suspended);
    for (entity, map) in suspended {
        if let Ok(mut entity) = world.get_entity_mut(entity) {
            entity.insert(map);
        }
    }
}
