use super::{Mobility, MotionFeedback, SprintState};
use crate::{assets::SOURCE_UNIT_METERS, origins::content::WeaponViewMotion, player::Player};
use bevy::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

// Native 008ebcc0 / 008eda80: one lateral cycle and two shaped vertical steps.
pub(super) fn bob(phase: f32, amplitude: Vec2) -> Vec2 {
    Vec2::new(
        phase.sin(),
        ((phase * 2.).sin() + (phase * 4. + FRAC_PI_2).sin() * 0.2) * 0.75,
    ) * amplitude
}

impl MotionFeedback {
    pub(crate) fn advance_bob(
        &mut self,
        player: &Player,
        sprint: &SprintState,
        mobility: &Mobility,
        weapon: &WeaponViewMotion,
        dt: f32,
    ) -> bool {
        self.previous_bob = self.bob;
        self.previous_cycle = self.cycle;
        let speed = player.velocity.xz().length() / SOURCE_UNIT_METERS;
        let diving = mobility.dive.is_some();
        let moving = speed >= 2. && (player.grounded || diving);
        if moving {
            // 008c3180 / 008c0660 and native cycle table 00bd049c.
            let cycle = if diving {
                0.24 * weapon.dive_cycle
            } else if sprint.sprinting && speed >= 185. {
                0.5 * weapon.sprint_cycle
            } else if player.prone {
                0.25
            } else if player.crouched {
                0.34
            } else {
                0.335
            };
            self.cycle = self.cycle.wrapping_add((dt * 1000. * cycle) as u8);
        }
        // 008c0020 / 008c00c0 emit the two contacts at byte cycles 64 and 192.
        let step = moving
            && (self.cycle ^ self.previous_cycle) & 0xc0 != 0
            && self.cycle & 0x7f > 63
            && speed >= 10.;
        let amplitude = if diving {
            Vec2::splat(0.002)
        } else if sprint.sprinting {
            Vec2::new(0.02, 0.014)
        } else if player.prone {
            Vec2::new(0.08, 0.04)
        } else if player.crouched {
            Vec2::splat(0.0075)
        } else {
            Vec2::splat(0.007)
        };
        self.bob = bob(
            self.cycle as f32 * TAU / 255.,
            (amplitude * speed).min(Vec2::splat(8.)),
        ) * SOURCE_UNIT_METERS;
        if !player.grounded && !diving {
            self.bob = Vec2::ZERO;
        }
        step && player.grounded && !player.prone && !mobility.busy()
    }

    pub(crate) fn weapon_pose(
        &mut self,
        player: &Player,
        sprint: &SprintState,
        mobility: &Mobility,
        weapon: &WeaponViewMotion,
        ads: f32,
        dt: f32,
        interpolation: f32,
    ) -> Transform {
        let look = Vec2::new(-player.pitch.to_degrees(), player.yaw.to_degrees());
        if dt > 0. {
            let delta = self.look.map_or(Vec2::ZERO, |previous| {
                let delta = look - previous;
                Vec2::new(wrap(delta.x), wrap(delta.y)) / (dt * 60.)
            });
            if self.look.is_none() {
                self.idle_factor = 1.;
            }
            self.look = Some(look);
            // 008ecca0: normalize turn speed to 60 Hz, clamp, then apply weapon scales.
            let sway: [f32; 6] =
                std::array::from_fn(|i| weapon.sway[i].lerp(weapon.ads_sway[i], ads));
            let delta = delta.clamp(Vec2::splat(-sway[0]), Vec2::splat(sway[0]));
            let blend = (sway[1] * dt).min(1.);
            self.sway = self.sway.lerp(delta * Vec2::new(sway[2], sway[3]), blend);
            self.sway_position = self
                .sway_position
                .lerp(delta * Vec2::new(sway[5], sway[4]), blend);
            // 008e0930 / 008ebf80: the idle clock continues across action changes.
            let speed = weapon.idle_speed[0].lerp(weapon.idle_speed[1], ads);
            self.idle_phase += Vec3::new(1., 0.7, 0.5) * (dt * speed);
            self.idle_phase = Vec3::new(
                self.idle_phase.x.rem_euclid(TAU),
                self.idle_phase.y.rem_euclid(TAU),
                self.idle_phase.z.rem_euclid(TAU),
            );
            let factor = if player.prone {
                weapon.idle_stance[1]
            } else if player.crouched {
                weapon.idle_stance[0]
            } else {
                1.
            };
            self.idle_factor += (factor - self.idle_factor).clamp(-dt * 0.5, dt * 0.5);
        }
        let amount =
            weapon.idle_amount[0].lerp(weapon.idle_amount[1], ads) * self.idle_factor * 0.01;
        let idle = Vec3::new(
            self.idle_phase.x.sin(),
            self.idle_phase.y.sin(),
            self.idle_phase.z.sin(),
        ) * amount;
        let end = self.cycle as f32
            + if self.cycle < self.previous_cycle {
                255.
            } else {
                0.
            };
        let view_phase = (self.previous_cycle as f32).lerp(end, interpolation) * TAU / 255.;
        let phase = view_phase + PI * 0.25;
        let speed = player.velocity.xz().length() / SOURCE_UNIT_METERS * 0.16;
        let amplitude = if mobility.dive.is_some() {
            Vec2::splat(0.002) * Vec2::from_array(weapon.dive_bob)
        } else if sprint.sprinting {
            Vec2::new(0.02, 0.014) * Vec2::from_array(weapon.sprint_bob)
        } else if player.prone {
            Vec2::new(0.02, 0.005)
        } else if player.crouched {
            Vec2::new(0.045, 0.025) * (1. - ads)
        } else {
            Vec2::new(0.055, 0.025) * (1. - ads)
        };
        let amplitude = (amplitude * speed).min(Vec2::splat(8.));
        let weapon_bob = bob(phase, amplitude);
        let bob_scale = 1. - (1. - weapon.ads_bob) * ads;
        let roll = ((phase - 0.47123894).sin() * amplitude.x * 1.5).min(0.);
        let view_fraction = if weapon.scoped_ads { ads } else { 0. };
        let mut angles = idle * (1. - view_fraction) - Vec3::new(self.sway.x, self.sway.y, 0.);
        angles += Vec3::new(-weapon_bob.y, -weapon_bob.x, roll) * bob_scale * (1. - view_fraction);
        // Scoped ADS idle belongs to the view, with the model following the aim axis.
        let view_bob = bob(view_phase, Vec2::splat(0.007 * speed / 0.16));
        self.view_angles = (Vec2::new(idle.x, idle.y)
            - Vec2::new(view_bob.y, view_bob.x) * weapon.ads_view_bob)
            * view_fraction;
        Transform {
            translation: Vec3::new(self.sway_position.y, self.sway_position.x, 0.)
                * SOURCE_UNIT_METERS,
            rotation: Quat::from_euler(
                EulerRot::YXZ,
                angles.y.to_radians(),
                -angles.x.to_radians(),
                -angles.z.to_radians(),
            ),
            ..default()
        }
    }
}

pub(super) fn wrap(degrees: f32) -> f32 {
    (degrees + 180.).rem_euclid(360.) - 180.
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player() -> Player {
        Player {
            yaw: 0.,
            pitch: 0.,
            velocity: Vec3::ZERO,
            height: 1.8,
            grounded: true,
            crouched: false,
            prone: false,
            eye_height: 1.62,
        }
    }

    #[test]
    fn native_turn_sway_has_the_same_strength_at_different_render_rates() {
        let weapon = WeaponViewMotion {
            sway: [5., 6., -2., -1., 0.3, 0.3],
            ..default()
        };
        for fps in [30, 60, 144] {
            let dt = 1. / fps as f32;
            let mut player = player();
            let mut feedback = MotionFeedback::default();
            feedback.weapon_pose(
                &player,
                &SprintState::default(),
                &Mobility::default(),
                &weapon,
                0.,
                dt,
                0.,
            );
            for _ in 0..fps * 3 {
                player.yaw += 60_f32.to_radians() * dt;
                feedback.weapon_pose(
                    &player,
                    &SprintState::default(),
                    &Mobility::default(),
                    &weapon,
                    0.,
                    dt,
                    0.,
                );
            }
            assert!(
                (feedback.sway.y + 1.).abs() < 0.0001,
                "{fps} Hz: {:?}",
                feedback.sway
            );
            assert!((feedback.sway_position.y - 0.3).abs() < 0.0001);
        }
    }

    #[test]
    fn scoped_idle_moves_the_view_while_iron_sight_idle_stays_on_the_model() {
        let mut weapon = WeaponViewMotion {
            idle_amount: [15., 28.],
            idle_speed: [5., 1.],
            idle_stance: [0.8, 0.6],
            ..default()
        };
        let mut feedback = MotionFeedback::default();
        let pose = feedback.weapon_pose(
            &player(),
            &SprintState::default(),
            &Mobility::default(),
            &weapon,
            1.,
            0.1,
            0.,
        );
        assert_ne!(pose.rotation, Quat::IDENTITY);
        assert_eq!(feedback.view_angles, Vec2::ZERO);
        weapon.scoped_ads = true;
        let phase = feedback.idle_phase;
        let pose = feedback.weapon_pose(
            &player(),
            &SprintState::default(),
            &Mobility::default(),
            &weapon,
            1.,
            0.,
            0.,
        );
        assert_eq!(pose.rotation, Quat::IDENTITY);
        assert_ne!(feedback.view_angles, Vec2::ZERO);
        assert_eq!(feedback.idle_phase, phase);
    }

    #[test]
    fn native_bob_byte_clock_wraps_and_emits_only_the_two_foot_contacts() {
        let mut player = player();
        player.velocity = Vec3::NEG_Z * (190. * SOURCE_UNIT_METERS);
        let mut feedback = MotionFeedback::default();
        let mut contacts = 0;
        for _ in 0..60 {
            contacts += usize::from(feedback.advance_bob(
                &player,
                &SprintState::default(),
                &Mobility::default(),
                &WeaponViewMotion::default(),
                1. / 60.,
            ));
        }
        assert_eq!(feedback.cycle, 44);
        assert_eq!(contacts, 2);
        player.velocity = Vec3::ZERO;
        assert!(!feedback.advance_bob(
            &player,
            &SprintState::default(),
            &Mobility::default(),
            &WeaponViewMotion::default(),
            1. / 60.
        ));
        assert_eq!(feedback.cycle, 44);
        assert_eq!(feedback.bob, Vec2::ZERO);
    }
}
