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
        aim_offset: Option<[f32; 3]>,
        arrival_point: Option<[f32; 3]>,
        arrival_radius: Option<f32>,
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
                aim_offset,
                arrival_point,
                arrival_radius,
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
                    || arrival_point.is_some_and(|p| !p.iter().all(|n| n.is_finite()))
                {
                    return Err("Input coordinates must be finite".into());
                }
                if aim.is_some() && aim_entity.is_some() {
                    return Err("Choose one aiming target".into());
                }
                if aim_offset.is_some_and(|offset| {
                    aim_entity.is_none() || !offset.iter().all(|n| n.is_finite())
                }) {
                    return Err("Aiming offsets require an entity and finite coordinates".into());
                }
                if arrival_radius.is_some_and(|radius| {
                    !radius.is_finite()
                        || radius <= 0.
                        || aim.is_none() && aim_entity.is_none() && arrival_point.is_none()
                }) {
                    return Err(
                        "Arrival requires a finite positive radius and a destination".into(),
                    );
                }
                if arrival_point.is_some() && arrival_radius.is_none() {
                    return Err("An arrival point requires a radius".into());
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Phase {
    Accepted,
    Executed,
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
    pub resulting_session: Option<u64>,
    pub error: Option<String>,
    pub after: Value,
}
