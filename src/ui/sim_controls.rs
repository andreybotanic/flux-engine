use bevy::prelude::*;

use crate::editor::MainMenuState;
use crate::plugins::PluginRuntimeEvent;
use crate::save::WorldLoadState;
use crate::simulation::{SimulationControl, SimulationSpeed};
use crate::ui::palette;

const PANEL_BG: Color = palette::PANEL_BG;
const BUTTON_IDLE: Color = palette::BUTTON_IDLE;
const BUTTON_ACTIVE: Color = palette::BUTTON_ACTIVE;
const BUTTON_PAUSED: Color = palette::BUTTON_PAUSED;
const LABEL_COLOR: Color = palette::TEXT_PRIMARY;

#[derive(Component, Clone, Copy)]
pub(crate) enum SimControlAction {
    Speed(SimulationSpeed),
    Pause,
}

#[derive(Component)]
pub(crate) struct SimPauseLabel;

#[derive(Component)]
pub(crate) struct SimControlRoot;

fn apply_pause_toggle(control: &mut SimulationControl, world_load_state: &WorldLoadState) {
    if world_load_state.has_world {
        control.paused = !control.paused;
    } else {
        control.paused = true;
    }
}

/// Runs `setup_sim_control_ui` logic.
pub(crate) fn setup_sim_control_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Visibility::Hidden,
            SimControlRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(52.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    SimControlAction::Speed(SimulationSpeed::X1),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("x1"),
                        TextFont::from_font_size(14.0),
                        TextColor(LABEL_COLOR),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(52.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    SimControlAction::Speed(SimulationSpeed::X2),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("x2"),
                        TextFont::from_font_size(14.0),
                        TextColor(LABEL_COLOR),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(52.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    SimControlAction::Speed(SimulationSpeed::X5),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("x5"),
                        TextFont::from_font_size(14.0),
                        TextColor(LABEL_COLOR),
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        min_width: Val::Px(84.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    SimControlAction::Pause,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Pause"),
                        TextFont::from_font_size(14.0),
                        TextColor(LABEL_COLOR),
                        SimPauseLabel,
                    ));
                });
        });
}

/// Runs `handle_sim_control_keyboard` logic.
pub(crate) fn handle_sim_control_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut control: ResMut<SimulationControl>,
    world_load_state: Res<WorldLoadState>,
    main_menu: Option<Res<MainMenuState>>,
    mut plugin_events: EventWriter<PluginRuntimeEvent>,
) {
    if main_menu.as_ref().map(|menu| menu.open).unwrap_or(false) {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        let was_paused = control.paused;
        apply_pause_toggle(&mut control, &world_load_state);
        if was_paused != control.paused {
            plugin_events.write(PluginRuntimeEvent::SimulationPausedChanged {
                paused: control.paused,
            });
        }
    }
    if keys.just_pressed(KeyCode::Period) {
        control.speed = control.speed.faster();
    }
    if keys.just_pressed(KeyCode::Comma) {
        control.speed = control.speed.slower();
    }
}

/// Runs `handle_sim_control_buttons` logic.
pub(crate) fn handle_sim_control_buttons(
    mut interactions: Query<
        (&Interaction, &SimControlAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut control: ResMut<SimulationControl>,
    world_load_state: Res<WorldLoadState>,
    main_menu: Option<Res<MainMenuState>>,
    mut plugin_events: EventWriter<PluginRuntimeEvent>,
) {
    if main_menu.as_ref().map(|menu| menu.open).unwrap_or(false) {
        return;
    }
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *action {
            SimControlAction::Speed(speed) => control.speed = speed,
            SimControlAction::Pause => {
                let was_paused = control.paused;
                apply_pause_toggle(&mut control, &world_load_state);
                if was_paused != control.paused {
                    plugin_events.write(PluginRuntimeEvent::SimulationPausedChanged {
                        paused: control.paused,
                    });
                }
            }
        }
    }
}

/// Runs `refresh_sim_control_visibility` logic.
pub(crate) fn refresh_sim_control_visibility(
    world_load_state: Res<WorldLoadState>,
    mut root_visibility: Single<&mut Visibility, With<SimControlRoot>>,
) {
    **root_visibility = if world_load_state.has_world {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

/// Runs `refresh_sim_control_ui` logic.
pub(crate) fn refresh_sim_control_ui(
    control: Res<SimulationControl>,
    mut button_query: Query<(&SimControlAction, &mut BackgroundColor), With<Button>>,
    mut pause_label: Single<&mut Text, With<SimPauseLabel>>,
) {
    for (action, mut bg) in &mut button_query {
        match *action {
            SimControlAction::Speed(speed) if speed == control.speed => {
                bg.0 = BUTTON_ACTIVE;
            }
            SimControlAction::Pause if control.paused => {
                bg.0 = BUTTON_PAUSED;
            }
            _ => {
                bg.0 = BUTTON_IDLE;
            }
        }
    }

    pause_label.0 = if control.paused {
        "Resume".to_string()
    } else {
        "Pause".to_string()
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_toggle_is_blocked_when_world_is_not_loaded() {
        let mut control = SimulationControl {
            paused: false,
            speed: SimulationSpeed::X1,
        };
        let world_state = WorldLoadState { has_world: false };
        apply_pause_toggle(&mut control, &world_state);
        assert!(control.paused);
    }

    #[test]
    fn pause_toggle_works_when_world_is_loaded() {
        let mut control = SimulationControl {
            paused: true,
            speed: SimulationSpeed::X1,
        };
        let world_state = WorldLoadState { has_world: true };
        apply_pause_toggle(&mut control, &world_state);
        assert!(!control.paused);
    }
}
