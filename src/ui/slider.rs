use bevy::{prelude::*, ui::RelativeCursorPosition};
use std::collections::HashMap;

const SLIDER_TRACK_BG: Color = crate::ui::palette::SELECT_BG;
const SLIDER_KNOB_BG: Color = crate::ui::palette::MENU_MODAL_BUTTON_BG;

const DEFAULT_TRACK_WIDTH: f32 = 260.0;
const TRACK_HEIGHT: f32 = 12.0;
const KNOB_WIDTH: f32 = 10.0;
const KNOB_HEIGHT: f32 = 26.0;

/// Stable identifier of one slider field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SliderId(&'static str);

impl SliderId {
    /// Creates one slider identifier from a static key.
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }
}

/// Configures one slider widget.
#[derive(Clone, Copy, Debug)]
pub struct SliderConfig {
    pub id: SliderId,
    pub min_value: i32,
    pub max_value: i32,
    pub step: i32,
    pub value: i32,
    pub track_width: f32,
}

impl SliderConfig {
    /// Creates one slider config using the default track width.
    pub fn with_default_width(
        id: SliderId,
        min_value: i32,
        max_value: i32,
        step: i32,
        value: i32,
    ) -> Self {
        Self {
            id,
            min_value,
            max_value,
            step,
            value,
            track_width: DEFAULT_TRACK_WIDTH,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct SliderEntry {
    min_value: i32,
    max_value: i32,
    step: i32,
    value: i32,
    track_width: f32,
}

#[derive(Resource, Default)]
/// Runtime slider state store.
pub struct SliderState {
    entries: HashMap<SliderId, SliderEntry>,
    active_drag: Option<SliderId>,
}

impl SliderState {
    /// Registers or replaces one slider entry.
    pub fn register_slider(&mut self, config: SliderConfig) {
        if config.min_value >= config.max_value {
            return;
        }
        if config.step <= 0 {
            return;
        }
        let mut entry = SliderEntry {
            min_value: config.min_value,
            max_value: config.max_value,
            step: config.step,
            value: config.value,
            track_width: config.track_width.max(1.0),
        };
        entry.value = quantize_value(entry.value, entry.min_value, entry.max_value, entry.step);
        self.entries.insert(config.id, entry);
    }

    /// Returns the current value of one slider.
    pub fn value(&self, id: SliderId) -> Option<i32> {
        self.entries.get(&id).map(|entry| entry.value)
    }

    /// Sets one slider value using clamp and step quantization.
    pub fn set_value(&mut self, id: SliderId, value: i32) -> Option<i32> {
        let entry = self.entries.get_mut(&id)?;
        let next = quantize_value(value, entry.min_value, entry.max_value, entry.step);
        if entry.value == next {
            return None;
        }
        entry.value = next;
        Some(next)
    }
}

/// Reports one slider value update.
#[derive(Event, Clone, Copy, Debug)]
pub struct SliderValueChanged {
    pub id: SliderId,
    pub value: i32,
}

#[derive(Component, Clone, Copy)]
struct SliderTrack {
    id: SliderId,
}

#[derive(Component, Clone, Copy)]
struct SliderTrackBar {
    id: SliderId,
}

#[derive(Component, Clone, Copy)]
struct SliderKnob {
    id: SliderId,
}

/// Spawns one reusable slider track with fill and knob visuals.
pub fn spawn_slider(parent: &mut ChildSpawnerCommands, config: SliderConfig) -> Entity {
    let track_width = config.track_width.max(1.0);
    let root_width = track_width + KNOB_WIDTH;
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(root_width),
                height: Val::Px(KNOB_HEIGHT),
                align_items: AlignItems::Center,
                position_type: PositionType::Relative,
                overflow: Overflow::visible(),
                ..default()
            },
            BackgroundColor(crate::ui::palette::TRANSPARENT),
            RelativeCursorPosition::default(),
            SliderTrack { id: config.id },
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Px(KNOB_WIDTH * 0.5),
                    top: Val::Px((KNOB_HEIGHT - TRACK_HEIGHT) * 0.5),
                    width: Val::Px(track_width),
                    height: Val::Px(TRACK_HEIGHT),
                    ..default()
                },
                BackgroundColor(SLIDER_TRACK_BG),
                SliderTrackBar { id: config.id },
            ));
            track.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(KNOB_WIDTH),
                    height: Val::Px(KNOB_HEIGHT),
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    ..default()
                },
                BackgroundColor(SLIDER_KNOB_BG),
                SliderKnob { id: config.id },
            ));
        })
        .id()
}

/// Stores `SliderPlugin` state.
pub struct SliderPlugin;

impl Plugin for SliderPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SliderState>()
            .add_event::<SliderValueChanged>()
            .add_systems(
                Update,
                (handle_slider_press, handle_slider_drag, sync_slider_visuals),
            );
    }
}

fn handle_slider_press(
    mut slider_state: ResMut<SliderState>,
    tracks: Query<
        (&Interaction, &SliderTrack, &RelativeCursorPosition),
        (Changed<Interaction>, With<Button>),
    >,
    mut changed: EventWriter<SliderValueChanged>,
) {
    for (interaction, track, cursor) in &tracks {
        if *interaction != Interaction::Pressed {
            continue;
        }
        slider_state.active_drag = Some(track.id);
        let Some(entry) = slider_state.entries.get(&track.id).copied() else {
            continue;
        };
        let Some(normalized) = cursor.normalized else {
            continue;
        };
        if let Some(value) = slider_state.set_value(
            track.id,
            value_from_track_cursor(
                normalized.x,
                entry.track_width,
                entry.min_value,
                entry.max_value,
                entry.step,
            ),
        ) {
            changed.write(SliderValueChanged {
                id: track.id,
                value,
            });
        }
    }
}

fn handle_slider_drag(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut slider_state: ResMut<SliderState>,
    tracks: Query<(&SliderTrack, &RelativeCursorPosition), With<Button>>,
    mut changed: EventWriter<SliderValueChanged>,
) {
    if mouse_buttons.just_released(MouseButton::Left) || !mouse_buttons.pressed(MouseButton::Left) {
        slider_state.active_drag = None;
        return;
    }

    let Some(active_id) = slider_state.active_drag else {
        return;
    };
    let Some(entry) = slider_state.entries.get(&active_id).copied() else {
        return;
    };

    for (track, cursor) in &tracks {
        if track.id != active_id {
            continue;
        }
        let Some(normalized) = cursor.normalized else {
            return;
        };
        if let Some(value) = slider_state.set_value(
            active_id,
            value_from_track_cursor(
                normalized.x,
                entry.track_width,
                entry.min_value,
                entry.max_value,
                entry.step,
            ),
        ) {
            changed.write(SliderValueChanged {
                id: active_id,
                value,
            });
        }
        return;
    }
}

fn sync_slider_visuals(
    slider_state: Res<SliderState>,
    mut bars: Query<(&SliderTrackBar, &mut BackgroundColor)>,
    mut knobs: Query<(&SliderKnob, &mut Node)>,
) {
    for (bar, mut background) in &mut bars {
        let Some(_entry) = slider_state.entries.get(&bar.id) else {
            continue;
        };
        background.0 = SLIDER_TRACK_BG;
    }

    for (knob, mut node) in &mut knobs {
        let Some(entry) = slider_state.entries.get(&knob.id) else {
            continue;
        };
        let normalized = normalized_from_value(entry.value, entry.min_value, entry.max_value);
        let center_x = entry.track_width * normalized;
        node.left = Val::Px(center_x);
    }
}

fn value_from_track_cursor(
    root_normalized_x: f32,
    track_width: f32,
    min_value: i32,
    max_value: i32,
    step: i32,
) -> i32 {
    let root_width = track_width + KNOB_WIDTH;
    if root_width <= f32::EPSILON {
        return min_value;
    }
    let root_px = root_normalized_x.clamp(0.0, 1.0) * root_width;
    let track_normalized = ((root_px - KNOB_WIDTH * 0.5) / track_width).clamp(0.0, 1.0);
    value_from_normalized(track_normalized, min_value, max_value, step)
}

fn quantize_value(value: i32, min_value: i32, max_value: i32, step: i32) -> i32 {
    if min_value >= max_value {
        return min_value;
    }
    let clamped = value.clamp(min_value, max_value);
    let offset = clamped - min_value;
    let steps = ((offset as f32) / (step as f32)).round() as i32;
    (min_value + steps * step).clamp(min_value, max_value)
}

fn value_from_normalized(normalized_x: f32, min_value: i32, max_value: i32, step: i32) -> i32 {
    let norm = normalized_x.clamp(0.0, 1.0);
    let span = (max_value - min_value) as f32;
    let raw = min_value as f32 + span * norm;
    quantize_value(raw.round() as i32, min_value, max_value, step)
}

fn normalized_from_value(value: i32, min_value: i32, max_value: i32) -> f32 {
    if max_value <= min_value {
        return 0.0;
    }
    ((value - min_value) as f32 / (max_value - min_value) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod slider_tests {
    use super::{quantize_value, value_from_normalized};

    #[test]
    fn quantize_value_respects_step_and_bounds() {
        assert_eq!(quantize_value(0, 0, 100, 1), 0);
        assert_eq!(quantize_value(17, 0, 100, 5), 15);
        assert_eq!(quantize_value(18, 0, 100, 5), 20);
        assert_eq!(quantize_value(101, 0, 100, 1), 100);
    }

    #[test]
    fn value_from_normalized_maps_to_range() {
        assert_eq!(value_from_normalized(0.0, 0, 100, 1), 0);
        assert_eq!(value_from_normalized(0.5, 0, 100, 1), 50);
        assert_eq!(value_from_normalized(1.0, 0, 100, 1), 100);
    }
}
