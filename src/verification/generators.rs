//! Opt-in exercise of production captures in the actual game window.
use super::*;
use crate::{
    generators::{GeneratorState, Generators},
    interactions::InteractionFocus,
    life::Vitality,
    movement::Mobility,
};
use avian3d::prelude::*;
#[derive(Resource, Default)]
pub(super) struct Coverage {
    phase: u8,
    site: usize,
    age: f32,
    center: Vec3,
    away: Vec3,
    aim: Vec3,
    rows: Vec<serde_json::Value>,
    baseline: u32,
    progress: f32,
    approaches: Vec<serde_json::Value>,
    abandoned: bool,
    progress_written_at: Option<Instant>,
    progress_written_state: Option<(u8, usize, usize)>,
}
impl Coverage {
    pub(super) fn finished(&self) -> bool {
        self.phase == 10
    }
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        FixedUpdate,
        drive
            .in_set(crate::gameplay::GameStep::Movement)
            .before(crate::player::move_player),
    );
}
fn drive(
    mut commands: Commands,
    v: Option<Res<Verification>>,
    game: Option<Res<GameDefinition>>,
    time: Res<Time<Fixed>>,
    state: Res<State<GameState>>,
    mut c: ResMut<Coverage>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    query: SpatialQuery,
    mut players: Query<
        (
            Entity,
            &mut Player,
            &mut Transform,
            &mut Vitality,
            &mut Mobility,
        ),
        Without<Zombie>,
    >,
    actors: Query<(Entity, Option<&crate::generators::GeneratorDefender>), With<Zombie>>,
    generators: Res<Generators>,
    focus: Res<InteractionFocus>,
    mut access: ResMut<crate::access::ZoneAccess>,
    mut next: ResMut<NextState<GameState>>,
) {
    let (Some(v), Some(game)) = (v, game) else {
        return;
    };
    if !v.generators {
        return;
    }
    let c = &mut *c;
    input.clear();
    if c.phase == 10 {
        return;
    }
    if c.phase == 8 && **state == GameState::GameOver {
        commands.write_message(crate::input::RestartGame);
        c.phase = 9;
        c.age = 0.;
        write_progress(&v, c, &game, &generators, &focus);
        return;
    }
    // Allocation instrumentation can slow a still-advancing fixed-step run.
    // Bound wall time independently from all original scenario clocks.
    if v.started.elapsed().as_secs() > 900 {
        c.rows.push(
            serde_json::json!({"case":"timeout","passed":false,"phase":c.phase,"site":c.site}),
        );
        finish(&mut commands, &v, &c, &generators);
        return;
    }
    if **state != GameState::Playing {
        return;
    }
    let Ok((entity, mut player, mut pose, mut health, mut mobility)) = players.single_mut() else {
        return;
    };
    c.age += time.delta_secs();
    session.pending = 6;
    session.spawn_timer = Timer::from_seconds(1000., TimerMode::Once);
    for (e, guard) in &actors {
        if guard.is_none() {
            commands.entity(e).despawn();
        }
    }
    let _ = health.regenerate(100);
    if c.phase < 8 {
        for d in &game.access.doors {
            let _ = access.unlock(&game, &d.connection);
        }
    }
    if c.site >= game.content.generators.len() {
        finish(&mut commands, &v, &c, &generators);
        return;
    }
    let d = &game.content.generators[c.site];
    let placement = game
        .content
        .placements
        .iter()
        .find(|p| p.id == d.placement)
        .unwrap();
    let origin = Vec3::from_array(placement.position);
    let capture = generators.sites.get(&d.id);
    if c.phase > 0 && c.phase < 8 {
        pose.translation = if c.phase == 4 { c.away } else { c.center };
        player.velocity = Vec3::ZERO;
        mobility.airborne = false;
        mobility.peak = pose.translation.y - player.height * 0.5;
        mobility.fall_speed = 0.;
        let eye = player.eye_height;
        super::semtex::aim(&mut player, &pose, c.aim, eye);
    }
    let mut advance = false;
    match c.phase {
        0 if c.age > 4. => {
            let rotation = Quat::from_rotation_y(placement.yaw);
            let mut center = None;
            c.approaches.clear();
            for radius in [0.75, 1., 1.5, 2., 2.5] {
                for sector in 0..12 {
                    let offset = Quat::from_rotation_y(sector as f32 * std::f32::consts::TAU / 12.)
                        * Vec3::X
                        * radius;
                    let position = super::semtex::standing(
                        &query,
                        entity,
                        origin + rotation * offset,
                        player.height,
                        game.tuning.radius,
                    );
                    let inside =
                        position.is_some_and(|p| p.y - player.height * 0.5 > origin.y - 0.508);
                    let obstruction = position.and_then(|p| {
                        let eye = p + Vec3::Y * (player.eye_height - player.height * 0.5);
                        let delta = crate::generators::use_point(origin, eye, 3.2512) - eye;
                        let direction = Dir3::new(delta).ok()?;
                        query.cast_ray(
                            eye,
                            direction,
                            (delta.length() - 0.12).max(0.),
                            true,
                            &crate::environment::solids([entity]),
                        )
                    });
                    c.approaches.push(serde_json::json!({"radius":radius,"sector":sector,"position":position.map(|p|p.to_array()),"inside":inside,"obstruction":obstruction.map(|h|h.distance),"origin":origin.to_array()}));
                    if inside && obstruction.is_none() {
                        center = position;
                        break;
                    }
                }
                if center.is_some() {
                    break;
                }
            }
            if let Some(center) = center {
                c.center = center;
                c.away = center + rotation * Vec3::X * 9.;
                c.aim = origin + Vec3::Y * 1.6256;
                session.points = if c.site == 0 { 199 } else { 1000 };
                advance = true;
            }
            if center.is_none() && c.age > 6. {
                c.rows.push(serde_json::json!({"case":"supported native capture approach","passed":false,"generator":d.number,"approaches":c.approaches}));
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("approach-failure.png")));
                finish(&mut commands, &v, &c, &generators);
            }
        }
        1 => {
            if c.site != 0 {
                advance = true;
            } else if c.age > 0.5 {
                c.rows.push(serde_json::json!({"case":"insufficient funds","passed":session.points==199 && capture.is_some_and(|s|s.state==GeneratorState::Inactive) && focus.rejection==Some(crate::interactions::InteractionRejection::InsufficientPoints)}));
                advance = true;
            }
            input.use_held = c.site == 0;
        }
        2 if c.age > 0.2 => {
            session.points = 1000;
            c.baseline = generators.activations;
            advance = true;
        }
        3 => {
            input.use_held = true;
            if c.age > 3. {
                c.rows.push(serde_json::json!({"case":"atomic activation and held latch","generator":d.number,"passed":session.points==800 && generators.activations==c.baseline+1 && capture.is_some_and(|s|s.state==GeneratorState::Capturing && s.progress>0.1)}));
                c.progress = capture.map_or(0., |s| s.progress);
                advance = true;
            }
        }
        4 if c.site == 0 && !c.abandoned && c.age > 8.5 => {
            c.rows.push(serde_json::json!({"case":"abandonment disables and clears defenders without refund","passed":capture.is_some_and(|s|s.state==GeneratorState::Disabled && s.progress==0.) && session.points==800 && generators.defenders_cleared>=4 && generators.playback.iter().any(|a|a.ends_with(":3"))}));
            c.abandoned = true;
            c.phase = 2;
            c.age = 0.;
        }
        4 if (c.site != 0 || c.abandoned) && c.age > 1. => {
            c.rows.push(serde_json::json!({"case":"capture decays outside","generator":d.number,"passed":capture.is_some_and(|s|s.progress<c.progress && s.progress>0.)}));
            advance = true;
        }
        5 if capture.is_some_and(|s| s.state == GeneratorState::Active) => {
            if c.site == 0 {
                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(v.output.join("generator-active.png")));
            }
            c.rows.push(serde_json::json!({"case":"resume and complete","generator":d.number,"passed":session.points==900}));
            advance = true;
        }
        6 if c.age > 1.2 => {
            c.rows.push(serde_json::json!({"case":"completion refund","generator":d.number,"passed":session.points==1100 && capture.is_some_and(|s|s.progress==1.)}));
            if c.site + 1 < game.content.generators.len() {
                c.site += 1;
                c.phase = 0;
                c.age = 3.9;
            } else {
                advance = true;
            }
        }
        7 => {
            c.rows.push(serde_json::json!({"case":"six original pumps and separate defenders","passed":generators.active_count()==6 && generators.animated==6 && generators.defenders_admitted>=4 && generators.defenders_cleared>=4 && generators.playing_sounds>=12 && generators.playback.iter().any(|a|a.ends_with(":0")) && generators.playback.iter().any(|a|a.ends_with(":1")),"admitted":generators.defenders_admitted,"cleared":generators.defenders_cleared,"audio_sinks":generators.playing_sounds,"playback":generators.playback}));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(v.output.join("six-generators.png")));
            next.set(GameState::GameOver);
            advance = true;
        }
        8 if c.age > 0.2 => {
            commands.write_message(crate::input::RestartGame);
            advance = true;
        }
        9 if c.age > 1. => {
            c.rows.push(serde_json::json!({"case":"fresh session restoration","passed":generators.active_count()==0 && generators.activations==0 && generators.sites.len()==6}));
            finish(&mut commands, &v, &c, &generators);
            c.phase = 10;
        }
        _ => {}
    }
    if advance {
        c.phase += 1;
        c.age = 0.;
    }
    write_progress(&v, c, &game, &generators, &focus);
}
/// Keep inspection receipts useful without 60 synchronous JSON writes per
/// second contaminating the gameplay workload being measured.
fn write_progress(
    v: &Verification,
    c: &mut Coverage,
    game: &GameDefinition,
    generators: &Generators,
    focus: &InteractionFocus,
) {
    let state = (c.phase, c.site, c.rows.len());
    if c.progress_written_state == Some(state)
        && c.progress_written_at
            .is_some_and(|at| at.elapsed().as_secs_f32() < 1.)
    {
        return;
    }
    let capture = game
        .content
        .generators
        .get(c.site)
        .and_then(|d| generators.sites.get(&d.id));
    let receipt = serde_json::json!({
        "phase":c.phase,"site":c.site,"progress":capture.map(|s|s.progress),
        "elapsed_wall_seconds":v.started.elapsed().as_secs_f64(),
        "rows":c.rows,"defenders":generators.defenders_admitted,
        "failures":generators.defender_failures,"prompt":focus.prompt,
        "rejection":format!("{:?}",focus.rejection),"approaches":c.approaches,
    });
    let _ = std::fs::write(
        v.output.join("generator-progress.json"),
        receipt.to_string(),
    );
    c.progress_written_at = Some(Instant::now());
    c.progress_written_state = Some(state);
}
fn finish(commands: &mut Commands, v: &Verification, c: &Coverage, g: &Generators) {
    let passed = c.rows.len() >= 28 && c.rows.iter().all(|r| r["passed"] == true);
    let _=std::fs::write(v.output.join("generators.json"),serde_json::json!({"passed":passed,"rows":c.rows,"admitted":g.defenders_admitted,"admission_failures":g.defender_failures}).to_string());
    commands.write_message(if passed {
        AppExit::Success
    } else {
        AppExit::error()
    });
}
