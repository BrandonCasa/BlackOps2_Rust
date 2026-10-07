//! Small game-loop timing display. These are frame intervals, not GPU timings.
use std::{collections::VecDeque, fmt::Write};

use bevy::{prelude::*, window::PrimaryWindow};

use crate::{GameState, presentation::ViewModelCamera};

const CAPACITY: usize = 4096;
const WINDOW_SECONDS: f64 = 5.;
const REFRESH_SECONDS: f64 = 0.25;

#[derive(Resource)]
struct FrameStats {
    enabled: bool,
    intervals: VecDeque<f64>,
    sorted: Vec<f64>,
    elapsed: f64,
    refresh: f64,
}

impl Default for FrameStats {
    fn default() -> Self {
        Self {
            enabled: true,
            intervals: VecDeque::with_capacity(CAPACITY),
            sorted: Vec::with_capacity(CAPACITY),
            elapsed: 0.,
            refresh: 0.,
        }
    }
}

impl FrameStats {
    fn clear(&mut self) {
        self.intervals.clear();
        self.elapsed = 0.;
        self.refresh = 0.;
    }
}

#[derive(Component)]
struct PerformancePanel;
#[derive(Component)]
struct AverageFps;
#[derive(Component)]
struct Percentile;

pub(super) struct PerformanceOverlayPlugin;
impl Plugin for PerformanceOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameStats>()
            .add_systems(
                Startup,
                setup.after(crate::presentation::first_person_light),
            )
            .add_systems(Update, update);
    }
}

fn setup(mut commands: Commands, cameras: Query<Entity, With<ViewModelCamera>>) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    commands
        .spawn((
            PerformancePanel,
            UiTargetCamera(camera),
            GlobalZIndex(30),
            Visibility::Hidden,
            BackgroundColor(Color::srgba(0.02, 0.025, 0.02, 0.75)),
            Node {
                position_type: PositionType::Absolute,
                left: px(16.),
                top: px(16.),
                padding: UiRect::axes(px(10.), px(6.)),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                AverageFps,
                Text::new("AVG FPS  —"),
                TextFont {
                    font_size: FontSize::Px(22.),
                    ..default()
                },
                TextColor(Color::srgb(0.65, 1., 0.45)),
            ));
            parent.spawn((
                Percentile,
                Text::new("P95  — ms   /   — FPS"),
                TextFont {
                    font_size: FontSize::Px(14.),
                    ..default()
                },
                TextColor(Color::srgb(0.85, 0.88, 0.85)),
            ));
        });
}

fn update(
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    keys: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut stats: ResMut<FrameStats>,
    mut panel: Query<&mut Visibility, With<PerformancePanel>>,
    mut texts: ParamSet<(
        Query<&mut Text, With<AverageFps>>,
        Query<&mut Text, With<Percentile>>,
    )>,
) {
    if keys.just_pressed(KeyCode::F12) {
        stats.enabled = !stats.enabled;
        stats.clear();
    }
    let active = stats.enabled
        && **state == GameState::Playing
        && windows.single().is_ok_and(|window| window.focused);
    if let Ok(mut visibility) = panel.single_mut() {
        let desired = if active {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != desired {
            *visibility = desired;
        }
    }
    if !active || state.is_changed() {
        if !stats.intervals.is_empty() {
            stats.clear();
        }
        return;
    }
    let interval = time.delta_secs_f64();
    if !interval.is_finite() || interval <= 0. {
        return;
    }
    // Every frame performs only bounded queue/sum work. Allocate once at startup.
    if stats.intervals.len() == CAPACITY {
        stats.elapsed -= stats.intervals.pop_front().unwrap_or(0.);
    }
    stats.intervals.push_back(interval);
    stats.elapsed += interval;
    while stats.intervals.len() > 1
        && stats
            .intervals
            .front()
            .is_some_and(|old| stats.elapsed - old >= WINDOW_SECONDS)
    {
        stats.elapsed -= stats.intervals.pop_front().unwrap_or(0.);
    }
    stats.refresh += interval;
    if stats.refresh < REFRESH_SECONDS {
        return;
    }
    stats.refresh %= REFRESH_SECONDS;
    let fps = stats.intervals.len() as f64 / stats.elapsed;
    let FrameStats {
        intervals, sorted, ..
    } = &mut *stats;
    sorted.clear();
    sorted.extend(intervals.iter().copied());
    // Same percentile-index convention as the existing benchmark summary.
    let index = ((sorted.len() - 1) as f64 * 0.95).round() as usize;
    let (_, p95, _) = sorted.select_nth_unstable_by(index, f64::total_cmp);
    let p95 = *p95;
    if let Ok(mut text) = texts.p0().single_mut() {
        text.0.clear();
        let _ = write!(text.0, "AVG FPS  {fps:.1}");
    }
    if let Ok(mut text) = texts.p1().single_mut() {
        text.0.clear();
        let _ = write!(
            text.0,
            "P95  {:.2} ms   /   {:.1} FPS",
            p95 * 1000.,
            1. / p95
        );
    }
}
