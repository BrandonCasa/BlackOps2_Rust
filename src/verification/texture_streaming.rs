use crate::{
    GameState,
    assets::{GameDefinition, LaunchOptions},
    input::GameInput,
    player::Player,
    presentation::ViewModelGroup,
    rendering::{TextureStreamingStats, TextureStreams, TextureUse},
    weapons::{WeaponAction, WeaponInventory},
};
use bevy::{
    gltf::GltfMaterialExtras,
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};
use std::{path::PathBuf, time::Instant};

#[derive(Resource)]
struct Run {
    output: PathBuf,
    started: Instant,
    changed: Instant,
    phase: usize,
    rows: Vec<serde_json::Value>,
    failures: Vec<String>,
    restart_sent: bool,
    competition_frames: [u64; 16],
    peak_pending: [usize; 4],
    native_passes_active: bool,
    capture_pending: bool,
    scene_started: bool,
    last_status: Instant,
    bootstrap_conservation_frames: u64,
    bootstrap_registration_frames: u64,
    last_registered_bootstrap_bytes: u64,
}
const LABELS: [&str; 5] = [
    "mauser-arms",
    "late-mp40",
    "frag-prime",
    "mp40-ads",
    "restart-mauser",
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
        phase: 0,
        rows: vec![],
        failures: vec![],
        restart_sent: false,
        competition_frames: [0; 16],
        peak_pending: [0; 4],
        native_passes_active: false,
        capture_pending: false,
        scene_started: false,
        last_status: Instant::now(),
        bootstrap_conservation_frames: 0,
        bootstrap_registration_frames: 0,
        last_registered_bootstrap_bytes: 0,
    });
}
fn input(run: Res<Run>, mut input: ResMut<GameInput>) {
    *input = GameInput::default();
    if run.phase == 2 {
        input.grenade_held = true;
    }
    input.ads = run.phase == 3;
}
fn observe(
    mut commands: Commands,
    mut run: ResMut<Run>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    game: Option<Res<GameDefinition>>,
    streams: Res<TextureStreams>,
    stats: Res<TextureStreamingStats>,
    server: Res<AssetServer>,
    materials: Res<Assets<StandardMaterial>>,
    parents: Query<&ChildOf>,
    groups: Query<(), With<ViewModelGroup>>,
    rigs: Query<(
        Entity,
        &crate::presentation::Rig,
        Option<&Visibility>,
        Option<&InheritedVisibility>,
        Option<&WorldAssetRoot>,
    )>,
    meshes: Query<
        (
            Entity,
            &GltfMaterialExtras,
            Option<&TextureUse>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            &GlobalTransform,
            &InheritedVisibility,
            Has<bevy::light::NotShadowCaster>,
        ),
        With<Mesh3d>,
    >,
    cameras: Query<&bevy::camera::visibility::VisibleEntities, With<crate::rendering::GameCamera>>,
    mut players: Query<(&mut WeaponInventory, &mut Player)>,
) {
    if stats.indexed_bootstrap_gpu_bytes > 0 {
        run.bootstrap_conservation_frames += 1;
        if stats.registered_bootstrap_gpu_bytes + stats.bootstrap_reserved_bytes
            != stats.indexed_bootstrap_gpu_bytes
        {
            run.failures.push(
                "bootstrap resident/reservation transfer did not conserve indexed bytes".into(),
            );
            finish(&mut commands, &run, &stats);
            return;
        }
        if stats.registered_bootstrap_gpu_bytes > run.last_registered_bootstrap_bytes {
            run.bootstrap_registration_frames += 1;
        }
        run.last_registered_bootstrap_bytes = stats.registered_bootstrap_gpu_bytes;
    }
    let rig_rows: Vec<_> = rigs.iter().map(|(entity, rig, visibility, inherited, root)| {
        let mut ancestry = vec![];
        let mut ancestor = Some(entity);
        while let Some(e) = ancestor {
            ancestry.push(e.to_bits());
            ancestor = parents.get(e).ok().map(ChildOf::parent);
        }
        serde_json::json!({"entity":entity.to_bits(),"ancestry":ancestry,"weapon":rig.weapon.0,"arms":rig.arms,"knife":rig.knife,"ready":rig.ready,"attached":rig.attached,"visibility":format!("{visibility:?}"),"inherited_visibility":inherited.map(|v|v.get()),"scene_loaded":root.map(|r|server.is_loaded_with_dependencies(r.0.id())),"scene_load_state":root.map(|r|format!("{:?}",server.get_load_state(r.0.id())))})
    }).collect();
    if run.last_status.elapsed().as_secs_f32() > 5. {
        let _ = std::fs::write(run.output.join("live-state.json"), serde_json::json!({"phase":run.phase,"state":format!("{:?}",**state),"age":run.changed.elapsed().as_secs_f32(),"rigs":rig_rows,"players":players.iter().map(|(inventory,_)|format!("{:?} {:?}",inventory.action,inventory.signal)).collect::<Vec<_>>(),"streaming":&*stats}).to_string());
        run.last_status = Instant::now();
    }
    let mut active = 0;
    for index in 0..4 {
        if stats.pending_by_class[index] > 0 {
            active |= 1 << index;
        }
        run.peak_pending[index] = run.peak_pending[index].max(stats.pending_by_class[index]);
    }
    run.competition_frames[active] += 1;
    run.native_passes_active |= stats.shadow_requests > 0;
    if run.started.elapsed().as_secs() > 240
        || streams.failure().is_some()
        || **state == GameState::LoadError
    {
        run.failures.push(format!(
            "deadline/state/streaming: {:?} {:?}",
            **state,
            streams.failure()
        ));
        finish(&mut commands, &run, &stats);
        return;
    }
    if run.phase == 5 {
        if **state == GameState::GameOver && !run.restart_sent {
            commands.write_message(crate::input::RestartGame);
            run.restart_sent = true;
        } else if **state == GameState::Playing && run.restart_sent {
            run.phase = 4;
            run.changed = Instant::now();
        }
        return;
    }
    if run.phase == 6 {
        let settled = stats.settled();
        if (settled && run.changed.elapsed().as_secs_f32() > 3.)
            || run.changed.elapsed().as_secs_f32() > 40.
        {
            if !settled {
                run.failures
                    .push("stationary native demand did not drain in 40 seconds".into());
            }
            finish(&mut commands, &run, &stats);
        }
        return;
    }
    let Some(game) = game else { return };
    let Ok((mut inventory, mut player)) = players.single_mut() else {
        return;
    };
    if run.capture_pending {
        if run.changed.elapsed().as_secs_f32() > 3. {
            run.capture_pending = false;
            advance(&mut run, &game, &mut inventory, &mut player, &mut next);
        }
        return;
    }
    if **state == GameState::Playing && !run.scene_started && run.phase == 0 {
        run.scene_started = true;
        run.changed = Instant::now();
        return;
    }
    let age = run.changed.elapsed().as_secs_f32();
    let expected = match run.phase {
        0 | 4 => "weapon.c96_zm",
        1 | 3 => "weapon.mp40_zm",
        2 => "weapon.frag_grenade_zm",
        _ => unreachable!(),
    };
    let settled = inventory.action == WeaponAction::Ready
        && rigs.iter().any(|(_, rig, _, visible, _)| {
            rig.weapon.0 == expected
                && !rig.arms
                && !rig.knife
                && rig.ready
                && rig.attached
                && visible.is_some_and(|v| v.get())
        });
    if **state == GameState::Playing && age > 2. && settled {
        next.set(GameState::Paused);
    }
    if **state != GameState::Paused || age < if run.phase == 3 { 3. } else { 10. } {
        return;
    }
    let mut first_person = Vec::new();
    let mut world = Vec::new();
    let camera_meshes = cameras
        .single()
        .ok()
        .map(|v| v.get(std::any::TypeId::of::<Mesh3d>()));
    for (entity, extras, usage, material, pose, visible, excluded) in &meshes {
        if !visible.get() {
            continue;
        }
        let mut ancestor = Some(entity);
        let mut viewmodel = false;
        let mut rig_weapon = None;
        let mut arms = false;
        while let Some(e) = ancestor {
            if rig_weapon.is_none()
                && let Ok((_, rig, _, _, _)) = rigs.get(e)
            {
                rig_weapon = Some(rig.weapon.0.clone());
                arms = rig.arms;
            }
            if groups.contains(e) {
                viewmodel = true;
                break;
            }
            ancestor = parents.get(e).ok().map(ChildOf::parent);
        }
        let metadata: serde_json::Value = serde_json::from_str(&extras.value).unwrap_or_default();
        let channels=metadata["surface"]["channels"].as_array().map(|channels|channels.iter().map(|channel| {
            let handle=channel["uri"].as_str().and_then(|uri|server.get_handle::<Image>(uri.to_owned()));
            serde_json::json!({"role":channel["role"],"uri":channel["uri"],"residency":handle.and_then(|h|streams.residency(h.id()))})
        }).collect::<Vec<_>>()).unwrap_or_default();
        let sampled = material.and_then(|m| materials.get(&m.0)).map(|m| {
            [
                &m.base_color_texture,
                &m.normal_map_texture,
                &m.metallic_roughness_texture,
                &m.emissive_texture,
                &m.clearcoat_texture,
            ]
            .into_iter()
            .flatten()
            .map(|h| streams.residency(h.id()))
            .collect::<Vec<_>>()
        });
        let row = serde_json::json!({"entity":entity.to_bits(),"weapon":rig_weapon,"arms":arms,"casts_shadows":!excluded,"identity":metadata["surface"]["identity"],"position":pose.translation().to_array(),"usage":usage.map(|u|u.textures.iter().map(|id|streams.residency(*id)).collect::<Vec<_>>()),"channels":channels,"sampled":sampled});
        if viewmodel {
            first_person.push(row);
        } else if world.len() < 40 && camera_meshes.is_some_and(|meshes| meshes.contains(&entity)) {
            world.push(row);
        }
    }
    let full = !first_person.is_empty()
        && first_person.iter().all(|r| {
            r["sampled"].as_array().is_some_and(|v| {
                !v.is_empty()
                    && v.iter().all(|t| {
                        t["resident_mip"] == 0 && t["wanted_mip"] == 0 && t["foreground"] == true
                    })
            })
        });
    if !full && age < 30. {
        return;
    }
    let visible_weapon = first_person
        .iter()
        .any(|row| row["weapon"] == expected && row["arms"] == false);
    if !visible_weapon && age < 30. {
        return;
    }
    let world_detail = world
        .iter()
        .filter(|row| {
            row["usage"].as_array().is_some_and(|maps| {
                !maps.is_empty()
                    && maps.iter().all(|map| {
                        map["resident_mip"]
                            .as_u64()
                            .zip(map["wanted_mip"].as_u64())
                            .is_some_and(|(resident, wanted)| resident <= wanted)
                    })
            })
        })
        .count();
    let passed = full && visible_weapon && streams.failure().is_none() && world_detail > 0;
    let label = LABELS[run.phase];
    let row = serde_json::json!({"case":label,"passed":passed,"first_person":first_person,"rigs":rig_rows,"world":world,"world_meshes_at_requested_detail":world_detail,"camera_yaw":player.yaw,"streaming":&*stats,"age_seconds":age});
    let _ = std::fs::write(run.output.join(format!("{label}.json")), row.to_string());
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(run.output.join(format!("{label}.png"))));
    run.rows.push(row);
    run.changed = Instant::now();
    run.capture_pending = true;
}
fn advance(
    run: &mut Run,
    game: &GameDefinition,
    inventory: &mut WeaponInventory,
    player: &mut Player,
    next: &mut NextState<GameState>,
) {
    run.changed = Instant::now();
    match run.phase {
        0 => {
            let weapon = game
                .content
                .weapons
                .iter()
                .find(|w| w.id.0 == "weapon.mp40_zm")
                .expect("validated native MP40");
            *inventory = WeaponInventory::temporary(weapon, 3);
            inventory.action = WeaponAction::Ready;
            player.yaw += std::f32::consts::PI;
            run.phase = 1;
            next.set(GameState::Playing);
        }
        1 => {
            player.yaw += std::f32::consts::FRAC_PI_2;
            run.phase = 2;
            next.set(GameState::Playing);
        }
        2 => {
            player.yaw -= std::f32::consts::PI;
            run.phase = 3;
            inventory.ads = 1.;
            next.set(GameState::Playing);
        }
        3 => {
            run.phase = 5;
            next.set(GameState::GameOver);
        }
        4 => {
            run.phase = 6;
        }
        _ => unreachable!(),
    }
}
fn finish(commands: &mut Commands, run: &Run, stats: &TextureStreamingStats) {
    let arm_ids = |row: &serde_json::Value| {
        let mut ids = std::collections::BTreeMap::new();
        if let Some(meshes) = row["first_person"].as_array() {
            for mesh in meshes.iter().filter(|m| m["arms"] == true) {
                if let Some(textures) = mesh["sampled"].as_array() {
                    for texture in textures {
                        ids.insert(texture["uri"].to_string(), texture["id"].clone());
                    }
                }
            }
        }
        ids
    };
    let stable_arms = run.rows.first().is_some_and(|first| {
        let ids = arm_ids(first);
        !ids.is_empty() && run.rows.iter().all(|row| arm_ids(row) == ids)
    });
    let restart_retired = run
        .rows
        .first()
        .zip(run.rows.get(4))
        .is_some_and(|(first, last)| {
            let old: std::collections::BTreeSet<_> = first["first_person"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|m| m["entity"].as_u64())
                .collect();
            last["first_person"].as_array().is_some_and(|meshes| {
                meshes
                    .iter()
                    .all(|m| m["entity"].as_u64().is_some_and(|id| !old.contains(&id)))
            })
        });
    let passed = run.rows.len() == LABELS.len()
        && run.rows.iter().all(|r| r["passed"] == true)
        && run.failures.is_empty()
        && stats.errors == 0
        && stats.standard_refreshes > 0
        && stats.opaque_refreshes > 0
        && stable_arms
        && restart_retired
        && run.native_passes_active
        && run
            .competition_frames
            .iter()
            .enumerate()
            .any(|(mask, frames)| mask & 1 != 0 && mask & 6 != 0 && *frames > 0)
        && stats.uploads_by_class[1] > 0
        && stats.uploads_by_class[2] > 0
        && stats.peak_active_workers <= 4
        && stats.detected_vram_bytes > 0
        && run.bootstrap_conservation_frames > 0
        && run.bootstrap_registration_frames > 1
        && stats.residency_budget_bytes <= stats.detected_vram_bytes / 4 * 3
        && stats.peak_accounted_gpu_bytes <= stats.residency_budget_bytes;
    let receipt = serde_json::json!({"passed":passed,"scope":"native first-person and camera-visible world mip residency with three camera turns, explicit controlled MP40 grant and ADS pose, real Frag prime and session restart under native GI/shadows; no combat, acquisition or throughput benchmark", "bootstrap_conservation_frames":run.bootstrap_conservation_frames,"bootstrap_registration_frames":run.bootstrap_registration_frames,"stable_arm_image_ids":stable_arms,"restart_retired_old_meshes":restart_retired,"native_lighting_and_shadow_demand":run.native_passes_active,"competition_frames_by_class_bitmask":run.competition_frames,"peak_pending_by_class":run.peak_pending,"class_order":["foreground","camera","indirect","prefetch"],"cases":run.rows,"failures":run.failures,"streaming":stats});
    let _ = std::fs::write(
        run.output.join("texture-streaming.json"),
        receipt.to_string(),
    );
    commands.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
