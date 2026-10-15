use super::{Disc, StaffUpgrades};
use crate::{GameState, gameplay::GameStep, quests::MainQuest};
use bevy::{gltf::GltfMaterialName, prelude::*};
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

#[derive(Component)]
pub(crate) struct LightSurface {
    disc: Entity,
    pub color: Vec3,
    material: Option<Handle<StandardMaterial>>,
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
        .add_systems(Update, (bind, present).chain().in_set(LightPresentation));
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

fn bind(
    mut commands: Commands,
    discs: Query<Entity, With<Disc>>,
    children: Query<&Children>,
    names: Query<&GltfMaterialName, Without<LightSurface>>,
) {
    for disc in &discs {
        for entity in children.iter_descendants(disc) {
            if names
                .get(entity)
                .is_ok_and(|name| name.0.ends_with("mtl_p6_zm_tm_chamber_mid_piece_lt"))
            {
                commands.entity(entity).insert(LightSurface {
                    disc,
                    material: None,
                    color: Vec3::splat(-1.),
                });
            }
        }
    }
}

fn present(
    show: Res<LightShow>,
    upgrades: Res<StaffUpgrades>,
    discs: Query<&Disc>,
    mut surfaces: Query<(&mut LightSurface, &mut MeshMaterial3d<StandardMaterial>)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut surface, mut handle) in &mut surfaces {
        let Ok(disc) = discs.get(surface.disc) else {
            continue;
        };
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
        surface.color = color;
        if surface.material.is_none() {
            let Some(source) = materials.get(&handle.0).cloned() else {
                continue;
            };
            let target = materials.add(source);
            handle.0 = target.clone();
            surface.material = Some(target);
        }
        if let Some(mut material) = surface.material.as_ref().and_then(|h| materials.get_mut(h)) {
            material.emissive = LinearRgba::rgb(color.x, color.y, color.z);
        }
    }
}
