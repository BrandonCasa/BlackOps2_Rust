//! Original CreateFX placements and their script-owned activation.
use super::*;

#[derive(Deserialize)]
pub(super) struct MapEffect {
    name: String,
    origin: [f32; 3],
    angles: [f32; 3],
    start_time: f32,
    exploder: Option<u32>,
}

/// Convert native pitch/yaw/roll around Y/Z/X into the engine's Y-up frame.
pub(super) fn rotation(angles: [f32; 3]) -> Quat {
    let [pitch, yaw, roll] = angles.map(f32::to_radians);
    Quat::from_rotation_y(yaw) * Quat::from_rotation_z(-pitch) * Quat::from_rotation_x(roll)
}

#[derive(Component)]
pub(super) struct Exploder {
    placement: usize,
    emitter: Option<Entity>,
    played: bool,
}

fn play(commands: &mut Commands, placement: &MapEffect) -> Entity {
    emit(
        commands,
        &placement.name,
        Transform::from_translation(native(placement.origin))
            .with_rotation(rotation(placement.angles)),
    )
}

pub(super) fn spawn(mut commands: Commands, catalog: Res<Catalog>) {
    for (index, placement) in catalog.map_effects.iter().enumerate() {
        if placement.exploder.is_some() {
            commands.spawn((
                SessionEntity,
                Exploder {
                    placement: index,
                    emitter: None,
                    played: false,
                },
            ));
        } else {
            let entity = play(&mut commands, placement);
            let start = placement.start_time;
            commands
                .entity(entity)
                .entry::<Emitter>()
                .and_modify(move |mut emitter| {
                    // _fx.csc waits for positive delays. CScr_PlayFX's sixth
                    // argument is an absolute source start time (004f9d10).
                    emitter.elapsed = -start;
                    emitter.start_ms = Some((start * 1000.) as i32 as u32);
                });
        }
    }
}

pub(super) fn activate(
    mut commands: Commands,
    catalog: Res<Catalog>,
    generators: Res<crate::generators::Generators>,
    portals: Res<crate::portals::Portals>,
    game: Res<GameDefinition>,
    mut exploders: Query<&mut Exploder>,
) {
    if !generators.is_changed() && !portals.is_changed() {
        return;
    }
    for mut state in &mut exploders {
        let placement = &catalog.map_effects[state.placement];
        let active = match placement.exploder.unwrap() {
            // monolith_crysal_glow: zombie-controlled crystals own this loop.
            id @ 5001..=5006 => !generators.powered(
                &game
                    .content
                    .generators
                    .iter()
                    .find(|generator| u32::from(generator.number) == id - 5000)
                    .expect("Original generator exploder number")
                    .id,
            ),
            222 => portals.excavation_open,
            // The source defines 333 but never activates it in ordinary play.
            333 => false,
            id => panic!("Unsupported original CreateFX exploder {id}"),
        };
        if active && state.emitter.is_none() && (!state.played || placement.exploder != Some(222)) {
            state.emitter = Some(play(&mut commands, placement));
            state.played = true;
        } else if !active && let Some(entity) = state.emitter.take() {
            stop(&mut commands, entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_angles_preserve_forward_and_up_axes() {
        for angles in [
            [0., 0., 0.],
            [-90., 0., 0.],
            [270., 0., 90.],
            [15.9883, 223.818, -2.05218],
        ] {
            let [p, y, _] = angles.map(f32::to_radians);
            let forward = Vec3::new(p.cos() * y.cos(), -p.sin(), -p.cos() * y.sin());
            assert!((rotation(angles) * Vec3::X).abs_diff_eq(forward, 0.000001));
            assert!((rotation(angles) * Vec3::Y).dot(forward).abs() < 0.000001);
        }
    }
}
