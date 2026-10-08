//! Frozen native scene comparisons: the only changed occluders are overlay meshes.
use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    player::Player,
    rendering::{ForegroundTexture, ShadowCacheStats, TextureStreamingStats, TextureStreams},
    weapons::{WeaponAction, WeaponInventory},
};
use bevy::{
    camera::visibility::{CascadesVisibleEntities, CubemapVisibleEntities, RenderLayers},
    light::NotShadowCaster,
    post_process::auto_exposure::AutoExposure,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::{collections::HashSet, path::PathBuf, time::Instant};

#[derive(Resource)]
struct Run {
    output: PathBuf,
    started: Instant,
    changed: Instant,
    stage: usize,
    initialized: bool,
    rows: Vec<serde_json::Value>,
    capture_pending: bool,
}
const LABELS: [&str; 8] = [
    "local-lit-on",
    "local-mask-on",
    "local-mask-off",
    "local-mask-restored",
    "sun-lit-on",
    "sun-mask-on",
    "sun-mask-off",
    "sun-mask-restored",
];
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup)
        .add_systems(
            PreUpdate,
            input
                .after(crate::input::collect)
                .before(crate::player::look),
        )
        .add_systems(Last, observe);
}
fn setup(mut commands: Commands, options: Res<LaunchOptions>) {
    let output = options.verify.as_ref().unwrap();
    if output.starts_with(env!("CARGO_MANIFEST_DIR")) || std::fs::create_dir_all(output).is_err() {
        commands.write_message(AppExit::error());
        return;
    }
    commands.insert_resource(Run {
        output: output.clone(),
        started: Instant::now(),
        changed: Instant::now(),
        stage: 0,
        initialized: false,
        rows: vec![],
        capture_pending: false,
    });
}
fn input(mut input: ResMut<crate::input::GameInput>) {
    *input = crate::input::GameInput::default();
}
fn observe(
    mut commands: Commands,
    mut run: ResMut<Run>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut options: ResMut<LaunchOptions>,
    definition: Option<Res<GameDefinition>>,
    streams: Res<TextureStreams>,
    streaming: Res<TextureStreamingStats>,
    cache: Res<ShadowCacheStats>,
    exposure: Query<Entity, With<AutoExposure>>,
    casters: Query<
        (
            Entity,
            &InheritedVisibility,
            Has<NotShadowCaster>,
            &RenderLayers,
        ),
        With<ForegroundTexture>,
    >,
    mut players: Query<
        (&mut Player, &mut Transform, &WeaponInventory),
        Without<crate::rendering::GameCamera>,
    >,
    lights: (
        Query<(
            Entity,
            &DirectionalLight,
            &CascadesVisibleEntities,
            &RenderLayers,
            &GlobalTransform,
        )>,
        Query<(Entity, &PointLight, &CubemapVisibleEntities, &RenderLayers)>,
    ),
) {
    if run.started.elapsed().as_secs() > 300
        || streams.failure().is_some()
        || **state == GameState::LoadError
    {
        finish(&mut commands, &run, &streaming, false);
        return;
    }
    if run.stage == LABELS.len() {
        if run.changed.elapsed().as_secs_f32() > 4. {
            let passed = run
                .rows
                .iter()
                .all(|r| r["native_admission_passed"] == true);
            finish(&mut commands, &run, &streaming, passed);
        }
        return;
    }
    if run.capture_pending {
        if run.changed.elapsed().as_secs_f32() < 3. {
            return;
        }
        run.capture_pending = false;
        run.stage += 1;
        run.changed = Instant::now();
        options.material_debug = match run.stage {
            1..=3 => 12,
            5..=7 => 10,
            _ => 0,
        };
        if matches!(run.stage, 2 | 6) {
            for (entity, visible, _, _) in &casters {
                if visible.get() {
                    commands.entity(entity).insert(NotShadowCaster);
                }
            }
        } else if matches!(run.stage, 3 | 7) {
            for (entity, _, _, _) in &casters {
                commands.entity(entity).remove::<NotShadowCaster>();
            }
        } else if run.stage == 4 {
            let Some(definition) = definition else { return };
            let Ok((mut player, mut body, _)) = players.single_mut() else {
                return;
            };
            // The existing native scene survey's outdoor vantage, with the
            // authored sun direction. Physics/animation remain paused.
            body.translation =
                Vec3::from_array(definition.survey_views[4]) + Vec3::Y * player.height * 0.5;
            if let Some((_, _, _, _, pose)) = lights.0.iter().next() {
                let direction = pose.rotation() * Vec3::Z;
                player.yaw = (-direction.x).atan2(-direction.z);
                player.pitch = 0.;
            }
        }
        return;
    }
    let visible: HashSet<_> = casters
        .iter()
        .filter(|(_, visibility, _, _)| visibility.get())
        .map(|(e, _, _, _)| e)
        .collect();
    if **state == GameState::Playing
        && visible.len() >= 5
        && players
            .iter()
            .any(|(_, _, inventory)| inventory.action == WeaponAction::Ready)
    {
        next.set(GameState::Paused);
        return;
    }
    if **state != GameState::Paused || visible.len() < 5 {
        return;
    }
    if !run.initialized {
        // Fixed native camera exposure makes shadow factors comparable across
        // on/off/restored captures instead of metering each diagnostic anew.
        for camera in &exposure {
            commands.entity(camera).remove::<AutoExposure>();
        }
        run.initialized = true;
        run.changed = Instant::now();
        return;
    }
    // Local mask weighting uses native BRDF/normal maps. Wait for the complete
    // accepted stationary allocation, rather than only the foreground queue.
    if run.changed.elapsed().as_secs_f32() < 5. || !streaming.settled() {
        return;
    }
    let sun_rows: Vec<_> = lights.0.iter().map(|(entity, light, entities, layers, _)| {
        let admitted: HashSet<_> = entities.entities.values().flatten().flat_map(|v| &v.entities).copied().filter(|e| visible.contains(e)).collect();
        serde_json::json!({"light":entity.to_bits(),"shadow_enabled":light.shadow_maps_enabled,"overlay_layer_intersects":layers.intersects(&RenderLayers::layer(1)),"native_casters":admitted.iter().map(|e| e.to_bits()).collect::<Vec<_>>()})
    }).collect();
    let point_rows: Vec<_> = lights.1.iter().filter_map(|(entity, light, entities, layers)| {
        let admitted: HashSet<_> = entities.iter().flat_map(|v| &v.entities).copied().filter(|e| visible.contains(e)).collect();
        (!admitted.is_empty()).then(|| serde_json::json!({"light":entity.to_bits(),"shadow_enabled":light.shadow_maps_enabled,"overlay_layer_intersects":layers.intersects(&RenderLayers::layer(1)),"native_casters":admitted.iter().map(|e| e.to_bits()).collect::<Vec<_>>()}))
    }).collect();
    let off = matches!(run.stage, 2 | 6);
    let all_excluded = casters
        .iter()
        .filter(|(e, _, _, _)| visible.contains(e))
        .all(|(_, _, excluded, _)| excluded == off);
    let sun_admitted = sun_rows.iter().any(|r| {
        r["native_casters"]
            .as_array()
            .is_some_and(|v| !v.is_empty())
    });
    let admitted = if off {
        !sun_admitted && point_rows.is_empty()
    } else if run.stage < 4 {
        !point_rows.is_empty()
    } else {
        sun_admitted
    };
    let stats = cache.report();
    let passed = all_excluded
        && admitted
        && stats["rendered_directional_cascades"].as_u64().unwrap_or(0) > 0
        && stats["rendered_cube_faces"].as_u64().unwrap_or(0) > 0;
    let label = LABELS[run.stage];
    let row = serde_json::json!({"case":label,"material_debug":options.material_debug,"automatic_exposure_cameras":exposure.iter().count(),"player_pose":players.iter().map(|(player,body,inventory)|serde_json::json!({"position":body.translation.to_array(),"yaw":player.yaw,"pitch":player.pitch,"action":format!("{:?}",inventory.action)})).collect::<Vec<_>>(),"native_admission_passed":passed,"casters":visible.iter().map(|e|e.to_bits()).collect::<Vec<_>>(),"sun":sun_rows,"points":point_rows,"shadow_cache":stats,"streaming":&*streaming});
    let _ = std::fs::write(run.output.join(format!("{label}.json")), row.to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(run.output.join(format!("{label}.png"))));
    run.rows.push(row);
    // Changes below affect the following diagnostic only after this frozen
    // scene has had time to render its requested screenshot.
    run.capture_pending = true;
    run.changed = Instant::now();
}
fn finish(commands: &mut Commands, run: &Run, stats: &TextureStreamingStats, passed: bool) {
    let _ = std::fs::write(run.output.join("viewmodel-shadows.json"), serde_json::json!({"native_admission_passed":passed,"scope":"native shadow caster admission, rendered pass/cache counters and frozen overlay-only on/off/restored screenshots; inspect receiver occlusion in captures","cases":run.rows,"streaming":stats}).to_string());
    commands.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
