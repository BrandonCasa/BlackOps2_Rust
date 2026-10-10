use super::{Settings, Tour, TourStatus, telemetry};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

const WIDTH: usize = 360;
const HEIGHT: usize = 90;
const BOUNDS: [f64; 6] = [8.33, 16.67, 25., 33.33, 50., 100.];

#[derive(Component)]
struct Panel;
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct ResourceText;
#[derive(Component)]
struct SummaryText;
#[derive(Component)]
struct GraphLabel;
#[derive(Resource)]
struct Charts {
    graph: Handle<Image>,
    histogram: Handle<Image>,
    refresh: f64,
}

pub(super) struct TourOverlayPlugin;
impl Plugin for TourOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            setup.after(crate::presentation::first_person_light),
        )
        .add_systems(Update, update);
    }
}

fn texture(images: &mut Assets<Image>) -> Handle<Image> {
    images.add(Image::new_fill(
        Extent3d {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[12, 18, 24, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    ))
}

fn text(value: &str) -> impl Bundle {
    (
        Text::new(value),
        TextFont {
            font_size: FontSize::Px(13.),
            ..default()
        },
        TextColor(Color::srgb(0.83, 0.88, 0.92)),
    )
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    cameras: Query<Entity, With<crate::presentation::ViewModelCamera>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    let graph = texture(&mut images);
    let histogram = texture(&mut images);
    commands
        .spawn((
            Panel,
            UiTargetCamera(camera),
            GlobalZIndex(29),
            BackgroundColor(Color::srgba(0.015, 0.022, 0.03, 0.88)),
            Node {
                position_type: PositionType::Absolute,
                left: px(16.),
                top: px(96.),
                width: px(384.),
                padding: UiRect::all(px(12.)),
                row_gap: px(7.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn(text("ORIGINS / VISUAL FLYTHROUGH"));
            parent.spawn((StatusText, text("Loading Origins...")));
            parent.spawn((SummaryText, text("Tour average -- FPS")));
            parent.spawn((GraphLabel, text("FRAME TIME / last 8 seconds / 0-64 ms")));
            parent.spawn((
                ImageNode::new(graph.clone()),
                Node {
                    width: px(WIDTH as f32),
                    height: px(HEIGHT as f32),
                    ..default()
                },
            ));
            parent.spawn(text("FRAME TIME DISTRIBUTION / whole tour / 0-100%"));
            parent.spawn((
                ImageNode::new(histogram.clone()),
                Node {
                    width: px(WIDTH as f32),
                    height: px(HEIGHT as f32),
                    ..default()
                },
            ));
            parent.spawn((
                Text::new("<8.3  <16.7  <25  <33.3  <50  <100  >100 ms"),
                TextFont {
                    font_size: FontSize::Px(11.),
                    ..default()
                },
                TextColor(Color::srgb(0.7, 0.78, 0.83)),
            ));
            parent.spawn((
                ResourceText,
                text("PROCESS AVERAGES\nCPU --    GPU --\nRAM --    VRAM --"),
            ));
            parent.spawn(text("Space pause / R restart / F12 FPS / Esc exit"));
        });
    commands.insert_resource(Charts {
        graph,
        histogram,
        refresh: 0.,
    });
}

fn update(
    time: Res<Time<Real>>,
    settings: Res<Settings>,
    tour: Res<Tour>,
    status: Res<TourStatus>,
    mut charts: ResMut<Charts>,
    mut images: ResMut<Assets<Image>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<StatusText>>,
        Query<&mut Text, With<ResourceText>>,
        Query<&mut Text, With<SummaryText>>,
        Query<&mut Text, With<GraphLabel>>,
    )>,
) {
    charts.refresh += time.delta_secs_f64();
    if charts.refresh < 0.25 {
        return;
    }
    charts.refresh %= 0.25;
    let elapsed: f64 = tour.frames.iter().sum();
    if let Ok(mut text) = texts.p0().single_mut() {
        text.0 = if !status.report_error.is_empty() {
            status.report_error.clone()
        } else if status.phase == "finished" {
            format!(
                "Tour complete / {:.0}s\nSaved: {}\nR to replay",
                status.elapsed_seconds,
                settings.output.display()
            )
        } else {
            format!(
                "{}{}\n{:.0} / {:.0}s | shot {} / {} | {:.2}x",
                if status.paused { "PAUSED / " } else { "" },
                status.label,
                status.elapsed_seconds,
                status.duration_seconds,
                (status.shot + 1).min(status.shots),
                status.shots,
                settings.speed
            )
        };
    }
    if let Ok(mut text) = texts.p2().single_mut() {
        text.0 = if elapsed > 0. {
            format!(
                "Tour average {:.1} FPS / {:.2} ms / {} frames",
                tour.frames.len() as f64 * 1000. / elapsed,
                elapsed / tour.frames.len() as f64,
                tour.frames.len()
            )
        } else {
            "Tour average -- FPS / warmup excluded".into()
        };
    }
    let format = |value: Option<f64>, unit: &str| {
        value.map_or_else(|| "unavailable".into(), |v| format!("{v:.1}{unit}"))
    };
    if let Ok(mut text) = texts.p1().single_mut() {
        text.0 = format!(
            "PROCESS AVERAGES\nCPU {} / GPU {} (busiest engine)\nResident RAM {} / VRAM {}\nSystem CPU {}",
            format(
                telemetry::average(&tour.telemetry, |s| Some(s.cpu_percent)),
                "%"
            ),
            format(telemetry::average(&tour.telemetry, |s| s.gpu_percent), "%"),
            format(
                telemetry::average(&tour.telemetry, |s| Some(s.working_set_mib / 1024.)),
                " GiB"
            ),
            format(
                telemetry::average(&tour.telemetry, |s| s.dedicated_gpu_mib.map(|m| m / 1024.)),
                " GiB"
            ),
            format(
                telemetry::average(&tour.telemetry, |s| s.system_cpu_percent),
                "%"
            )
        );
    }
    let mut recent = Vec::new();
    let mut seconds = 0.;
    for &ms in tour.frames.iter().rev() {
        recent.push(ms);
        seconds += ms / 1000.;
        if seconds >= 8. {
            break;
        }
    }
    recent.reverse();
    let scale = 2f64.powf(recent.iter().copied().fold(64., f64::max).log2().ceil());
    if let Ok(mut label) = texts.p3().single_mut() {
        label.0 = format!("FRAME TIME / last 8 seconds / 0-{scale:.0} ms");
    }
    if let Some(mut image) = images.get_mut(&charts.graph)
        && let Some(data) = image.data.as_mut()
    {
        background(data);
        for reference in [16.67, 33.33] {
            let y = (HEIGHT - 1) as i32 - (reference / scale * (HEIGHT - 1) as f64) as i32;
            line(data, (0, y), ((WIDTH - 1) as i32, y), [65, 75, 85, 255]);
        }
        let mut x_seconds = 8. - seconds.min(8.);
        let mut previous = None;
        for ms in recent {
            x_seconds += ms / 1000.;
            let point = (
                ((x_seconds / 8.) * (WIDTH - 1) as f64).clamp(0., (WIDTH - 1) as f64) as i32,
                (HEIGHT - 1) as i32
                    - (ms / scale * (HEIGHT - 1) as f64).clamp(0., (HEIGHT - 1) as f64) as i32,
            );
            if let Some(last) = previous {
                line(data, last, point, [77, 218, 248, 255]);
            }
            previous = Some(point);
        }
    }
    if let Some(mut image) = images.get_mut(&charts.histogram)
        && let Some(data) = image.data.as_mut()
    {
        background(data);
        for percent in [25, 50, 75] {
            let y = HEIGHT as i32 - percent * HEIGHT as i32 / 100;
            line(data, (0, y), ((WIDTH - 1) as i32, y), [45, 55, 65, 255]);
        }
        let mut bins = [0usize; 7];
        for ms in &tour.frames {
            bins[BOUNDS.iter().position(|b| ms <= b).unwrap_or(6)] += 1;
        }
        for (i, count) in bins.into_iter().enumerate() {
            let height = if tour.frames.is_empty() {
                0
            } else {
                count * (HEIGHT - 1) / tour.frames.len()
            };
            let x0 = i * WIDTH / 7 + 3;
            let x1 = (i + 1) * WIDTH / 7 - 3;
            let color = if i < 2 {
                [108, 220, 133, 255]
            } else if i < 5 {
                [245, 190, 80, 255]
            } else {
                [242, 105, 100, 255]
            };
            for x in x0..x1 {
                line(
                    data,
                    (x as i32, (HEIGHT - 1) as i32),
                    (x as i32, (HEIGHT - 1 - height) as i32),
                    color,
                );
            }
        }
    }
}

fn background(data: &mut [u8]) {
    for pixel in data.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[12, 18, 24, 255]);
    }
}

fn line(data: &mut [u8], from: (i32, i32), to: (i32, i32), color: [u8; 4]) {
    let steps = (to.0 - from.0).abs().max((to.1 - from.1).abs()).max(1);
    for step in 0..=steps {
        let x = from.0 + (to.0 - from.0) * step / steps;
        let y = from.1 + (to.1 - from.1) * step / steps;
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            let offset = (y as usize * WIDTH + x as usize) * 4;
            data[offset..offset + 4].copy_from_slice(&color);
        }
    }
}
