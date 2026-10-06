//! Accepted player damage feedback and fixed-step recovery; never revives terminal actors.
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
#[serde(deny_unknown_fields)]
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
impl HealthProfile {
    pub fn valid(&self) -> bool {
        self.maximum > 0
            && self.maximum <= i32::MAX as u32
            && [
                self.regular_delay,
                self.regular_step_fraction,
                self.critical_delay,
                self.critical_ratio,
                self.critical_step_seconds,
                self.critical_step_fraction,
                self.worthy_damage_ratio,
                self.immunity_regular,
                self.immunity_critical_enter,
                self.immunity_critical_repeat,
                self.feedback_seconds,
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.)
            && self.critical_delay >= self.regular_delay
            && self.critical_ratio <= 1.
            && self.regular_step_fraction <= 1.
            && self.critical_step_fraction <= 1.
            && self.worthy_damage_ratio <= 1.
    }
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
        }
    }
}
impl Recovery {
    pub fn hit(&mut self, before: u32, after: u32, maximum: u32, profile: &HealthProfile) {
        let ratio = after as f32 / maximum as f32;
        let entered = !self.critical && ratio <= profile.critical_ratio;
        self.critical |= ratio <= profile.critical_ratio;
        self.age = 0.;
        self.step = 0.;
        self.just_hurt = true;
        self.feedback = ((before - after) as f32 / maximum as f32).clamp(0., 1.);
        if self.last_immune_ratio - ratio > profile.worthy_damage_ratio {
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
        if health.current() == Some(health.maximum()) {
            recovery.critical = false;
            recovery.last_immune_ratio = 1.;
            recovery.step = 0.;
            continue;
        }
        let delay = if recovery.critical {
            profile.critical_delay
        } else {
            profile.regular_delay
        };
        if recovery.age < delay {
            continue;
        }
        let before = health.current().unwrap_or(0);
        let amount = if recovery.critical {
            recovery.step += dt;
            if recovery.step < profile.critical_step_seconds {
                continue;
            }
            recovery.step -= profile.critical_step_seconds;
            ((health.maximum() as f32 * profile.critical_step_fraction) as u32).max(1)
        } else {
            ((health.maximum() as f32 * profile.regular_step_fraction) as u32).max(1)
        };
        if health.regenerate(amount).is_ok_and(|healed| healed > 0) && evidence.heals.len() < 128 {
            evidence.heals.push(serde_json::json!({"target":entity.to_bits(),"before":before,"after":health.current(),
                "since_hit":recovery.age,"critical":recovery.critical,"delay":delay,"state":format!("{:?}",health.state())}));
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
