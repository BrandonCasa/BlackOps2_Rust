use super::{MenuFont, MenuItems, MenuRoot, menu_text, units};
use crate::{GameState, rendering::GameCamera};
use bevy::{
    anti_alias::dlss::neural_rendering::DlssNeuralRendering,
    prelude::*,
    ui::RelativeCursorPosition,
    ui_widgets::{
        Slider, SliderDragState, SliderPrecision, SliderRange, SliderStep, SliderThumb,
        SliderValue, TrackClick, ValueChange,
    },
};

#[derive(Component)]
pub(crate) struct Panel;

#[derive(Component, Clone, Copy)]
enum Field {
    Preset,
    Intensity,
    Style,
}

impl Field {
    fn label(self, value: f32) -> String {
        match self {
            Self::Preset => format!(
                "Preset: {}",
                ["Default", "1", "2", "3"][value.round() as usize]
            ),
            Self::Intensity => format!("Intensity: {:.0}%", value * 100.),
            Self::Style => format!(
                "Style: {}",
                ["Standard", "Natural", "Cinematic"][value.round() as usize]
            ),
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_observer(change).add_systems(
        Update,
        preview
            .after(super::rebuild)
            .run_if(in_state(GameState::Paused)),
    );
}

pub(super) fn panel(font: &MenuFont, nr: &DlssNeuralRendering) -> impl Bundle {
    (
        Panel,
        RelativeCursorPosition::default(),
        Node {
            position_type: PositionType::Absolute,
            right: units(32.),
            top: units(100.),
            width: units(300.),
            padding: UiRect::all(units(16.)),
            flex_direction: FlexDirection::Column,
            row_gap: units(14.),
            ..default()
        },
        BackgroundColor(Color::srgba(0.035, 0.035, 0.035, 0.94)),
        children![
            menu_text("NEURAL RENDERING", 28., font),
            control(Field::Preset, nr.preset as f32, 3., font),
            control(Field::Intensity, nr.intensity, 1., font),
            control(Field::Style, nr.style as f32, 2., font),
        ],
    )
}

fn control(field: Field, value: f32, maximum: f32, font: &MenuFont) -> impl Bundle {
    let value = value.clamp(0., maximum);
    let intensity = matches!(field, Field::Intensity);
    (
        field,
        Slider {
            track_click: TrackClick::Snap,
            ..default()
        },
        SliderValue(value),
        SliderRange::new(0., maximum),
        SliderStep(if intensity { 0.01 } else { 1. }),
        SliderPrecision(if intensity { 2 } else { 0 }),
        Node {
            height: units(56.),
            flex_shrink: 0.,
            ..default()
        },
        children![
            menu_text(field.label(value), 24., font),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    right: px(0.),
                    top: units(38.),
                    height: units(4.),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.3, 0.3, 0.3)),
                Pickable::IGNORE,
            ),
            (
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    right: units(12.),
                    top: units(32.),
                    height: units(16.),
                    ..default()
                },
                Pickable::IGNORE,
                children![(
                    SliderThumb,
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(value / maximum * 100.),
                        width: units(12.),
                        height: percent(100.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.95, 0.4, 0.05)),
                )],
            ),
        ],
    )
}

fn change(
    event: On<ValueChange<f32>>,
    controls: Query<(&Field, &SliderRange, &SliderDragState)>,
    children: Query<&Children>,
    mut texts: Query<&mut Text>,
    mut thumbs: Query<&mut Node, With<SliderThumb>>,
    mut cameras: Query<&mut DlssNeuralRendering, With<GameCamera>>,
    mut commands: Commands,
) {
    let Ok((field, range, drag)) = controls.get(event.source) else {
        return;
    };
    let value = range.clamp(event.value);
    commands.entity(event.source).insert(SliderValue(value));
    for child in children.iter_descendants(event.source) {
        if let Ok(mut text) = texts.get_mut(child) {
            text.0 = field.label(value);
        }
        if let Ok(mut thumb) = thumbs.get_mut(child) {
            thumb.left = percent(range.thumb_position(value) * 100.);
        }
    }
    // Apply clicks immediately and dragged values on release, avoiding model recreation per motion.
    if (event.is_final || !drag.dragging)
        && let Ok(mut nr) = cameras.single_mut()
    {
        match field {
            Field::Preset => nr.preset = value.round() as u32,
            Field::Intensity => nr.intensity = value,
            Field::Style => nr.style = value.round() as u32,
        }
        nr.reset = true;
    }
}

fn preview(
    panels: Query<&RelativeCursorPosition, With<Panel>>,
    mut roots: Query<&mut BackgroundColor, With<MenuRoot>>,
    mut items: Query<&mut Visibility, With<MenuItems>>,
) {
    let preview = panels.iter().any(|cursor| cursor.cursor_over());
    let color = if preview {
        Color::NONE
    } else {
        Color::linear_rgba(0., 0., 0., 0.62)
    };
    for mut background in &mut roots {
        if background.0 != color {
            background.0 = color;
        }
    }
    let visible = if preview {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut visibility in &mut items {
        if *visibility != visible {
            *visibility = visible;
        }
    }
}
