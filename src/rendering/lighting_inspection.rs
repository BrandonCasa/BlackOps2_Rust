//! One-shot actual-GPU radiance evidence for functional window verification.
//! Disabled in profiling runs so readback does not contaminate timing captures.
use bevy::{
    prelude::*,
    render::gpu_readback::{Readback, ReadbackComplete},
};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Component)]
struct Sample {
    label: String,
    width: usize,
    height: usize,
    field: bool,
    output: PathBuf,
}
#[derive(Resource, Default)]
struct Samples(BTreeMap<String, serde_json::Value>);

pub(super) struct LightingInspectionPlugin;
impl Plugin for LightingInspectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Samples>()
            .add_systems(Update, (enable_readback, inspect).chain());
    }
}
fn enable_readback(
    options: Res<crate::assets::LaunchOptions>,
    cameras: Query<&bevy::light::EnvironmentMapLight, With<super::GameCamera>>,
    mut images: ResMut<Assets<Image>>,
) {
    if options.verify.is_none() || options.benchmark {
        return;
    }
    for environment in &cameras {
        for handle in [&environment.diffuse_map, &environment.specular_map] {
            let Some(image) = images.get(handle) else {
                continue;
            };
            if !image
                .texture_descriptor
                .usage
                .contains(bevy::render::render_resource::TextureUsages::COPY_SRC)
                && let Some(mut image) = images.get_mut(handle)
            {
                image.texture_descriptor.usage |=
                    bevy::render::render_resource::TextureUsages::COPY_SRC;
            }
        }
    }
}
fn inspect(
    mut commands: Commands,
    options: Res<crate::assets::LaunchOptions>,
    state: Res<State<crate::GameState>>,
    cameras: Query<&bevy::light::EnvironmentMapLight, With<super::GameCamera>>,
    fields: Res<super::static_gi::GiInputs>,
    gi: Res<super::static_gi::StaticGiStats>,
    images: Res<Assets<Image>>,
    time: Res<Time<Real>>,
    mut started: Local<Option<f32>>,
    mut done: Local<bool>,
) {
    let Some(output) = &options.verify else {
        return;
    };
    if *done || options.benchmark || **state == crate::GameState::Loading {
        return;
    }
    let start = *started.get_or_insert(time.elapsed_secs());
    if time.elapsed_secs() - start < 10.0 || gi.updates() < 2 {
        return;
    }
    let Ok(environment) = cameras.single() else {
        return;
    };
    let mut candidates = vec![
        (
            "sky-specular".to_owned(),
            environment.specular_map.clone(),
            false,
        ),
        (
            "sky-diffuse".to_owned(),
            environment.diffuse_map.clone(),
            false,
        ),
    ];
    candidates.extend(
        fields
            .cascades
            .iter()
            .enumerate()
            .map(|(i, c)| (format!("ddgi-{i}"), c.image.clone(), true)),
    );
    for (label, image, field) in candidates {
        let Some(source) = images.get(&image) else {
            continue;
        };
        let width = source.texture_descriptor.size.width as usize;
        commands
            .spawn((
                Readback::texture(image),
                Sample {
                    label,
                    width,
                    height: source.texture_descriptor.size.height as usize,
                    field,
                    output: output.clone(),
                },
            ))
            .observe(sample);
    }
    *done = true;
}
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = (bits >> 10) & 31;
    let fraction = bits & 1023;
    if exponent == 31 {
        return f32::NAN;
    }
    if exponent == 0 {
        sign * f32::from(fraction) * 2.0_f32.powi(-24)
    } else {
        sign * (1.0 + f32::from(fraction) / 1024.0) * 2.0_f32.powi(i32::from(exponent) - 15)
    }
}
fn sample(
    event: On<ReadbackComplete>,
    tags: Query<&Sample>,
    mut samples: ResMut<Samples>,
    gi: Res<super::static_gi::StaticGiStats>,
    mut commands: Commands,
) {
    let Ok(tag) = tags.get(event.entity) else {
        return;
    };
    let mut values = Vec::new();
    let mut nonfinite = 0usize;
    for (index, texel) in event.data.chunks_exact(8).enumerate() {
        if tag.field {
            let tile = super::static_gi::PROBE_TILE.as_usizevec2();
            let x = index % tag.width;
            let y = (index / tag.width) % tag.height;
            let z = index / (tag.width * tag.height);
            if x % tile.x + y % tile.y * tile.x >= 64 {
                continue;
            }
            let state = z * tag.width * tag.height
                + (y / tile.y * tile.y + 192 / tile.x) * tag.width
                + x / tile.x * tile.x
                + 192 % tile.x;
            let alpha = state * 8 + 6;
            if half(u16::from_le_bytes([
                event.data[alpha],
                event.data[alpha + 1],
            ])) <= 0.
            {
                continue;
            }
        }
        let mut rgb: [f32; 3] =
            std::array::from_fn(|i| half(u16::from_le_bytes([texel[i * 2], texel[i * 2 + 1]])));
        if tag.field {
            rgb = rgb.map(|v| v.powi(5) * 2.);
        }
        if rgb.iter().all(|value| value.is_finite()) {
            values.push(rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722);
        } else {
            nonfinite += 1;
        }
    }
    values.sort_by(f32::total_cmp);
    let count = values.len();
    let data = serde_json::json!({"samples":count,"nonfinite":nonfinite,"mean_radiance":values.iter().sum::<f32>() / count.max(1) as f32,
        "median_radiance":values.get(count / 2),"p99_radiance":values.get(count.saturating_sub(1) * 99 / 100),"max_radiance":values.last(),
        "ddgi_probe_batches":gi.updates(),
        "quantity":if tag.field { "diffuse_irradiance_over_pi" } else { "environment_radiance" }});
    info!("GPU lighting sample {}: {}", tag.label, data);
    samples.0.insert(tag.label.clone(), data);
    if let Err(error) = std::fs::write(
        tag.output.join("lighting-samples.json"),
        serde_json::to_vec_pretty(&samples.0).expect("Lighting evidence serialization"),
    ) {
        warn!("Cannot save GPU lighting evidence: {error}");
    }
    commands.entity(event.entity).despawn();
}
