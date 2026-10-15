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
    queued_fire: bool,
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
        );
        let duration = if phase == WeaponAnimation::ProneLeave {
            weapon.mechanics.crawl_seconds[5]
        } else {
            self.duration
        };
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
        || inventory.action != WeaponAction::Ready
        || inventory.signal == WeaponSignal::Fire
        || lethal.busy()
        || tactical.busy()
        || mines.busy()
        || drinking.is_some()
        || reserved
        || actions.reload
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
        actions.ads = false;
        return;
    }
    if mobility.busy() {
        state.phase = None;
        return;
    }
    let firing = (actions.fire || actions.fire_held)
        && inventory
            .active()
            .is_some_and(|w| w.magazine > 0 || w.left_magazine.is_some_and(|n| n > 0));
    let crawling = player.prone && input.movement != Vec2::ZERO && !actions.ads && !firing;
    if state.phase == Some(WeaponAnimation::ProneLeave) {
        state.queued_fire |= actions.fire;
        if state.elapsed < state.duration {
            actions.fire = false;
            actions.fire_held = false;
            actions.ads = false;
            return;
        }
        actions.fire |= state.queued_fire && input.fire_held;
        state.phase = None;
        state.queued_fire = false;
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
        state.queued_fire = actions.fire;
        state.start(
            WeaponAnimation::ProneLeave,
            weapon.mechanics.crawl_seconds[if firing { 6 } else { 5 }],
        );
    }
    if state.phase.is_some() {
        actions.fire = false;
        actions.fire_held = false;
        actions.ads = false;
    }
}
