use super::*;
use crate::weapons::{WeaponAction, WeaponInventory};
use crate::{presentation::Rig, rendering::GameCamera};
use bevy::window::PrimaryWindow;

const WEAPONS: [&str; 5] = [
    "weapon.c96_zm",
    "weapon.m14_zm",
    "weapon.mp40_zm",
    "weapon.870mcs_zm",
    "weapon.beretta93r_zm",
];
const STEPS: [&str; 12] = [
    "hip",
    "entry",
    "held",
    "firing",
    "fire-return",
    "reload",
    "exit",
    "reentry",
    "paused",
    "resume",
    "switch-away",
    "switch-return",
];
#[derive(Resource, Default)]
struct Coverage {
    weapon: usize,
    aspect: usize,
    step: usize,
    age: f32,
    sent: bool,
    captured: bool,
    initialized: bool,
    reloading: bool,
    shots_before: u32,
    entry_resetting: bool,
    records: Vec<serde_json::Value>,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>()
        .add_systems(
            PreUpdate,
            drive.after(super::drive).before(crate::player::look),
        )
        .add_systems(Last, capture);
}
fn drive(
    mut commands: Commands,
    mut coverage: ResMut<Coverage>,
    verification: Option<Res<Verification>>,
    definition: Option<Res<GameDefinition>>,
    time: Res<Time<Real>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut inventories: Query<&mut WeaponInventory, With<Player>>,
    enemies: Query<Entity, With<Zombie>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    let (Some(ver), Some(def), Ok(mut inventory)) =
        (verification, definition, inventories.single_mut())
    else {
        return;
    };
    if !ver.ads {
        return;
    }
    input.clear();
    if ver.started.elapsed().as_secs() > 240 {
        let _ = std::fs::write(ver.output.join("ads.json"), serde_json::json!({"passed":false,"reason":"window workload timed out","records":coverage.records}).to_string());
        commands.write_message(AppExit::error());
        return;
    }
    if !matches!(**state, GameState::Playing | GameState::Paused) {
        return;
    }
    session.spawn_timer = Timer::from_seconds(3600., TimerMode::Once);
    session.intermission = None;
    session.pending = 1;
    for enemy in &enemies {
        commands.entity(enemy).despawn();
    }
    if coverage.aspect == 2 {
        coverage.age += time.delta_secs();
        if coverage.age > 3. {
            let complete = coverage.records.len() == WEAPONS.len() * STEPS.len() * 2
                && coverage.records.iter().all(|r| r["passed"] == true);
            let aligned = coverage
                .records
                .iter()
                .filter(|r| r["settled"] == true)
                .all(|r| r["optical_error_pixels"].as_f64().is_some_and(|e| e <= 2.));
            let result = serde_json::json!({"scenarios_passed":complete,"projected_optical_alignment_passed":aligned,
                "scope":"controlled inventory, enemy-free stationary actual game window; source sight tags are diagnostics, not automatically iron-sight centers", "records":coverage.records});
            let _ = std::fs::write(
                ver.output.join("ads.json"),
                serde_json::to_string_pretty(&result).unwrap(),
            );
            commands.write_message(if complete {
                AppExit::Success
            } else {
                AppExit::error()
            });
        }
        return;
    }
    if !coverage.initialized {
        let weapon = def
            .content
            .weapons
            .iter()
            .find(|w| w.id.0 == WEAPONS[coverage.weapon])
            .unwrap();
        *inventory = WeaponInventory::temporary(weapon, 4);
        let backup_id = if coverage.weapon == 0 {
            WEAPONS[2]
        } else {
            WEAPONS[0]
        };
        let backup = def
            .content
            .weapons
            .iter()
            .find(|w| w.id.0 == backup_id)
            .unwrap();
        let _ = inventory.grant(backup);
        coverage.initialized = true;
        coverage.shots_before = session.shots;
        coverage.age = 0.;
        return;
    }
    coverage.age += time.delta_secs();
    let step = coverage.step;
    input.ads = !matches!(step, 0 | 6 | 10);
    if step == 1 && !coverage.captured {
        if coverage.age > 0.15 && inventory.ads >= 1. {
            coverage.entry_resetting = true;
        }
        if coverage.entry_resetting {
            input.ads = false;
            if inventory.ads == 0. {
                coverage.entry_resetting = false;
                coverage.age = 0.;
            }
        }
    }
    if step == 3 && !coverage.sent {
        input.fire = true;
        coverage.sent = true;
    }
    if step == 5 {
        if !coverage.sent {
            input.reload = true;
            coverage.sent = true;
        }
        coverage.reloading |= inventory.action != WeaponAction::Ready;
    }
    if step == 8 && !coverage.sent {
        next.set(GameState::Paused);
        coverage.sent = true;
    }
    if step == 9 && **state == GameState::Paused {
        next.set(GameState::Playing);
    }
    if matches!(step, 10 | 11) && !coverage.sent {
        input.weapon_slot = Some(usize::from(step == 10));
        coverage.sent = true;
    }
    let duration = match step {
        1 => 0.06,
        3 => 0.25,
        8 => 0.6,
        5 => 0.6,
        _ => 1.5,
    };
    let ready = step != 5 || (coverage.reloading && inventory.action == WeaponAction::Ready);
    if coverage.age >= duration && coverage.captured && ready {
        coverage.step += 1;
        coverage.age = 0.;
        coverage.sent = false;
        coverage.captured = false;
        coverage.reloading = false;
        coverage.entry_resetting = false;
        if coverage.step == STEPS.len() {
            coverage.step = 0;
            coverage.weapon += 1;
            coverage.initialized = false;
            if coverage.weapon == WEAPONS.len() {
                coverage.weapon = 0;
                coverage.aspect += 1;
                if coverage.aspect == 1 {
                    if let Ok(mut window) = windows.single_mut() {
                        window.resolution.set_physical_resolution(1440, 1080);
                    }
                }
            }
        }
    }
}
fn capture(
    mut commands: Commands,
    verification: Option<Res<Verification>>,
    mut coverage: ResMut<Coverage>,
    inventories: Query<&WeaponInventory, With<Player>>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<GameCamera>>,
    rigs: Query<(Entity, &Rig)>,
    children: Query<&Children>,
    joints: Query<(&Name, &GlobalTransform)>,
    state: Res<State<GameState>>,
    session: Res<Session>,
    definition: Res<GameDefinition>,
) {
    let (Some(ver), Ok(inventory), Ok((camera, view, projection))) =
        (verification, inventories.single(), cameras.single())
    else {
        return;
    };
    if !ver.ads || coverage.aspect >= 2 || !coverage.initialized || coverage.captured {
        return;
    }
    let settled = matches!(coverage.step, 2 | 4 | 7 | 9 | 11);
    let delay = match coverage.step {
        1 => 0.03,
        3 => 0.12,
        5 => 0.3,
        8 => 0.25,
        _ => 1.,
    };
    if coverage.age < delay
        || (coverage.step == 1
            && (inventory.ads <= 0. || inventory.ads >= 1. || coverage.entry_resetting))
        || (settled && (inventory.ads < 0.999 || inventory.action != WeaponAction::Ready))
    {
        return;
    }
    let Some(active) = inventory.active() else {
        return;
    };
    let Some((entity, _)) = rigs.iter().find(|(_, r)| {
        !r.arms && !r.knife && r.weapon == active.definition && r.ready && r.attached
    }) else {
        return;
    };
    let Some((_, sight)) = children
        .iter_descendants(entity)
        .filter_map(|e| joints.get(e).ok())
        .find(|(n, _)| n.as_str() == "tag_sights")
    else {
        return;
    };
    let Ok(pixel) = camera.world_to_viewport(view, sight.translation()) else {
        return;
    };
    let size = camera.logical_viewport_size().unwrap_or_default();
    let error = pixel - size * 0.5;
    let optical = definition
        .content
        .weapons
        .iter()
        .find(|w| w.id == active.definition)
        .and_then(|w| w.mechanics.ads_sight.as_ref())
        .and_then(|s| {
            children
                .iter_descendants(entity)
                .filter_map(|e| joints.get(e).ok())
                .find(|(n, _)| n.as_str() == s.joint)
                .and_then(|(_, joint)| {
                    camera
                        .world_to_viewport(view, joint.transform_point(Vec3::from_array(s.point)))
                        .ok()
                })
        });
    let optical_error = optical.map(|p| p.distance(size * 0.5));
    let camera_point = view
        .affine()
        .inverse()
        .transform_point3(sight.translation());
    let fov = match projection {
        Projection::Perspective(p) => p.fov.to_degrees(),
        _ => 0.,
    };
    let name = format!(
        "{}-{}-{}",
        WEAPONS[coverage.weapon],
        if coverage.aspect == 0 { "16x9" } else { "4x3" },
        STEPS[coverage.step]
    );
    let return_error = if matches!(coverage.step, 4 | 7 | 9 | 11) {
        coverage
            .records
            .iter()
            .rev()
            .find(|r| r["case"] == name.replace(STEPS[coverage.step], "held"))
            .and_then(|r| {
                optical.map(|pixel| {
                    Vec2::new(
                        r["optical_pixel"][0].as_f64().unwrap() as f32,
                        r["optical_pixel"][1].as_f64().unwrap() as f32,
                    )
                    .distance(pixel)
                })
            })
    } else {
        None
    };
    let passed = match coverage.step {
        0 | 6 => inventory.ads < 0.001,
        1 => inventory.ads > 0. && inventory.ads < 1.,
        3 => session.shots > coverage.shots_before,
        5 => inventory.action != WeaponAction::Ready,
        8 => **state == GameState::Paused,
        10 => active.definition.0 != WEAPONS[coverage.weapon],
        _ => {
            active.definition.0 == WEAPONS[coverage.weapon]
                && inventory.ads >= 0.999
                && **state == GameState::Playing
        }
    } && return_error.is_none_or(|e| e <= 2.)
        && (!settled || optical_error.is_some_and(|e| e <= 2.));
    coverage.records.push(serde_json::json!({"case":name,"passed":passed,"settled":settled,"optical_pixel":optical.map(|p|p.to_array()),"optical_error_pixels":optical_error,"return_error_pixels":return_error,"shots":session.shots,"state":format!("{:?}",**state),"weapon":active.definition.0,"ads":inventory.ads,"action":format!("{:?}",inventory.action),"viewport":size.to_array(),"fov_vertical":fov,"sight_tag_camera":camera_point.to_array(),"sight_tag_pixel":pixel.to_array(),"sight_tag_error":error.to_array()}));
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(ver.output.join(name + ".png")));
    coverage.captured = true;
    let _ = std::fs::write(
        ver.output.join("ads-progress.json"),
        serde_json::json!({"records":coverage.records}).to_string(),
    );
}
