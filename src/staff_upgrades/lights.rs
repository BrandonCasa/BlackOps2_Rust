use super::{Disc, StaffUpgrades};
use crate::{GameState, gameplay::GameStep, quests::MainQuest};
use bevy::prelude::*;
use std::collections::VecDeque;

const KEY: &str = "INFERNO";
const MESSAGES: [&str; 8] = [
    "WARN MESSINES",
    "SOMETHING BLUE IN THE EARTH",
    "NOT CLAY",
    "WE GREW WEAK",
    "THOUGHT IT WAS FLU",
    "MEN BECAME BEASTS",
    "BLOOD TURNED TO ASH",
    "LIBERATE TUTE DE INFERNIS",
];
const MORSE: [&str; 26] = [
    ".-", "-...", "-.-.", "-..", ".", "..-.", "--.", "....", "..", ".---", "-.-", ".-..", "--",
    "-.", "---", ".--.", "--.-", ".-.", "...", "-", "..-", "...-", ".--", "-..-", "-.--", "--..",
];

#[derive(Clone, serde::Serialize)]
pub(crate) struct LightStep {
    pub mode: u8,
    pub on: bool,
    pub seconds: f64,
    pub message: String,
}

#[derive(Clone, serde::Serialize)]
pub(crate) struct LightEvent {
    pub sequence: u64,
    pub elapsed: f64,
    pub mode: u8,
    pub on: bool,
    pub message: String,
}

#[derive(Resource, Default)]
pub(crate) struct LightShow {
    pub active: bool,
    pub mode: u8,
    pub on: bool,
    pub elapsed: f64,
    pub cycles: u32,
    pub events: VecDeque<LightEvent>,
    pub sequence: u64,
    remaining: f64,
    steps: VecDeque<LightStep>,
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct LightPresentation;

pub(super) fn install(app: &mut App) {
    app.init_resource::<LightShow>()
        .add_systems(
            OnEnter(GameState::StartingSession),
            reset.in_set(crate::session::SessionSetup::World),
        )
        .add_systems(
            FixedUpdate,
            tick.in_set(GameStep::Rewards)
                .after(super::progress)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(Update, present.in_set(LightPresentation));
}

fn reset(mut show: ResMut<LightShow>) {
    *show = default();
}

fn cipher(phrase: &str) -> String {
    let mut index = 0;
    phrase
        .bytes()
        .map(|letter| {
            if letter.is_ascii_uppercase() {
                let offset = KEY.as_bytes()[index % KEY.len()] - b'A';
                index += 1;
                (b'A' + (letter - b'A' + offset) % 26) as char
            } else {
                letter as char
            }
        })
        .collect()
}

fn message(steps: &mut VecDeque<LightStep>, phrase: String, mode: u8) {
    for letter in phrase.bytes() {
        if letter.is_ascii_uppercase() {
            for symbol in MORSE[(letter - b'A') as usize].bytes() {
                steps.push_back(LightStep {
                    mode,
                    on: true,
                    seconds: if symbol == b'.' { 0.2 } else { 1. },
                    message: phrase.clone(),
                });
                steps.push_back(LightStep {
                    mode,
                    on: false,
                    seconds: 0.5,
                    message: phrase.clone(),
                });
            }
        } else {
            steps.push_back(LightStep {
                mode,
                on: false,
                seconds: 2.,
                message: phrase.clone(),
            });
        }
        steps.push_back(LightStep {
            mode,
            on: false,
            seconds: 1.5,
            message: phrase.clone(),
        });
    }
}

pub(crate) fn cycle(bellaso: bool) -> VecDeque<LightStep> {
    let mut steps = VecDeque::new();
    if bellaso {
        steps.push_back(LightStep {
            mode: 1,
            on: false,
            seconds: 10.,
            message: String::new(),
        });
        message(&mut steps, "GIOVAN BATTISTA BELLASO".into(), 3);
    }
    steps.push_back(LightStep {
        mode: 1,
        on: false,
        seconds: 10.,
        message: String::new(),
    });
    message(&mut steps, KEY.into(), 2);
    for phrase in MESSAGES {
        steps.push_back(LightStep {
            mode: 1,
            on: false,
            seconds: 10.,
            message: String::new(),
        });
        message(&mut steps, cipher(phrase), 1);
    }
    steps
}

fn tick(
    mut commands: Commands,
    time: Res<Time<Fixed>>,
    quest: Res<MainQuest>,
    mut session: ResMut<crate::gameplay::Session>,
    mut show: ResMut<LightShow>,
    discs: Query<Entity, With<Disc>>,
) {
    if !show.active {
        if quest.upgraded.len() != 4 {
            return;
        }
        show.active = true;
    } else {
        show.remaining -= time.delta_secs_f64();
        show.elapsed += time.delta_secs_f64();
    }
    while show.remaining <= 0. {
        if show.steps.is_empty() {
            show.steps = cycle(session.random() % 100 < 10);
            show.cycles += 1;
        }
        let step = show.steps.pop_front().unwrap();
        let changed = show.mode != step.mode || show.on != step.on;
        show.mode = step.mode;
        show.on = step.on;
        show.sequence += 1;
        let event = LightEvent {
            sequence: show.sequence,
            elapsed: show.elapsed + show.remaining,
            mode: step.mode,
            on: step.on,
            message: step.message,
        };
        if show.events.len() == 256 {
            show.events.pop_front();
        }
        show.events.push_back(event);
        show.remaining += step.seconds;
        if changed {
            for entity in &discs {
                crate::audio::emit(
                    &mut commands,
                    "zmb_crypt_disc_light",
                    crate::audio::SoundSource::attached(entity),
                );
            }
        }
    }
}

fn present(
    mut commands: Commands,
    show: Res<LightShow>,
    upgrades: Res<StaffUpgrades>,
    mut discs: Query<(
        Entity,
        &Disc,
        Option<&mut crate::presentation::cues::CueParameters>,
    )>,
) {
    for (entity, disc, inputs) in &mut discs {
        if !show.is_changed() && !upgrades.is_changed() && inputs.is_some() {
            continue;
        }
        let color = if show.active {
            match (show.mode, show.on) {
                (2, true) => Vec3::new(2., 0.1, 0.1),
                (2, false) => Vec3::new(0.5, 0.1, 0.1),
                (3, true) => Vec3::new(0.1, 2., 0.1),
                (3, false) => Vec3::new(0.1, 0.5, 0.1),
                (_, true) => Vec3::splat(2.),
                (_, false) => Vec3::splat(0.25),
            }
        } else {
            [
                Vec3::new(0.1, 0.1, 0.5),
                Vec3::new(0.5, 0.1, 0.5),
                Vec3::new(0.5, 0.1, 0.1),
                Vec3::new(0.5, 0.5, 0.1),
            ][upgrades.rings[disc.0] as usize]
        };
        // zm_tomb.csc maps client slot0 to ScriptVector3 and submits RGB plus
        // alpha1. PS88e8a7be consumes it before premultiplication and squaring.
        let vector = color.extend(1.).to_array();
        if let Some(mut inputs) = inputs {
            if inputs.script_vectors[3] != vector || inputs.script_available & 8 == 0 {
                inputs.script_vectors[3] = vector;
                inputs.script_available |= 8;
            }
        } else {
            let mut inputs = crate::presentation::cues::CueParameters::default();
            inputs.script_vectors[3] = vector;
            inputs.script_available = 8;
            commands.entity(entity).insert(inputs);
        }
    }
}
