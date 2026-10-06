//! Bounded input-driven wall-buy coverage in the actual game and original map.
use super::Verification;
use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::Session,
    input::GameInput,
    interactions::{InteractionFocus, InteractionRejection, Usable},
    player::Player,
    weapons::{WeaponAction, WeaponInventory},
};
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
};

const FAMILIES: [&str; 4] = [
    "weapon.m14_zm",
    "weapon.mp40_zm",
    "weapon.870mcs_zm",
    "weapon.beretta93r_zm",
];
#[derive(Resource, Default)]
struct Coverage {
    family: usize,
    phase: u8,
    elapsed: f32,
    pressed: bool,
    rejected: bool,
    accepted: u32,
    magazine: u32,
    slot: usize,
    steps: Vec<serde_json::Value>,
    isolated: bool,
    balance: u32,
    ammo_price: u32,
    attempted_busy_switch: bool,
    approach_cursor: usize,
    second_shot: bool,
    reload_pending_ammo: Option<u32>,
    shell_start: bool,
    shell_loop: bool,
    shell_end: bool,
    shell_abort_sent: bool,
    shell_restarted: bool,
    shell_interrupted: bool,
    burst_started: bool,
}
pub fn install(app: &mut App) {
    app.init_resource::<Coverage>().add_systems(
        PreUpdate,
        drive.after(super::drive).before(crate::player::look),
    );
}
fn drive(
    mut commands: Commands,
    verification: Option<ResMut<Verification>>,
    mut coverage: ResMut<Coverage>,
    time: Res<Time>,
    definition: Option<Res<GameDefinition>>,
    mut input: ResMut<GameInput>,
    mut session: ResMut<Session>,
    focus: Res<InteractionFocus>,
    mut players: Query<(Entity, &mut Player, &mut Transform, &WeaponInventory)>,
    targets: Query<(&Usable, &Transform), Without<Player>>,
    query: SpatialQuery,
    mut next: ResMut<NextState<GameState>>,
    enemies: Query<Entity, With<crate::gameplay::Zombie>>,
) {
    let (Some(mut verification), Some(definition)) = (verification, definition) else {
        return;
    };
    if verification.stage != 4 {
        return;
    }
    if !coverage.isolated {
        // Normal combat already advanced the round. Isolate the subsequent
        // input-driven weapon segment from unrelated enemy attacks.
        for entity in &enemies {
            commands.entity(entity).despawn();
        }
        session.pending = 0;
        session.intermission = Some(Timer::from_seconds(300., TimerMode::Once));
        coverage.isolated = true;
    }
    let Ok((entity, mut player, mut body, inventory)) = players.single_mut() else {
        return;
    };
    input.movement = Vec2::ZERO;
    input.look = Vec2::ZERO;
    input.fire_held = false;
    input.use_held = false;
    input.ads = false;
    input.sprint = false;
    input.crouch = false;
    input.jump = false;
    input.melee = false;
    coverage.elapsed += time.delta_secs();
    if coverage.elapsed > 12. {
        let result = serde_json::json!({"passed": false, "family": FAMILIES[coverage.family], "phase": coverage.phase, "focus": focus.prompt, "rejection": format!("{:?}",focus.rejection), "action": format!("{:?}", inventory.action), "position": body.translation.to_array()});
        let _ = std::fs::write(
            verification.output.join("wall-weapons.json"),
            result.to_string(),
        );
        commands.write_message(AppExit::error());
        return;
    }
    if coverage.family == FAMILIES.len() {
        let result = serde_json::json!({"passed": true, "scope": "native wall locations and prices; controlled 100000 point balance after an insufficient-points rejection; enemy-free input-driven purchase, held firing, reload, direct/cyclic switching after normal round-one combat", "insufficient_points_rejected": coverage.rejected, "families": coverage.steps});
        let _ = std::fs::write(
            verification.output.join("wall-weapons.json"),
            result.to_string(),
        );
        verification.stage = 1;
        verification.playing = 0.;
        next.set(GameState::Paused);
        return;
    }
    let family = FAMILIES[coverage.family];
    let Some(weapon) = definition.content.weapons.iter().find(|w| w.id.0 == family) else {
        return;
    };
    if coverage.phase != 0 {
        if let Some((_, target)) = targets.iter().find(|(t,_)| matches!(&t.source.action, crate::origins::InteractionAction::Weapon { weapon, .. } if weapon.0 == family)) {
            let delta=target.translation-(body.translation+Vec3::Y*(definition.tuning.eye_height-player.height*0.5));
            player.yaw=(-delta.x).atan2(-delta.z)-inventory.recoil.x;
            player.pitch=delta.y.atan2(delta.xz().length())-inventory.recoil.y;
        }
    }
    match coverage.phase {
        0 => {
            let Some((usable, target)) = targets.iter().find(|(t,_)| matches!(&t.source.action, crate::origins::InteractionAction::Weapon { weapon, .. } if weapon.0 == family)) else { return; };
            if let crate::origins::InteractionAction::Weapon {
                price, ammo_price, ..
            } = usable.source.action
            {
                coverage.ammo_price = ammo_price.unwrap_or(price / 2);
            }
            let position = target.translation;
            let normal = target.rotation * Vec3::Z;
            let filter = SpatialQueryFilter::from_excluded_entities([entity]);
            let approach = [
                normal,
                -normal,
                Vec3::X,
                Vec3::NEG_X,
                Vec3::Z,
                Vec3::NEG_Z,
                Vec3::new(1., 0., 1.).normalize(),
                Vec3::new(-1., 0., 1.).normalize(),
                Vec3::new(1., 0., -1.).normalize(),
                Vec3::new(-1., 0., -1.).normalize(),
            ]
            .into_iter()
            .cycle()
            .skip(coverage.approach_cursor)
            .take(10)
            .find_map(|offset| {
                let probe = position + offset * 1.25;
                let floor =
                    query.cast_ray(probe + Vec3::Y * 1.5, Dir3::NEG_Y, 4., false, &filter)?;
                let center = probe + Vec3::Y * (1.5 - floor.distance + player.height * 0.5 + 0.03);
                let eye = center + Vec3::Y * (definition.tuning.eye_height - player.height * 0.5);
                let delta = position - eye;
                let distance = delta.length();
                if distance > 2.
                    || query
                        .cast_ray(
                            eye,
                            Dir3::new(delta).ok()?,
                            (distance - 0.12).max(0.),
                            true,
                            &filter,
                        )
                        .is_some()
                {
                    return None;
                }
                Some((center, delta))
            });
            if let Some((center, delta)) = approach {
                body.translation = center;
                player.velocity = Vec3::ZERO;
                player.yaw = (-delta.x).atan2(-delta.z);
                player.pitch = delta.y.atan2(delta.xz().length());
                coverage.accepted = focus.accepted;
                session.points = if coverage.family == 0 { 0 } else { 100000 };
                coverage.phase = 1;
                coverage.elapsed = 0.;
                coverage.pressed = false;
            }
        }
        1 if coverage.elapsed > 0.3 => {
            if coverage.elapsed > 0.8 && focus.target.is_none() && coverage.approach_cursor < 10 {
                coverage.approach_cursor += 1;
                coverage.phase = 0;
                coverage.elapsed = 0.;
                return;
            }
            input.use_held = true;
            if coverage.family == 0 && !coverage.rejected {
                if focus.rejection == Some(InteractionRejection::InsufficientPoints) {
                    coverage.rejected = true;
                    session.points = 100000;
                    input.use_held = false;
                    coverage.elapsed = 0.;
                }
            } else if focus.accepted > coverage.accepted {
                coverage.phase = 2;
                coverage.elapsed = 0.;
            }
        }
        2 if coverage.elapsed > 1. && inventory.action == WeaponAction::Ready => {
            if inventory.active().is_none_or(|w| w.definition != weapon.id) {
                return;
            }
            coverage.magazine = inventory.active().map_or(0, |w| w.magazine);
            coverage.slot = inventory.active_slot();
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(
                    verification
                        .output
                        .join(format!("wall-{}-equipped.png", coverage.family)),
                ));
            coverage.phase = 3;
            coverage.elapsed = 0.;
            coverage.pressed = false;
        }
        3 => {
            input.fire_held = true;
            input.fire |= !coverage.pressed;
            coverage.pressed = true;
            if weapon.mechanics.shell_reload
                && coverage.elapsed > weapon.fire_interval + 0.15
                && !coverage.second_shot
            {
                input.fire = true;
                coverage.second_shot = true;
            }
            if coverage.elapsed
                > if weapon.mechanics.shell_reload {
                    weapon.fire_interval + 0.4
                } else {
                    0.7
                }
            {
                input.fire_held = false;
                let magazine = inventory.active().map_or(0, |w| w.magazine);
                if magazine >= coverage.magazine {
                    return;
                }
                let step = serde_json::json!({"weapon":family,"magazine_before":coverage.magazine,"magazine_after_held_fire":magazine,"pattern":format!("{:?}",weapon.mechanics.fire),"pellets_per_shot":weapon.mechanics.pellets});
                coverage.steps.push(step);
                coverage.phase = 4;
                coverage.reload_pending_ammo = Some(magazine);
                coverage.elapsed = 0.;
                coverage.pressed = false;
            }
        }
        4 => {
            input.reload |= !coverage.pressed;
            coverage.pressed = true;
            if weapon.mechanics.shell_reload
                && !coverage.shell_abort_sent
                && coverage.elapsed > 0.3
                && matches!(
                    inventory.action,
                    WeaponAction::ShellReload {
                        phase: crate::weapons::ShellPhase::Start,
                        ..
                    }
                )
            {
                input.fire = true;
                coverage.shell_abort_sent = true;
            }
            if coverage.shell_abort_sent
                && !coverage.shell_restarted
                && coverage.elapsed > 1.3
                && inventory.action == WeaponAction::Ready
            {
                if inventory.active().map(|w| w.magazine)
                    != coverage.reload_pending_ammo.map(|m| m.saturating_sub(1))
                {
                    coverage.elapsed = 13.;
                    return;
                }
                coverage.shell_interrupted = true;
                coverage.shell_restarted = true;
                coverage.pressed = false;
            }
            if let WeaponAction::ShellReload { phase, .. } = inventory.action {
                match phase {
                    crate::weapons::ShellPhase::Start => coverage.shell_start = true,
                    crate::weapons::ShellPhase::Loop => coverage.shell_loop = true,
                    crate::weapons::ShellPhase::End => coverage.shell_end = true,
                }
            }
            if matches!(inventory.action, WeaponAction::Reloading { .. })
                && inventory.active().map(|w| w.magazine) != coverage.reload_pending_ammo
            {
                coverage.elapsed = 13.;
                return;
            }
            if coverage.elapsed > 0.3
                && matches!(inventory.action, WeaponAction::Reloading { .. })
                && !coverage.attempted_busy_switch
            {
                input.weapon_slot = Some(1 - coverage.slot);
                coverage.attempted_busy_switch = true;
            }
            if coverage.elapsed > 0.5
                && inventory.action == WeaponAction::Ready
                && inventory
                    .active()
                    .is_some_and(|w| w.magazine == weapon.magazine)
            {
                if inventory.active_slot() != coverage.slot {
                    return;
                }
                coverage.phase = 7;
                coverage.elapsed = 0.;
                coverage.pressed = false;
                coverage.accepted = focus.accepted;
                coverage.balance = session.points;
                let step = serde_json::json!({"weapon":family,"magazine_transfer_at_completion":!weapon.mechanics.shell_reload,"shell_start":coverage.shell_start,"shell_loop":coverage.shell_loop,"shell_end":coverage.shell_end,"shell_interrupted_and_resumed":coverage.shell_interrupted});
                coverage.steps.push(step);
            }
        }
        7 => {
            input.use_held = true;
            if coverage.elapsed > 0.6 {
                if focus.accepted != coverage.accepted + 1
                    || session.points != coverage.balance - coverage.ammo_price
                {
                    return;
                }
                let step = serde_json::json!({"weapon":family,"ammo_price":coverage.ammo_price,"points_before":coverage.balance,"points_after":session.points,"one_charge_while_held":true,"reserve_after_refill":inventory.reserve(weapon),"switch_blocked_during_reload":coverage.attempted_busy_switch});
                coverage.steps.push(step);
                coverage.phase = 5;
                coverage.elapsed = 0.;
                coverage.pressed = false;
            }
        }
        5 => {
            if matches!(
                weapon.mechanics.fire,
                crate::origins::FirePattern::Burst { .. }
            ) {
                if !coverage.burst_started {
                    input.fire = true;
                    coverage.burst_started = true;
                    return;
                }
                if inventory
                    .active()
                    .is_some_and(|w| w.magazine == weapon.magazine)
                {
                    return;
                }
            }
            if !coverage.pressed {
                input.weapon_slot = Some(1 - coverage.slot);
            }
            coverage.pressed = true;
            if coverage.elapsed > 0.2
                && inventory.active_slot() != coverage.slot
                && inventory.action == WeaponAction::Ready
            {
                if coverage.burst_started {
                    let rounds = inventory
                        .owned()
                        .find(|(slot, _)| *slot == coverage.slot)
                        .map(|(_, w)| w.magazine);
                    if rounds != Some(weapon.magazine - 1) {
                        coverage.elapsed = 13.;
                        return;
                    }
                    coverage.steps.push(
                        serde_json::json!({"weapon":family,"burst_interrupted_on_switch":true}),
                    );
                }
                coverage.phase = 6;
                coverage.elapsed = 0.;
                coverage.pressed = false;
            }
        }
        6 => {
            input.cycle_weapon |= !coverage.pressed;
            coverage.pressed = true;
            if coverage.elapsed > 0.2
                && inventory.active_slot() == coverage.slot
                && inventory.action == WeaponAction::Ready
            {
                coverage.family += 1;
                coverage.phase = 0;
                coverage.elapsed = 0.;
                coverage.attempted_busy_switch = false;
                coverage.approach_cursor = 0;
                coverage.second_shot = false;
                coverage.shell_start = false;
                coverage.shell_loop = false;
                coverage.shell_end = false;
                coverage.shell_abort_sent = false;
                coverage.shell_restarted = false;
                coverage.shell_interrupted = false;
                coverage.burst_started = false;
            }
        }
        _ => {}
    }
}
