use super::*;

#[derive(Resource, Default)]
struct Run {
    phase: u8,
    case: usize,
    age: f32,
    shots: u32,
    shot_baseline: u32,
    effect_baseline: usize,
    reload_started: bool,
    rows: Vec<serde_json::Value>,
    captured: Option<usize>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<Run>()
        .add_systems(
            PreUpdate,
            drive.after(super::super::drive).before(crate::player::look),
        )
        .add_systems(
            PostUpdate,
            capture_pose.after(bevy::transform::TransformSystems::Propagate),
        );
}
fn capture_pose(
    v: Option<Res<Verification>>,
    mut run: ResMut<Run>,
    rigs: Query<(Entity, &crate::presentation::Rig, &GlobalTransform)>,
    nodes: Query<(
        Entity,
        Option<&Name>,
        &Transform,
        &GlobalTransform,
        Option<&bevy::animation::AnimatedBy>,
        Option<&bevy::mesh::skinning::SkinnedMesh>,
    )>,
    children: Query<&Children>,
) {
    let Some(v) = v else { return };
    if run.phase != 2 || run.captured == Some(run.case) {
        return;
    }
    let records: Vec<_> = rigs.iter().filter(|(_, r, _)| r.weapon.0.contains("fivesevendw")).map(|(entity, r, pose)| {
        let bones: Vec<_> = children.iter_descendants(entity).filter_map(|e| nodes.get(e).ok()).map(|(e, name, local, global, player, skin)| {
            serde_json::json!({"entity":e.to_bits(),"name":name.map(Name::as_str),"local":local.to_matrix().to_cols_array(),"world":global.to_matrix().to_cols_array(),"player":player.map(|p|p.0.to_bits()),"joints":skin.map(|s|s.joints.iter().map(|j|j.to_bits()).collect::<Vec<_>>())})
        }).collect();
        serde_json::json!({"entity":entity.to_bits(),"left":r.left,"arms":r.arms,"world":pose.to_matrix().to_cols_array(),"nodes":bones})
    }).collect();
    let _ = std::fs::write(
        v.output.join(format!("dual-{}-pose.json", run.case)),
        serde_json::to_vec(&records).unwrap(),
    );
    run.captured = Some(run.case);
}
fn drive(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut inventories: Query<
        (&mut WeaponInventory, &mut crate::solo_revive::PerkInventory),
        With<Player>,
    >,
    effects: Res<crate::perk_effects::EffectEvidence>,
    enemies: Query<Entity, With<Zombie>>,
    rigs: Query<&crate::presentation::Rig>,
    animations: Query<&AnimationPlayer>,
) {
    let (Some(v), Some(game), Ok((mut inventory, mut perks))) = (v, game, inventories.single_mut())
    else {
        return;
    };
    input.clear();
    run.age += time.delta_secs();
    if v.started.elapsed().as_secs() > 120 || **state == GameState::LoadError {
        run.phase = 99;
    }
    let id = if run.case == 0 {
        "weapon.fivesevendw_zm"
    } else {
        "weapon.fivesevendw_upgraded_zm"
    };
    let weapon = game.content.weapons.iter().find(|w| w.id.0 == id).unwrap();
    let magazines = inventory.active().map(|w| (w.magazine, w.left_magazine));
    let shots = run.shots;
    match run.phase {
        0 if **state == GameState::Playing && run.age > 2. => {
            for entity in &enemies {
                commands.entity(entity).despawn();
            }
            session.pending = 0;
            session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
            *inventory = WeaponInventory::temporary(weapon, 4);
            perks.owned.insert(
                game.content
                    .perks
                    .iter()
                    .find(|p| p.cherry.is_some())
                    .unwrap()
                    .id
                    .clone(),
            );
            run.shot_baseline = session.shots;
            run.effect_baseline = effects.events.len();
            run.reload_started = false;
            run.phase = 1;
            run.age = 0.;
        }
        1 if run.age > 1. => {
            let matching: Vec<_> = rigs
                .iter()
                .filter(|r| r.weapon == weapon.id && r.ready && r.attached)
                .collect();
            let ready = [false, true].into_iter().all(|left| {
                matching.iter().any(|r| r.left == left && !r.arms)
                    && matching.iter().any(|r| {
                        r.left == left
                            && r.arms
                            && r.players.iter().any(|e| {
                                animations
                                    .get(*e)
                                    .is_ok_and(|p| p.playing_animations().next().is_some())
                            })
                    })
            });
            if ready {
                run.rows.push(serde_json::json!({"weapon":id,"check":"two native models and independent hand animation players","passed":true}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("dual-{}-idle.png", run.case)),
                    ));
                run.phase = 2;
                run.age = 0.;
                run.shots = 0;
            }
        }
        2 if run.age > 0.15 => {
            let actual = session.shots.saturating_sub(run.shot_baseline);
            let expected = (
                weapon.magazine.saturating_sub(actual.div_ceil(2)),
                Some(weapon.magazine.saturating_sub(actual / 2)),
            );
            if actual < shots || actual > shots + 1 || magazines != Some(expected) {
                run.rows.push(serde_json::json!({"weapon":id,"check":"alternating independent magazine depletion","passed":false,"acknowledged_shots":shots,"actual_shots":actual,"actual":magazines,"expected":expected}));
                run.phase = 99;
            } else if actual > shots {
                run.shots = actual;
                run.age = 0.;
                if actual == 2 * weapon.magazine - 1 {
                    run.rows.push(serde_json::json!({"weapon":id,"check":"alternating independent magazine depletion","passed":true,"shots":actual,"actual":magazines}));
                    run.phase = 3;
                }
            } else {
                input.fire = true;
            }
        }
        3 => {
            if !run.reload_started {
                if inventory.hand_reload_timing(false).is_some()
                    && inventory.hand_reload_timing(true).is_some()
                {
                    run.reload_started = true;
                    run.age = 0.;
                } else {
                    input.reload = true;
                }
            } else if run.age > 0.5 {
                let independent = inventory
                    .hand_reload_timing(false)
                    .is_some_and(|(d, _)| (d - weapon.mechanics.empty_reload).abs() < 0.001)
                    && inventory
                        .hand_reload_timing(true)
                        .is_some_and(|(d, _)| (d - weapon.reload_seconds).abs() < 0.001);
                run.rows.push(serde_json::json!({"weapon":id,"check":"empty right and partial left reload durations","passed":independent}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(
                        v.output.join(format!("dual-{}-reload.png", run.case)),
                    ));
                run.phase = 4;
            }
        }
        4 if run.age > weapon.reload_seconds + 0.1 => {
            run.rows.push(serde_json::json!({"weapon":id,"check":"left reload completes while right remains empty","passed":magazines==Some((0,Some(weapon.magazine))),"actual":magazines}));
            run.phase = 5;
        }
        5 if run.age > weapon.mechanics.empty_reload + 0.2 => {
            run.rows.push(serde_json::json!({"weapon":id,"check":"both reloads complete with reserve conservation","passed":magazines==Some((weapon.magazine,Some(weapon.magazine))) && inventory.reserve(weapon)==2*weapon.magazine+1,"actual":magazines,"reserve":inventory.reserve(weapon)}));
            let discharges: Vec<_> = effects.events[run.effect_baseline..]
                .iter()
                .filter(|event| event["event"] == "cherry_discharge")
                .collect();
            run.rows.push(serde_json::json!({"weapon":id,"check":"one Electric Cherry discharge for paired reload with primary weapon identity","passed":discharges.len()==1 && discharges[0]["weapon"]==id,"discharges":discharges}));
            if run.case == 0 {
                run.case = 1;
                run.phase = 0;
                run.age = 0.;
            } else {
                run.phase = 99;
            }
        }
        99 => {
            let passed = run.rows.len() == 12 && run.rows.iter().all(|r| r["passed"] == true);
            let result = serde_json::json!({"passed":passed,"checks":run.rows});
            let _ = std::fs::write(v.output.join("dual-wield.json"), result.to_string());
            commands.write_message(if passed {
                AppExit::Success
            } else {
                AppExit::error()
            });
        }
        _ => {}
    }
}
