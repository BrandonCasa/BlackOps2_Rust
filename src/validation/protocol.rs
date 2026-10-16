use crate::input::Action;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub session: u64,
    pub id: u64,
    pub action: Command,
    #[serde(default)]
    pub label: String,
    pub expect: Option<Expectation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Command {
    Start,
    Input {
        ticks: u32,
        #[serde(default)]
        held: Vec<Action>,
        #[serde(default)]
        look: [f32; 2],
        aim: Option<[f32; 3]>,
        aim_entity: Option<u64>,
    },
    Pause,
    Resume,
    Step,
    Cancel {
        command: u64,
    },
    Abort,
    Finish,
    Fail {
        reason: String,
    },
    Assist {
        god: bool,
        points: u32,
        ammo: bool,
        doors: bool,
    },
    Teleport {
        feet: [f32; 3],
    },
    Capture {
        name: String,
    },
    Quit,
}

impl Command {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Self::Input {
                ticks,
                held,
                look,
                aim,
                aim_entity,
            } => {
                if !(1..=1800).contains(ticks) {
                    return Err("Input must contain 1..1800 ticks".into());
                }
                if held
                    .iter()
                    .any(|action| matches!(action, Action::Pause | Action::Restart))
                {
                    return Err("Pause and restart use control commands".into());
                }
                if !look.iter().all(|n| n.is_finite())
                    || aim.is_some_and(|p| !p.iter().all(|n| n.is_finite()))
                {
                    return Err("Input coordinates must be finite".into());
                }
                if aim.is_some() && aim_entity.is_some() {
                    return Err("Choose one aiming target".into());
                }
            }
            Self::Teleport { feet } if !feet.iter().all(|n| n.is_finite()) => {
                return Err("Teleport coordinates must be finite".into());
            }
            Self::Capture { name }
                if name.is_empty()
                    || name.len() > 80
                    || !name
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_') =>
            {
                return Err(
                    "Capture name must contain only letters, numbers, hyphens, or underscores"
                        .into(),
                );
            }
            _ => {}
        }
        Ok(())
    }
}

/// Read-only assertions, never reflected world mutations.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Expectation {
    pub path: String,
    pub before: Value,
    pub equals: Option<Value>,
    pub increase: Option<u64>,
    #[serde(default = "default_deadline")]
    pub seconds: f64,
}
fn default_deadline() -> f64 {
    30.
}
impl Expectation {
    pub fn validate(&self) -> Result<(), String> {
        if !self.path.starts_with('/')
            || self.equals.is_some() == self.increase.is_some()
            || !self.seconds.is_finite()
            || self.seconds <= 0.
            || self.seconds > 600.
        {
            return Err("Expectation needs a JSON pointer, exactly one comparison, and a deadline in (0, 600] seconds".into());
        }
        if self.equals.as_ref() == Some(&self.before) {
            return Err("An unchanged initial value cannot prove an action".into());
        }
        if self.increase == Some(0) {
            return Err("An increase must be positive".into());
        }
        Ok(())
    }
    pub fn satisfied(&self, snapshot: &Value) -> bool {
        let Some(value) = snapshot.pointer(&self.path) else {
            return false;
        };
        if let Some(expected) = &self.equals {
            return value == expected;
        }
        value
            .as_u64()
            .zip(self.before.as_u64())
            .zip(self.increase)
            .is_some_and(|((after, before), increase)| after >= before.saturating_add(increase))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Accepted,
    Executed,
    OutcomeObserved,
    Rejected,
    Failed,
    Cancelled,
}

#[derive(Clone, Serialize)]
pub(super) struct Receipt {
    pub request: Request,
    pub phase: Phase,
    pub accepted_tick: u64,
    pub executed_tick: Option<u64>,
    pub observed_tick: Option<u64>,
    pub resulting_session: Option<u64>,
    pub error: Option<String>,
    pub before: Value,
    pub after: Value,
    #[serde(skip)]
    pub started: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preexisting_success_and_zero_increase_are_not_proof() {
        let mut e = Expectation {
            path: "/done".into(),
            before: json!(true),
            equals: Some(json!(true)),
            increase: None,
            seconds: 30.,
        };
        assert!(e.validate().is_err());
        e.equals = None;
        e.increase = Some(0);
        assert!(e.validate().is_err());
    }
    #[test]
    fn observation_checks_real_state_and_positive_delta() {
        let e = Expectation {
            path: "/kills".into(),
            before: json!(8),
            equals: None,
            increase: Some(1),
            seconds: 30.,
        };
        assert!(!e.satisfied(&json!({"kills":8})));
        assert!(e.satisfied(&json!({"kills":9})));
        assert!(!e.satisfied(&json!({"missing":9})));
    }
}
