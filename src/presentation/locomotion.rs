use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::GameStep,
    input::GameInput,
    life::{LifeState, Vitality},
    movement::Mobility,
    origins::content::{WeaponAnimation, WeaponDefinition, WeaponId},
    player::{Player, TickActions},
    weapons::{WeaponAction, WeaponInventory, WeaponSignal},
};
use bevy::prelude::*;

#[derive(Component, Default)]
pub(crate) struct WeaponLocomotion {
    weapon: Option<WeaponId>,
    pub(crate) phase: Option<WeaponAnimation>,
    pub(crate) elapsed: f32,
    duration: f32,
    diving: bool,
    landed: bool,
    queued_fire: [bool; 2],
    sprinting: bool,
}

impl WeaponLocomotion {
    fn start(&mut self, phase: WeaponAnimation, duration: f32) {
        self.phase = Some(phase);
        self.elapsed = 0.;
        self.duration = duration;
    }

    pub(crate) fn sample(
        &self,
        weapon: &WeaponDefinition,
        empty: bool,
    ) -> Option<(WeaponAnimation, f32, f32, bool)> {
        let phase = self.phase?;
        let action = if empty {
            match phase {
                WeaponAnimation::ProneEnter => WeaponAnimation::ProneEnterEmpty,
                WeaponAnimation::ProneLoop => WeaponAnimation::ProneLoopEmpty,
                WeaponAnimation::ProneBack => WeaponAnimation::ProneBackEmpty,
                WeaponAnimation::ProneRight => WeaponAnimation::ProneRightEmpty,
                WeaponAnimation::ProneLeft => WeaponAnimation::ProneLeftEmpty,
                WeaponAnimation::ProneLeave => WeaponAnimation::ProneLeaveEmpty,
                WeaponAnimation::DiveEnter => WeaponAnimation::DiveEnterEmpty,
                WeaponAnimation::DiveLoop => WeaponAnimation::DiveLoopEmpty,
                WeaponAnimation::DiveLeave => WeaponAnimation::DiveLeaveEmpty,
                WeaponAnimation::SprintEnter => WeaponAnimation::SprintEnterEmpty,
                WeaponAnimation::SprintLoop => WeaponAnimation::SprintLoopEmpty,
                WeaponAnimation::SprintLeave => WeaponAnimation::SprintLeaveEmpty,
                _ => phase,
            }
        } else {
            phase
        };
        let action = if weapon.animation_actions.contains_key(&action) {
            action
        } else if weapon.animation_actions.contains_key(&phase) {
            phase
        } else {
            return None;
        };
        let looping = matches!(
            phase,
            WeaponAnimation::ProneLoop
                | WeaponAnimation::ProneBack
                | WeaponAnimation::ProneRight
                | WeaponAnimation::ProneLeft
                | WeaponAnimation::DiveLoop
                | WeaponAnimation::SprintLoop
        );
        let duration = self.duration;
        Some((action, self.elapsed, duration, looping))
    }
}

pub(crate) fn install(app: &mut App) {
    app.register_required_components::<Player, WeaponLocomotion>()
        .add_systems(
            FixedUpdate,
            update
                .after(GameStep::Movement)
                .before(GameStep::Interactions)
                .run_if(in_state(GameState::Playing)),
        );
}

fn direction(input: Vec2) -> (WeaponAnimation, usize) {
    if input.y * input.y * 4. > input.x * input.x {
        if input.y > 0. {
            (WeaponAnimation::ProneLoop, 1)
        } else {
            (WeaponAnimation::ProneBack, 2)
        }
    } else if input.x > 0. {
        (WeaponAnimation::ProneRight, 3)
    } else {
        (WeaponAnimation::ProneLeft, 4)
    }
}

fn update(
    time: Res<Time<Fixed>>,
    game: Res<GameDefinition>,
    input: Res<GameInput>,
    mut actions: ResMut<TickActions>,
    mut players: Query<(
        &Player,
        &Vitality,
        &Mobility,
        &crate::movement::SprintState,
        &WeaponInventory,
        &crate::equipment::GrenadeInventory,
        &crate::equipment::TacticalInventory,
        &crate::mines::MineInventory,
        Option<&crate::perks::PerkDrinking>,
        Has<crate::actions::HandReservation>,
        &mut WeaponLocomotion,
    )>,
) {
    let Ok((
        player,
        life,
        mobility,
        sprint,
        inventory,
        lethal,
        tactical,
        mines,
        drinking,
        reserved,
        mut state,
    )) = players.single_mut()
    else {
        return;
    };
    let Some(weapon) = inventory.definition(&game.content.weapons) else {
        *state = default();
        return;
    };
    if life.state() != LifeState::Standing
        || inventory.action != WeaponAction::Ready && !actions.sprint
        || inventory.signal == WeaponSignal::Fire
        || lethal.busy()
        || tactical.busy()
        || mines.busy()
        || drinking.is_some()
        || reserved
        || actions.reload && !sprint.sprinting
        || actions.melee
        || actions.cycle_weapon
        || actions.weapon_slot.is_some()
    {
        *state = default();
        return;
    }
    if state.weapon.as_ref() != Some(&weapon.id) {
        *state = WeaponLocomotion {
            weapon: Some(weapon.id.clone()),
            ..default()
        };
    }
    state.elapsed += time.delta_secs();
    let diving = mobility.dive.is_some();
    let landed = mobility.dive.is_some_and(|d| d.slide_remaining.is_some());
    if diving && !state.diving {
        state.start(WeaponAnimation::DiveEnter, weapon.mechanics.dive_seconds[0]);
    }
    if landed && !state.landed {
        state.start(WeaponAnimation::DiveLeave, weapon.mechanics.dive_seconds[2]);
    } else if state.phase == Some(WeaponAnimation::DiveEnter) && state.elapsed >= state.duration {
        let elapsed = state.elapsed - state.duration;
        state.start(WeaponAnimation::DiveLoop, weapon.mechanics.dive_seconds[1]);
        state.elapsed = elapsed;
    } else if state.phase == Some(WeaponAnimation::DiveLoop) && !diving {
        state.start(WeaponAnimation::DiveLeave, weapon.mechanics.dive_seconds[2]);
    }
    state.diving = diving;
    state.landed = landed;
    if state.phase == Some(WeaponAnimation::DiveLeave) && state.elapsed >= state.duration {
        state.phase = None;
    }
    if matches!(
        state.phase,
        Some(WeaponAnimation::DiveEnter | WeaponAnimation::DiveLoop | WeaponAnimation::DiveLeave)
    ) {
        actions.fire = false;
        actions.fire_held = false;
        actions.secondary_fire = false;
        actions.secondary_fire_held = false;
        actions.ads = false;
        return;
    }
    if mobility.busy() {
        state.phase = None;
        return;
    }
    if sprint.sprinting != state.sprinting {
        state.sprinting = sprint.sprinting;
        if sprint.sprinting {
            state.start(
                WeaponAnimation::SprintEnter,
                weapon.mechanics.sprint_seconds[0],
            );
        } else if matches!(
            state.phase,
            Some(WeaponAnimation::SprintEnter | WeaponAnimation::SprintLoop)
        ) {
            state.start(
                WeaponAnimation::SprintLeave,
                weapon.mechanics.sprint_seconds[2],
            );
        }
    }
    if state.phase == Some(WeaponAnimation::SprintEnter) && state.elapsed >= state.duration {
        let elapsed = state.elapsed - state.duration;
        state.start(
            WeaponAnimation::SprintLoop,
            weapon.mechanics.sprint_seconds[1],
        );
        state.elapsed = elapsed;
    }
    if state.phase == Some(WeaponAnimation::SprintLeave) {
        state.queued_fire[0] |= actions.fire;
        state.queued_fire[1] |= actions.secondary_fire;
        if state.elapsed >= state.duration {
            actions.fire |= state.queued_fire[0] && input.fire_held;
            actions.secondary_fire |= state.queued_fire[1] && actions.secondary_fire_held;
            state.phase = None;
            state.queued_fire = [false; 2];
        } else {
            actions.fire = false;
            actions.fire_held = false;
            actions.secondary_fire = false;
            actions.secondary_fire_held = false;
        }
    }
    if matches!(
        state.phase,
        Some(
            WeaponAnimation::SprintEnter
                | WeaponAnimation::SprintLoop
                | WeaponAnimation::SprintLeave
        )
    ) {
        return;
    }
    let firing = (actions.fire
        || actions.fire_held
        || actions.secondary_fire
        || actions.secondary_fire_held)
        && inventory
            .active()
            .is_some_and(|w| w.magazine > 0 || w.left_magazine.is_some_and(|n| n > 0));
    let crawling = player.prone && input.movement != Vec2::ZERO && !actions.ads && !firing;
    if state.phase == Some(WeaponAnimation::ProneLeave) {
        state.queued_fire[0] |= actions.fire;
        state.queued_fire[1] |= actions.secondary_fire;
        if state.elapsed < state.duration {
            actions.fire = false;
            actions.fire_held = false;
            actions.secondary_fire = false;
            actions.secondary_fire_held = false;
            actions.ads = false;
            return;
        }
        actions.fire |= state.queued_fire[0] && input.fire_held;
        actions.secondary_fire |= state.queued_fire[1] && actions.secondary_fire_held;
        state.phase = None;
        state.queued_fire = [false; 2];
    }
    if crawling {
        let (phase, index) = direction(input.movement);
        if state.phase.is_none() {
            state.start(
                WeaponAnimation::ProneEnter,
                weapon.mechanics.crawl_seconds[0],
            );
        }
        if state.phase == Some(WeaponAnimation::ProneEnter) {
            if state.elapsed >= state.duration {
                let elapsed = state.elapsed - state.duration;
                state.start(phase, weapon.mechanics.crawl_seconds[index]);
                state.elapsed = elapsed;
            }
        } else if state.phase != Some(phase) {
            state.start(phase, weapon.mechanics.crawl_seconds[index]);
        }
    } else if state.phase.is_some() {
        state.queued_fire = [actions.fire, actions.secondary_fire];
        state.start(
            WeaponAnimation::ProneLeave,
            weapon.mechanics.crawl_seconds[if firing { 6 } else { 5 }],
        );
    }
    if state.phase.is_some() {
        actions.fire = false;
        actions.fire_held = false;
        actions.secondary_fire = false;
        actions.secondary_fire_held = false;
        actions.ads = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sprint_exit_blocks_fire_until_the_native_clock_finishes_and_requires_a_held_button() {
        use bevy::ecs::system::RunSystemOnce;
        for (held, secondary) in [(false, false), (true, false), (false, true), (true, true)] {
            let catalog: serde_json::Value =
                serde_json::from_str(include_str!("../../gameplay/origins/weapons.json")).unwrap();
            let weapons = catalog["weapons"].as_array().unwrap();
            let weapon: WeaponDefinition = serde_json::from_value(
                weapons
                    .iter()
                    .find(|w| {
                        w["id"]
                            == if secondary {
                                "weapon.fivesevendw_zm"
                            } else {
                                "weapon.c96_zm"
                            }
                    })
                    .unwrap()
                    .clone(),
            )
            .unwrap();
            let grenade: crate::equipment::ThrowableDefinition = serde_json::from_value(
                weapons
                    .iter()
                    .find(|w| w["id"] == "weapon.frag_grenade_zm")
                    .unwrap()["throwable"]
                    .clone(),
            )
            .unwrap();
            let mut game: GameDefinition =
                serde_json::from_str(include_str!("../../gameplay/origins/game.json")).unwrap();
            game.content.weapons = vec![weapon.clone()];
            if let Some(partner) = &weapon.dual_weapon {
                game.content.weapons.push(
                    serde_json::from_value(
                        weapons
                            .iter()
                            .find(|w| w["id"] == partner.0)
                            .unwrap()
                            .clone(),
                    )
                    .unwrap(),
                );
            }
            let mut inventory = WeaponInventory::default();
            inventory.grant(&weapon).unwrap();
            let mut world = World::new();
            world.insert_resource(game);
            world.insert_resource(GameInput::default());
            world.insert_resource(TickActions::default());
            let mut time = Time::<Fixed>::default();
            time.advance_by(std::time::Duration::from_secs_f32(0.05));
            world.insert_resource(time);
            let mut sprint = crate::movement::SprintState::new(4.);
            sprint.sprinting = true;
            let owner = world
                .spawn((
                    Player {
                        yaw: 0.,
                        pitch: 0.,
                        velocity: Vec3::ZERO,
                        height: 1.8,
                        grounded: true,
                        crouched: false,
                        prone: false,
                        eye_height: 1.6,
                    },
                    Vitality::human(
                        100,
                        100,
                        1,
                        crate::life::PlayerLethalPolicy::SoloOrLastEligiblePlayer,
                    )
                    .unwrap(),
                    Mobility::default(),
                    sprint,
                    inventory,
                    WeaponLocomotion::default(),
                    crate::equipment::GrenadeInventory::new(
                        WeaponId("weapon.frag_grenade_zm".into()),
                        &grenade,
                    ),
                ))
                .id();
            for _ in 0..6 {
                world.run_system_once(update).unwrap();
            }
            assert_eq!(
                world.get::<WeaponLocomotion>(owner).unwrap().phase,
                Some(WeaponAnimation::SprintLoop)
            );
            world
                .get_mut::<crate::movement::SprintState>(owner)
                .unwrap()
                .sprinting = false;
            world.resource_mut::<GameInput>().fire_held = !secondary;
            *world.resource_mut::<TickActions>() = TickActions {
                fire: !secondary,
                fire_held: !secondary,
                secondary_fire: secondary,
                secondary_fire_held: secondary,
                ..default()
            };
            world.run_system_once(update).unwrap();
            assert!(!world.resource::<TickActions>().fire);
            assert!(!world.resource::<TickActions>().fire_held);
            assert!(!world.resource::<TickActions>().secondary_fire);
            assert!(!world.resource::<TickActions>().secondary_fire_held);
            world.resource_mut::<GameInput>().fire_held = held && !secondary;
            for _ in 0..8 {
                *world.resource_mut::<TickActions>() = TickActions {
                    fire_held: held && !secondary,
                    secondary_fire_held: held && secondary,
                    ..default()
                };
                world.run_system_once(update).unwrap();
                if world
                    .get::<WeaponLocomotion>(owner)
                    .unwrap()
                    .phase
                    .is_none()
                {
                    break;
                }
                assert!(!world.resource::<TickActions>().fire_held);
                assert!(!world.resource::<TickActions>().secondary_fire_held);
            }
            assert!(
                world
                    .get::<WeaponLocomotion>(owner)
                    .unwrap()
                    .phase
                    .is_none()
            );
            let actions = world.resource::<TickActions>();
            assert_eq!(actions.fire, held && !secondary);
            assert_eq!(actions.secondary_fire, held && secondary);
        }
    }
    #[test]
    fn firing_crawl_exit_keeps_its_selected_native_duration() {
        let mut weapon: WeaponDefinition = serde_json::from_value(serde_json::json!({
            "id":"weapon.test", "name":"test", "damage":20, "category":"bullet", "magazine":8,
            "reserve":0, "fire_interval":0.1, "reload_seconds":1.
        }))
        .unwrap();
        weapon.animation_actions.insert(
            WeaponAnimation::ProneLeave,
            crate::origins::ContentId("animation.exit".into()),
        );
        weapon.mechanics.crawl_seconds[5] = 0.4;
        let mut state = WeaponLocomotion::default();
        state.start(WeaponAnimation::ProneLeave, 0.15);
        assert_eq!(state.sample(&weapon, false).unwrap().2, 0.15);
    }
}
