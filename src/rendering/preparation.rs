use crate::{GameState, assets::LoadFailure, presentation::ViewModelCamera};
use bevy::{
    camera::visibility::{NoFrustumCulling, VisibilitySystems},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        render_resource::{CachedPipelineState, PipelineCache},
    },
    shader::ShaderCacheError,
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

#[derive(Clone, Default, serde::Serialize)]
pub(crate) struct PipelineProgress {
    pub preparing: bool,
    pub frame: u64,
    pub total: usize,
    pub ready: usize,
    pub error: Option<String>,
}

#[derive(Resource, Clone, Default)]
pub(crate) struct ShaderProgress(pub Arc<Mutex<PipelineProgress>>);

#[derive(Resource, Default)]
struct Preparation {
    started: Option<Instant>,
    frame: u64,
    total: usize,
    settled_frames: u32,
    shaders_ready: bool,
}

#[derive(Component)]
struct LoadingScreen;
#[derive(Component)]
struct LoadingLabel;
#[derive(Component)]
struct LoadingBar;
#[derive(Component)]
struct PreparationMesh;

pub(crate) fn install(app: &mut App) {
    let progress = ShaderProgress::default();
    app.insert_resource(progress.clone())
        .init_resource::<Preparation>()
        .add_systems(
            Startup,
            setup.after(crate::presentation::first_person_light),
        )
        .add_systems(OnEnter(GameState::PreparingShaders), begin)
        .add_systems(OnExit(GameState::PreparingShaders), restore_culling)
        .add_systems(Update, (advance, display).chain())
        .add_systems(
            PostUpdate,
            warm_meshes
                .run_if(in_state(GameState::PreparingShaders))
                .before(VisibilitySystems::CheckVisibility),
        );
    if let Some(render) = app.get_sub_app_mut(RenderApp) {
        render
            .insert_resource(progress)
            .add_systems(Render, inspect.after(RenderSystems::Render));
    }
}

fn inspect(cache: Res<PipelineCache>, progress: Res<ShaderProgress>) {
    let Ok(mut progress) = progress.0.lock() else {
        return;
    };
    if !progress.preparing {
        return;
    }
    progress.frame += 1;
    progress.total = 0;
    progress.ready = 0;
    for pipeline in cache.pipelines() {
        progress.total += 1;
        match &pipeline.state {
            CachedPipelineState::Ok(_) => progress.ready += 1,
            CachedPipelineState::Err(error)
                if !matches!(
                    error,
                    ShaderCacheError::ShaderNotLoaded(_)
                        | ShaderCacheError::ShaderImportNotYetAvailable
                ) =>
            {
                progress.error.get_or_insert_with(|| error.to_string());
            }
            _ => {}
        }
    }
}

fn begin(mut preparation: ResMut<Preparation>, progress: Res<ShaderProgress>) {
    if let Ok(mut progress) = progress.0.lock() {
        progress.preparing = true;
        progress.error = None;
    }
    *preparation = Preparation {
        started: Some(Instant::now()),
        ..default()
    };
    info!("Preparing Origins shaders before gameplay");
}

fn warm_meshes(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<NoFrustumCulling>)>,
) {
    for entity in &meshes {
        commands
            .entity(entity)
            .insert((NoFrustumCulling, PreparationMesh));
    }
}

fn restore_culling(
    mut commands: Commands,
    meshes: Query<Entity, With<PreparationMesh>>,
    progress: Res<ShaderProgress>,
) {
    if let Ok(mut progress) = progress.0.lock() {
        progress.preparing = false;
    }
    for entity in &meshes {
        commands
            .entity(entity)
            .remove::<(NoFrustumCulling, PreparationMesh)>();
    }
}

fn advance(
    state: Res<State<GameState>>,
    progress: Res<ShaderProgress>,
    loading: Res<crate::cinematics::LoadingPlayback>,
    mut preparation: ResMut<Preparation>,
    surfaces: Res<super::OpaqueSurfaceStatus>,
    atmosphere: Res<super::atmosphere::AtmosphereState>,
    grade: Res<super::grading::SourceGrade>,
    rigs: Query<&crate::presentation::Rig>,
    options: Res<crate::assets::LaunchOptions>,
    mut failure: ResMut<LoadFailure>,
    mut next: ResMut<NextState<GameState>>,
) {
    if **state != GameState::PreparingShaders {
        return;
    }
    let Ok(progress) = progress.0.lock() else {
        return;
    };
    if let Some(error) = &progress.error {
        failure.0 = format!("Shader preparation failed: {error}");
        error!("{}", failure.0);
        next.set(GameState::LoadError);
        return;
    }
    if !preparation.shaders_ready
        && preparation
            .started
            .is_some_and(|started| started.elapsed().as_secs() > 180)
    {
        failure.0 = format!(
            "Shader preparation timed out: {} of {} ready.",
            progress.ready, progress.total
        );
        error!("{}", failure.0);
        next.set(GameState::LoadError);
        return;
    }
    if preparation.frame == progress.frame {
        return;
    }
    preparation.frame = progress.frame;
    let models_ready = options.flythrough
        || (!rigs.is_empty()
            && rigs
                .iter()
                .all(|rig| rig.ready && (rig.arms || rig.attached)));
    if surfaces.ready()
        && atmosphere.shader_ready
        && grade.ready()
        && models_ready
        && progress.total > 0
        && progress.ready == progress.total
        && preparation.total == progress.total
    {
        preparation.settled_frames += 1;
    } else {
        preparation.settled_frames = 0;
    }
    preparation.total = progress.total;
    preparation.shaders_ready |= preparation.settled_frames >= 8;
    if preparation.shaders_ready && loading.done {
        info!(
            "Origins shaders ready: {} pipelines in {:.2}s",
            progress.total,
            preparation
                .started
                .map_or(0., |started| started.elapsed().as_secs_f32())
        );
        next.set(GameState::Playing);
    }
}

fn setup(mut commands: Commands, cameras: Query<Entity, With<ViewModelCamera>>) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    commands
        .spawn((
            LoadingScreen,
            UiTargetCamera(camera),
            GlobalZIndex(100),
            BackgroundColor(Color::srgb(0.015, 0.02, 0.025)),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: px(20.),
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                LoadingLabel,
                Text::new("Loading Origins..."),
                TextFont {
                    font_size: FontSize::Px(24.),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            parent
                .spawn((
                    BackgroundColor(Color::srgb(0.12, 0.14, 0.16)),
                    Node {
                        width: percent(40.),
                        max_width: px(560.),
                        height: px(8.),
                        ..default()
                    },
                ))
                .with_child((
                    LoadingBar,
                    BackgroundColor(Color::srgb(0.7, 0.8, 0.45)),
                    Node {
                        width: percent(0.),
                        height: percent(100.),
                        ..default()
                    },
                ));
        });
}

fn display(
    state: Res<State<GameState>>,
    progress: Res<ShaderProgress>,
    mut screens: Query<&mut Visibility, With<LoadingScreen>>,
    mut labels: Query<&mut Text, With<LoadingLabel>>,
    mut bars: Query<&mut Node, With<LoadingBar>>,
) {
    let loading = matches!(**state, GameState::Loading | GameState::PreparingShaders);
    for mut visibility in &mut screens {
        *visibility = if loading {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
    if !loading {
        return;
    }
    let Ok(progress) = progress.0.lock() else {
        return;
    };
    let percent_ready = if **state == GameState::PreparingShaders && progress.total > 0 {
        100. * progress.ready as f32 / progress.total as f32
    } else {
        0.
    };
    for mut label in &mut labels {
        label.0 = if **state == GameState::PreparingShaders {
            format!("Preparing shaders... {:.0}%", percent_ready)
        } else {
            "Loading Origins...".into()
        };
    }
    for mut bar in &mut bars {
        bar.width = percent(percent_ready);
    }
}
