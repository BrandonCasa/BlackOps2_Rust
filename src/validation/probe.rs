//! Read-only airborne candidate inspection against the live native collision world.
use avian3d::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize)]
pub(super) struct Candidate {
    feet: [f32; 3],
    velocity: [f32; 3],
    aim: [f32; 3],
    #[serde(default)]
    runup_ticks: Option<u32>,
    #[serde(default)]
    include_tank: bool,
}

pub(super) fn remote(In(params): In<Option<Value>>, world: &mut World) -> bevy::remote::BrpResult {
    let candidates: Vec<Candidate> =
        serde_json::from_value(params.ok_or_else(|| super::error("Missing jump candidates"))?)
            .map_err(|e| super::error(e.to_string()))?;
    if !(1..=64).contains(&candidates.len())
        || candidates.iter().any(|c| {
            c.feet
                .iter()
                .chain(&c.velocity)
                .chain(&c.aim)
                .any(|v| !v.is_finite())
                || Vec3::from_array(c.velocity).length() > 30.
                || c.runup_ticks.is_some_and(|ticks| ticks > 60)
        })
    {
        return Err(super::error(
            "Expected 1..64 finite jump candidates with velocity at most 30 m/s",
        ));
    }
    world
        .run_system_cached_with(inspect, candidates)
        .map_err(|e| super::error(e.to_string()))?
        .map_err(super::error)
}

fn inspect(
    In(candidates): In<Vec<Candidate>>,
    slide: MoveAndSlide,
    definition: Res<crate::assets::GameDefinition>,
    time: Res<Time<Fixed>>,
    players: Query<(Entity, &crate::player::Player, &Collider)>,
    tank: Res<crate::tank::Tank>,
) -> Result<Value, String> {
    let (owner, player, collider) = players.single().map_err(|_| "Expected one player")?;
    let tuning = &definition.tuning;
    let dt = time.timestep().as_secs_f32();
    let duration = std::time::Duration::from_secs_f32(dt);
    let offset = Vec3::Y * player.height * 0.5;
    let mut results = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let filter = crate::environment::player_solids(
            [Some(owner), tank.entity.filter(|_| !candidate.include_tank)]
                .into_iter()
                .flatten(),
        );
        let mut position = Vec3::from_array(candidate.feet) + offset;
        let mut velocity = Vec3::from_array(candidate.velocity);
        let aim = Vec3::from_array(candidate.aim);
        let mut blocked = false;
        let mut landed = None;
        let mut ticks = 0;
        let mut takeoff = None;
        let mut runup = Vec::new();
        for tick in 1..=120 + candidate.runup_ticks.unwrap_or(0) {
            ticks = tick;
            let ground = candidate
                .runup_ticks
                .filter(|runup| tick <= runup + 1)
                .and_then(|_| {
                    slide
                        .spatial_query
                        .cast_shape(
                            collider,
                            position,
                            Quat::IDENTITY,
                            Dir3::NEG_Y,
                            &ShapeCastConfig::from_max_distance(tuning.physics.ground_probe),
                            &filter,
                        )
                        .and_then(|hit| {
                            crate::movement::physics::walkable_surface(
                                &slide,
                                hit.entity,
                                hit.point1,
                                hit.normal1,
                                &filter,
                                &tuning.physics,
                            )
                        })
                        .filter(|_| velocity.y <= 1e-3)
                });
            if let Some(runup_ticks) = candidate.runup_ticks {
                if tick <= runup_ticks + 1 {
                    runup.push(json!({"tick":tick,"feet":(position-offset).to_array(),
                        "velocity":velocity.to_array(),"grounded":ground.is_some()}));
                }
                if tick == runup_ticks + 1 {
                    if ground.is_none() {
                        break;
                    }
                    velocity.y = tuning.jump_speed;
                    takeoff = Some(
                        json!({"feet":(position-offset).to_array(),"velocity":velocity.to_array()}),
                    );
                }
            }
            let grounded = ground.is_some() && tick <= candidate.runup_ticks.unwrap_or(0);
            let direction = (aim - position).with_y(0.).normalize_or_zero();
            let wish = direction
                * if grounded {
                    tuning.sprint_speed
                } else {
                    tuning.walk_speed
                };
            velocity = tuning.physics.velocity(
                velocity,
                wish,
                ground.filter(|_| grounded).map(|(normal, _)| normal),
                false,
                false,
                false,
                false,
                dt,
            );
            let gravity = if grounded { 0. } else { tuning.gravity * dt };
            let move_velocity = velocity - Vec3::Y * gravity * 0.5;
            let result = crate::movement::physics::move_and_slide(
                &slide,
                &slide.spatial_query,
                crate::player::MotionShape {
                    collider,
                    radius: tuning.radius,
                    step_height: tuning.step_height,
                    grounded,
                },
                position,
                move_velocity,
                duration,
                &filter,
                &tuning.physics,
            )
            .slide;
            let travel = move_velocity.xz() * dt;
            blocked |= (result.position.xz() - position.xz()).dot(travel) + 1e-6
                < travel.length_squared() * 0.8;
            position = result.position;
            velocity = result.projected_velocity;
            if (velocity.y - move_velocity.y).abs() < 0.00001 {
                velocity.y -= gravity * 0.5;
            }
            if velocity.y <= 1e-3 {
                landed = slide
                    .spatial_query
                    .cast_shape(
                        collider,
                        position,
                        Quat::IDENTITY,
                        Dir3::NEG_Y,
                        &ShapeCastConfig::from_max_distance(tuning.physics.ground_probe),
                        &filter,
                    )
                    .and_then(|hit| {
                        crate::movement::physics::walkable_surface(
                            &slide,
                            hit.entity,
                            hit.point1,
                            hit.normal1,
                            &filter,
                            &tuning.physics,
                        )
                        .map(|_| hit.entity)
                    });
                if landed.is_some() && !grounded {
                    break;
                }
            }
        }
        results.push(
            json!({"feet":(position-offset).to_array(),"velocity":velocity.to_array(),
            "ticks":ticks,"blocked":blocked,"landed":landed.map(Entity::to_bits),
            "clear":slide.spatial_query.shape_intersections(collider,position+Vec3::Y*0.03,Quat::IDENTITY,&filter).is_empty(),
            "takeoff":takeoff,"runup":runup}),
        );
    }
    Ok(
        json!({"results":results,"scope":"Frozen world collision; tank included only when requested. Run-up assumes native sprint speed; platform motion, input state and pickup require a real run."}),
    )
}
