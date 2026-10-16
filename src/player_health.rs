use crate::{
    GameState,
    assets::GameDefinition,
    gameplay::GameStep,
    life::{LifeState, Vitality},
    player::Player,
};
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct HealthProfile {
    pub maximum: u32,
    pub regular_delay: f32,
    pub regular_step_fraction: f32,
    pub critical_delay: f32,
    pub critical_ratio: f32,
    pub critical_step_seconds: f32,
    pub critical_step_fraction: f32,
    pub worthy_damage_ratio: f32,
    pub immunity_regular: f32,
    pub immunity_critical_enter: f32,
    pub immunity_critical_repeat: f32,
    pub feedback_seconds: f32,
}
#[derive(Component)]
pub struct Recovery {
    pub age: f32,
    pub immunity: f32,
    pub critical: bool,
    pub feedback: f32,
    just_hurt: bool,
    step: f32,
    last_immune_ratio: f32,
    ignored_attacker: Option<(Entity, f32)>,
}
impl Default for Recovery {
    fn default() -> Self {
        Self {
            age: 0.,
            immunity: 0.,
            critical: false,
            feedback: 0.,
            just_hurt: false,
            step: 0.,
            last_immune_ratio: 1.,
            ignored_attacker: None,
        }
    }
}
impl Recovery {
    pub(crate) fn accepts_swipe(&mut self, attacker: Entity) -> bool {
        if self
            .ignored_attacker
            .is_some_and(|(ignored, _)| ignored == attacker)
        {
            return false;
        }
        // _zm::remove_ignore_attacker is separate from the health invulnerability window.
        self.ignored_attacker = Some((attacker, 0.4));
        true
    }

    pub fn hit(&mut self, before: u32, after: u32, maximum: u32, profile: &HealthProfile) {
        let ratio = after as f32 / maximum as f32;
        let entered = !self.critical && ratio <= profile.critical_ratio;
        self.critical |= ratio <= profile.critical_ratio;
        self.age = 0.;
        self.step = 0.;
        self.just_hurt = true;
        self.feedback = ((before - after) as f32 / maximum as f32).clamp(0., 1.);
        if after <= 1 || self.last_immune_ratio - ratio > profile.worthy_damage_ratio {
            self.immunity = if entered {
                profile.immunity_critical_enter
            } else if self.critical {
                profile.immunity_critical_repeat
            } else {
                profile.immunity_regular
            };
            self.last_immune_ratio = ratio;
        }
    }
}
#[derive(Message, Clone)]
pub struct PlayerDamage {
    pub impact: crate::damage::Impact,
    pub before: u32,
    pub after: u32,
    pub terminal: bool,
}
#[derive(Message, Clone)]
pub struct PlayerDeath {
    pub target: Entity,
    pub attacker: Entity,
}
#[derive(Resource, Default)]
pub struct HealthEvidence {
    pub nonterminal_hits: u32,
    pub immunity_rejections: u32,
    pub terminal_rejections: u32,
    pub death_events: u32,
    pub feedback_events: u32,
    pub hits: Vec<serde_json::Value>,
    pub heals: Vec<serde_json::Value>,
}
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct HealthUpdate;
pub struct PlayerHealthPlugin;
impl Plugin for PlayerHealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerDamage>()
            .add_message::<PlayerDeath>()
            .init_resource::<HealthEvidence>()
            .add_systems(
                FixedUpdate,
                (feedback, regenerate, terminal)
                    .chain()
                    .in_set(GameStep::PlayerLife)
                    .in_set(HealthUpdate)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
fn feedback(
    mut events: MessageReader<PlayerDamage>,
    recoveries: Query<&Recovery>,
    mut evidence: ResMut<HealthEvidence>,
) {
    for damage in events.read() {
        evidence.feedback_events += 1;
        if !damage.terminal {
            evidence.nonterminal_hits += 1;
        }
        if evidence.hits.len() < 128 {
            evidence.hits.push(serde_json::json!({"target":damage.impact.target.to_bits(),
                "attacker":damage.impact.attacker.to_bits(),"source":damage.impact.source.id(),
                "damage":damage.impact.amount,"before":damage.before,"after":damage.after,"terminal":damage.terminal,
                "immunity_seconds":recoveries.get(damage.impact.target).ok().map(|r|r.immunity),
                "critical":recoveries.get(damage.impact.target).ok().map(|r|r.critical)}));
        }
    }
}
fn regenerate(
    time: Res<Time<Fixed>>,
    definition: Res<GameDefinition>,
    mut players: Query<(Entity, &mut Vitality, &mut Recovery), With<Player>>,
    mut evidence: ResMut<HealthEvidence>,
) {
    let profile = &definition.player_health;
    for (entity, mut health, mut recovery) in &mut players {
        if health.state() != LifeState::Standing {
            continue;
        }
        if recovery.just_hurt {
            recovery.just_hurt = false;
            continue;
        }
        let dt = time.delta_secs();
        recovery.age += dt;
        recovery.immunity = (recovery.immunity - dt).max(0.);
        if let Some((_, remaining)) = &mut recovery.ignored_attacker {
            *remaining -= dt;
            if *remaining <= 0.000001 {
                recovery.ignored_attacker = None;
            }
        }
        if health.current() == Some(health.maximum()) {
            recovery.critical = false;
            recovery.last_immune_ratio = 1.;
            recovery.step = 0.;
            continue;
        }
        recovery.step += dt;
        // _zm_playerhealth polls at 20 Hz, including the one-health floor repair.
        let polls = ((recovery.step + 0.000001) / profile.critical_step_seconds) as u32;
        recovery.step = (recovery.step - polls as f32 * profile.critical_step_seconds).max(0.);
        for poll in 0..polls {
            let age = recovery.age
                - recovery.step
                - (polls - poll - 1) as f32 * profile.critical_step_seconds;
            if health.current() == Some(1) {
                if health.regenerate(1).is_ok_and(|healed| healed > 0) {
                    recovery.last_immune_ratio = 2. / health.maximum() as f32;
                    if evidence.heals.len() < 128 {
                        evidence.heals.push(serde_json::json!({"target":entity.to_bits(),"before":1,"after":health.current(),
                        "since_hit":recovery.age,"critical":recovery.critical,"kind":"damage_floor","state":format!("{:?}",health.state())}));
                    }
                }
            }
            let delay = if recovery.critical {
                profile.critical_delay
            } else {
                profile.regular_delay
            };
            if age + 0.000001 < delay || (recovery.critical && age <= delay + 0.000001) {
                continue;
            }
            let before = health.current().unwrap_or(0);
            let amount = if recovery.critical {
                ((health.maximum() as f32 * profile.critical_step_fraction) as u32).max(1)
            } else {
                ((health.maximum() as f32 * profile.regular_step_fraction) as u32).max(1)
            };
            if health.regenerate(amount).is_ok_and(|healed| healed > 0)
                && evidence.heals.len() < 128
            {
                evidence.heals.push(serde_json::json!({"target":entity.to_bits(),"before":before,"after":health.current(),
                "since_hit":recovery.age,"critical":recovery.critical,"delay":delay,"state":format!("{:?}",health.state())}));
            }
        }
    }
}
fn terminal(
    mut events: MessageReader<PlayerDeath>,
    mut next: ResMut<NextState<GameState>>,
    mut evidence: ResMut<HealthEvidence>,
) {
    for _ in events.read() {
        evidence.death_events += 1;
        next.set(GameState::GameOver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use std::time::Duration;

    fn damaged(current: u32) -> (World, Entity) {
        let mut world = World::new();
        let game: GameDefinition =
            serde_json::from_str(include_str!("../gameplay/origins/game.json")).unwrap();
        let mut recovery = Recovery::default();
        recovery.hit(100, current, 100, &game.player_health);
        recovery.just_hurt = false;
        world.insert_resource(game);
        world.init_resource::<Time<Fixed>>();
        world.init_resource::<HealthEvidence>();
        let entity = world
            .spawn((
                Player {
                    yaw: 0.,
                    pitch: 0.,
                    velocity: Vec3::ZERO,
                    height: 1.8,
                    grounded: true,
                    crouched: false,
                    prone: false,
                    eye_height: 1.62,
                },
                Vitality::human(
                    current,
                    100,
                    1,
                    crate::life::PlayerLethalPolicy::SoloSelfRevive,
                )
                .unwrap(),
                recovery,
            ))
            .id();
        (world, entity)
    }

    fn advance(world: &mut World, seconds: f32) {
        world
            .resource_mut::<Time<Fixed>>()
            .advance_by(Duration::from_secs_f32(seconds));
        world.run_system_once(regenerate).unwrap();
    }

    #[test]
    fn native_health_poll_preserves_regular_and_critical_recovery_deadlines() {
        let (mut world, entity) = damaged(50);
        advance(&mut world, 2.35);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(50));
        advance(&mut world, 0.05);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(100));

        let (mut world, entity) = damaged(20);
        advance(&mut world, 5.);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(20));
        advance(&mut world, 0.05);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(30));
        advance(&mut world, 0.35);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(100));
    }

    #[test]
    fn native_swipe_lock_outlasts_regular_immunity_and_tracks_the_last_attacker() {
        let (mut world, entity) = damaged(50);
        let attacker = world.spawn_empty().id();
        let other = world.spawn_empty().id();
        assert!(
            world
                .get_mut::<Recovery>(entity)
                .unwrap()
                .accepts_swipe(attacker)
        );
        advance(&mut world, 0.36);
        let mut recovery = world.get_mut::<Recovery>(entity).unwrap();
        assert_eq!(recovery.immunity, 0.);
        assert!(!recovery.accepts_swipe(attacker));
        assert!(recovery.accepts_swipe(other));
        assert!(recovery.accepts_swipe(attacker));
        drop(recovery);
        advance(&mut world, 0.4);
        assert!(
            world
                .get_mut::<Recovery>(entity)
                .unwrap()
                .accepts_swipe(attacker)
        );
    }

    #[test]
    fn native_one_health_floor_repairs_on_the_next_script_poll() {
        let (mut world, entity) = damaged(1);
        advance(&mut world, 0.025);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(1));
        advance(&mut world, 0.025);
        assert_eq!(world.get::<Vitality>(entity).unwrap().current(), Some(2));
    }
}
