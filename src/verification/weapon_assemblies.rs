//! Opt-in catalog pose survey inside origins.exe; controlled poses are explicit.
use super::*;
use crate::{
    origins::content::{WeaponAnimation, WeaponId},
    weapons::{WeaponAction, WeaponIntent, WeaponInventory, WeaponSignal},
};
use std::collections::BTreeSet;

#[derive(Resource, Default)]
struct Survey {
    catalog: Vec<WeaponId>,
    cursor: usize,
    phase: usize,
    sample: usize,
    age: f32,
    begun: bool,
    isolated: bool,
    paused: bool,
    resumed: bool,
    rows: Vec<serde_json::Value>,
    world: Option<Entity>,
    world_fitted: bool,
    world_captured: bool,
    restarting: bool,
    capture_pending: Option<(f32, bool)>,
}
#[derive(Component)]
struct WorldPreview;
#[derive(Component)]
struct WorldReady;
const POSES: [(WeaponAnimation, WeaponSignal); 15] = [
    (WeaponAnimation::Equip, WeaponSignal::Equip),
    (WeaponAnimation::Idle, WeaponSignal::Idle),
    (WeaponAnimation::AdsEnter, WeaponSignal::Idle),
    (WeaponAnimation::Fire, WeaponSignal::Fire),
    (WeaponAnimation::Reload, WeaponSignal::Reload),
    (WeaponAnimation::ReloadEmpty, WeaponSignal::ReloadEmpty),
    (WeaponAnimation::ReloadStart, WeaponSignal::ReloadStart),
    (WeaponAnimation::ReloadEnd, WeaponSignal::ReloadEnd),
    (WeaponAnimation::LastShot, WeaponSignal::Fire),
    (WeaponAnimation::IdleEmpty, WeaponSignal::Idle),
    (WeaponAnimation::Rechamber, WeaponSignal::Fire),
    (WeaponAnimation::Melee, WeaponSignal::Melee),
    (WeaponAnimation::Prime, WeaponSignal::Prime),
    (WeaponAnimation::Unequip, WeaponSignal::Unequip),
    (WeaponAnimation::Idle, WeaponSignal::Idle),
];
pub fn install(app: &mut App) {
    app.init_resource::<Survey>()
        .add_systems(
            PreUpdate,
            drive.after(super::drive).before(crate::player::look),
        )
        .add_systems(
            Update,
            hold_shot
                .before(crate::presentation::PresentationAnimation)
                .before(crate::audio::AudioPlayback),
        )
        .add_observer(world_ready);
}
fn hold_shot(
    survey: Res<Survey>,
    game: Option<Res<GameDefinition>>,
    mut players: Query<&mut WeaponInventory, With<Player>>,
) {
    if !survey.begun || !matches!(survey.phase, 3 | 8 | 10) {
        return;
    }
    let (Some(game), Ok(mut inv)) = (game, players.single_mut()) else {
        return;
    };
    let Some(weapon) = game
        .content
        .weapons
        .iter()
        .find(|w| Some(&w.id) == survey.catalog.get(survey.cursor))
    else {
        return;
    };
    let duration = |action| {
        weapon
            .animation_actions
            .get(&action)
            .and_then(|id| game.content.animations.iter().find(|a| a.id == *id))
            .and_then(|a| a.notifications.iter().find(|n| n.event == "end"))
            .map_or(0., |n| n.seconds)
    };
    let action = POSES[survey.phase].0;
    let fraction = [0.25, 0.5, 0.75][survey.sample];
    let age = if survey.phase == 10 {
        duration(WeaponAnimation::Fire) + duration(action) * fraction
    } else {
        duration(action) * fraction
    };
    inv.survey_shot_pose(age, survey.phase == 8);
}
fn world_ready(
    event: On<bevy::world_serialization::WorldInstanceReady>,
    mut commands: Commands,
    roots: Query<(), With<WorldPreview>>,
    children: Query<&Children>,
    meshes: Query<&MeshMaterial3d<StandardMaterial>, With<Mesh3d>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !roots.contains(event.entity) {
        return;
    }
    commands.entity(event.entity).insert(WorldReady);
    for entity in children
        .iter_descendants(event.entity)
        .filter(|e| meshes.contains(*e))
    {
        commands.entity(entity).insert((
            bevy::camera::visibility::RenderLayers::layer(1),
            bevy::light::NotShadowCaster,
        ));
        // Match the production wall-pickup material preparation.
        if let Ok(material) = meshes.get(entity)
            && let Some(source) = materials.get(&material.0)
        {
            let filled = crate::rendering::with_actor_fill(source);
            commands
                .entity(entity)
                .insert(MeshMaterial3d(materials.add(filled)));
        }
    }
}
fn drive(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    state: Res<State<GameState>>,
    mut next: ResMut<NextState<GameState>>,
    time: Res<Time>,
    mut survey: ResMut<Survey>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    mut players: Query<(&mut WeaponInventory, &mut Player), With<Player>>,
    enemies: Query<Entity, With<Zombie>>,
    evidence: Res<crate::presentation::EquipmentPresentationEvidence>,
    server: Res<AssetServer>,
    cameras: Query<Entity, With<crate::rendering::GameCamera>>,
    mut worlds: Query<&mut Transform, With<WorldReady>>,
    geometry: (
        Query<&Children>,
        Query<&bevy::camera::primitives::Aabb, With<Mesh3d>>,
    ),
) {
    let Some(v) = v else {
        return;
    };
    if !v.weapon_assemblies {
        return;
    }
    if **state == GameState::LoadError || v.started.elapsed().as_secs() > 1800 {
        let _ = std::fs::write(v.output.join("weapon-assemblies-failure.json"), serde_json::json!({"state":format!("{:?}", **state),"reason":"load failure or catalog deadline; inspect stderr.log"}).to_string());
        commands.write_message(AppExit::error());
        return;
    }
    let (Some(game), Ok((mut inv, mut player))) = (game, players.single_mut()) else {
        return;
    };
    input.clear();
    if !survey.isolated {
        for entity in &enemies {
            commands.entity(entity).despawn();
        }
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(100_000., TimerMode::Once));
        let mut seen = BTreeSet::new();
        survey.catalog = game
            .content
            .weapons
            .iter()
            .filter(|w| w.presentation.is_some())
            .filter(|w| {
                seen.insert((
                    w.presentation.clone(),
                    w.ads_presentation.clone(),
                    w.pickup.clone(),
                    w.animation_actions.clone(),
                ))
            })
            .map(|w| w.id.clone())
            .collect();
        survey.isolated = true;
        player.pitch = 0.;
    }
    survey.age += time.delta_secs();
    if survey.cursor == survey.catalog.len() {
        if !survey.restarting {
            next.set(GameState::GameOver);
            survey.restarting = true;
            survey.age = 0.;
            return;
        }
        if **state == GameState::GameOver {
            commands.write_message(crate::input::RestartGame);
            survey.age = 0.;
            return;
        }
        if survey.age < 0.5 {
            return;
        }
        let restarted = **state == GameState::Playing
            && session.round == 1
            && inv
                .active()
                .is_some_and(|w| w.definition.0 == "weapon.c96_zm");
        let passed = !survey.rows.is_empty()
            && survey.rows.iter().all(|r| r["visible"] == true)
            && restarted;
        let result = serde_json::json!({"passed": passed, "scope": "controlled native catalog animation poses in the real game window; no acquisition or unsupported weapon behavior is claimed", "distinct_compositions": survey.catalog.len(), "pause":survey.paused,"resume":survey.resumed,"restart":restarted,"frames": survey.rows});
        let _ = std::fs::write(v.output.join("weapon-assemblies.json"), result.to_string());
        commands.write_message(if passed {
            AppExit::Success
        } else {
            AppExit::error()
        });
        return;
    }
    if survey.age > 30. {
        let _ = std::fs::write(v.output.join("weapon-assemblies-failure.json"), serde_json::json!({"weapon":survey.catalog[survey.cursor].0,"phase":survey.phase,"views":evidence.views,"captured":survey.rows}).to_string());
        commands.write_message(AppExit::error());
        return;
    }
    if **state != GameState::Playing {
        if **state == GameState::Paused && survey.paused && survey.age > 0.3 {
            next.set(GameState::Playing);
            survey.resumed = true;
            survey.age = 0.;
        }
        return;
    }
    let weapon = game
        .content
        .weapons
        .iter()
        .find(|w| w.id == survey.catalog[survey.cursor])
        .unwrap();
    if let Some((until, moving)) = survey.capture_pending {
        if survey.age < until {
            return;
        }
        survey.capture_pending = None;
        if moving && survey.sample < 2 {
            survey.sample += 1;
        } else {
            survey.phase += 1;
            survey.begun = false;
            survey.age = 0.;
        }
        return;
    }
    if survey.phase == POSES.len() {
        if let Some(scene) = weapon
            .pickup
            .as_ref()
            .and_then(|id| game.content.presentations.iter().find(|p| p.id == *id))
            .and_then(|p| p.scene.as_ref())
        {
            if survey.world.is_none() {
                *inv = WeaponInventory::default();
                let entity = commands
                    .spawn((
                        bevy::world_serialization::WorldAssetRoot(server.load(scene.clone())),
                        Transform::from_xyz(0., -0.1, -1.)
                            .with_rotation(Quat::from_rotation_y(0.8)),
                        Visibility::Inherited,
                        WorldPreview,
                    ))
                    .id();
                if let Ok(camera) = cameras.single() {
                    commands.entity(camera).add_child(entity);
                }
                survey.world = Some(entity);
                survey.world_fitted = false;
                survey.world_captured = false;
                survey.age = 0.;
                return;
            }
            if survey.age < 0.4 || !survey.world.is_some_and(|e| worlds.contains(e)) {
                return;
            }
            if !survey.world_fitted {
                let entity = survey.world.unwrap();
                let bounds = geometry
                    .0
                    .iter_descendants(entity)
                    .filter_map(|e| geometry.1.get(e).ok())
                    .fold(None, |bounds: Option<(Vec3, Vec3)>, aabb| {
                        let low = Vec3::from(aabb.center - aabb.half_extents);
                        let high = Vec3::from(aabb.center + aabb.half_extents);
                        Some(bounds.map_or((low, high), |(a, b)| (a.min(low), b.max(high))))
                    });
                let Some((low, high)) = bounds else { return };
                let mut pose = worlds.get_mut(entity).unwrap();
                let distance = (high - low).length() + 0.3;
                pose.translation = Vec3::NEG_Z * distance - pose.rotation * ((low + high) * 0.5);
                survey.world_fitted = true;
                survey.age = 0.;
                return;
            }
            let name = format!("{:03}-{}-World.png", survey.cursor, weapon.id.0);
            if !survey.world_captured {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join(&name)));
                survey.rows.push(serde_json::json!({"weapon":weapon.id.0,"phase":"World","image":name,"visible":true}));
                // Keep the scene alive through render extraction and screenshot
                // readback; immediate despawn produces an empty world capture.
                survey.world_captured = true;
                survey.age = 0.;
                return;
            }
            if let Some(entity) = survey.world.take() {
                commands.entity(entity).despawn();
            }
        }
        survey.cursor += 1;
        survey.phase = 0;
        survey.begun = false;
        survey.age = 0.;
        if !survey.paused {
            survey.paused = true;
            next.set(GameState::Paused);
        }
        return;
    }
    if !survey.begun {
        survey.sample = 0;
        let mut staged = weapon.clone();
        // Ammo-free native helper definitions must still stage the requested
        // nonempty Idle/Fire pose, rather than implicitly selecting IdleEmpty.
        staged.magazine = match survey.phase {
            8 => 1,
            9 => 0,
            _ => staged.magazine.max(2),
        };
        *inv = WeaponInventory::temporary(&staged, 3);
        inv.action = WeaponAction::Ready;
        inv.signal = POSES[survey.phase].1;
        inv.sequence = (survey.cursor as u64) * 100 + survey.phase as u64 + 1;
        if matches!(survey.phase, 3 | 8 | 10) {
            inv.tick(
                0.,
                WeaponIntent {
                    pressed: true,
                    ..default()
                },
                &game.content.weapons,
            );
        }
        inv.ads = if survey.phase == 2 { 1. } else { 0. };
        survey.begun = true;
        survey.age = 0.;
    }
    // Hold the selected pose independent of normal action/gameplay progression.
    input.ads = survey.phase == 2;
    if survey.phase == 2 {
        inv.ads = 1.;
    }
    let action = POSES[survey.phase].0;
    let actual = if action == WeaponAnimation::AdsEnter {
        WeaponAnimation::Idle
    } else {
        action
    };
    if (actual != WeaponAnimation::Idle && !weapon.animation_actions.contains_key(&actual))
        || (survey.phase == 2 && !weapon.mechanics.ads_enabled)
    {
        survey.phase += 1;
        survey.begun = false;
        survey.age = 0.;
    } else {
        let key = format!("{}:{actual:?}", weapon.id.0);
        let duration = |action| {
            weapon
                .animation_actions
                .get(&action)
                .and_then(|id| game.content.animations.iter().find(|a| a.id == *id))
                .and_then(|a| a.notifications.iter().find(|n| n.event == "end"))
                .map_or(0.2, |n| n.seconds)
        };
        let moving = !matches!(
            action,
            WeaponAnimation::Idle | WeaponAnimation::IdleEmpty | WeaponAnimation::AdsEnter
        );
        let fraction = if moving {
            [0.25, 0.5, 0.75][survey.sample]
        } else {
            0.35
        };
        let capture_at = if action == WeaponAnimation::Rechamber {
            duration(WeaponAnimation::Fire) + duration(action) * fraction
        } else if moving {
            (duration(actual) * fraction).max(0.035)
        } else {
            (duration(actual) * 0.35).clamp(0.035, 0.6)
        };
        if survey.age > capture_at
            && let Some(view) = evidence.views.get(&key)
        {
            let visible =
                view["ready"] == true && view["attached"] == true && view["visible"] == true;
            let label = if survey.phase == 14 {
                "ReturnIdle".to_owned()
            } else {
                format!("{action:?}")
            };
            let name = format!(
                "{:03}-{}-{label}-{}.png",
                survey.cursor, weapon.id.0, survey.sample
            );
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(v.output.join(&name)));
            survey.rows.push(serde_json::json!({"weapon":weapon.id.0,"phase":label,"fraction":fraction,"image":name,"visible":visible,"view":view}));
            // Preserve this action/sample until the requested screenshot has
            // passed through extraction and readback.
            survey.capture_pending = Some((survey.age + 0.12, moving));
        }
    }
}
