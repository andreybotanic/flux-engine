use bevy::prelude::*;

use crate::simulation::{SimulationControl, SimulationSpeed};

const PANEL_BG: Color = Color::srgba(0.91, 0.92, 0.93, 0.96);
const BUTTON_IDLE: Color = Color::srgba(0.78, 0.80, 0.83, 0.95);
const BUTTON_ACTIVE: Color = Color::srgba(0.58, 0.68, 0.58, 0.96);
const BUTTON_PAUSED: Color = Color::srgba(0.80, 0.46, 0.44, 0.96);
const LABEL_COLOR: Color = Color::srgba(0.10, 0.10, 0.12, 1.0);

#[derive(Component, Clone, Copy)]
pub(crate) enum SimControlAction {
    Speed(SimulationSpeed),
    Pause,
}

#[derive(Component)]
pub(crate) struct SimPauseLabel;

pub fn setup_sim_control_ui(mut commands: Commands) {
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

pub fn handle_sim_control_keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    mut control: ResMut<SimulationControl>,
) {
    if keys.just_pressed(KeyCode::Space) {
        control.paused = !control.paused;
    }
    if keys.just_pressed(KeyCode::Period) {
        control.speed = control.speed.faster();
    }
    if keys.just_pressed(KeyCode::Comma) {
        control.speed = control.speed.slower();
    }
}

pub fn handle_sim_control_buttons(
    mut interactions: Query<
        (&Interaction, &SimControlAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut control: ResMut<SimulationControl>,
) {
    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *action {
            SimControlAction::Speed(speed) => control.speed = speed,
            SimControlAction::Pause => control.paused = !control.paused,
        }
    }
}

pub fn refresh_sim_control_ui(
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
