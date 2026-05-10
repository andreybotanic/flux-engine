use bevy::prelude::*;

/// Runtime configuration for one reusable two-position toggle switch.
#[derive(Clone, Debug)]
pub struct ToggleSwitchConfig {
    pub on: bool,
    pub interactive: bool,
    pub label: String,
}

impl ToggleSwitchConfig {
    /// Creates a toggle switch config with an optional text label.
    pub fn new(on: bool, interactive: bool, label: impl Into<String>) -> Self {
        Self {
            on,
            interactive,
            label: label.into(),
        }
    }
}

/// Color palette used by the standard toggle switch.
#[derive(Clone, Copy, Debug)]
pub struct ToggleSwitchPalette {
    pub root_bg: Color,
    pub track_on: Color,
    pub track_off: Color,
    pub track_disabled: Color,
    pub knob: Color,
    pub text: Color,
    pub text_disabled: Color,
}

impl Default for ToggleSwitchPalette {
    fn default() -> Self {
        Self {
            root_bg: Color::NONE,
            track_on: Color::srgba(0.33, 0.62, 0.38, 1.0),
            track_off: Color::srgba(0.54, 0.56, 0.59, 1.0),
            track_disabled: Color::srgba(0.70, 0.72, 0.75, 1.0),
            knob: Color::srgba(0.96, 0.97, 0.98, 1.0),
            text: crate::ui::palette::TEXT_SECONDARY,
            text_disabled: crate::ui::palette::TEXT_MUTED,
        }
    }
}

/// Marks the root node of a standard toggle switch.
#[derive(Component, Clone, Debug)]
pub struct ToggleSwitchRoot {
    pub on: bool,
    pub interactive: bool,
    pub label: String,
}

/// Marks the visual track of a standard toggle switch.
#[derive(Component)]
pub struct ToggleSwitchTrack;

/// Marks the visual knob of a standard toggle switch.
#[derive(Component)]
pub struct ToggleSwitchKnob;

/// Marks the optional text label of a standard toggle switch.
#[derive(Component)]
pub struct ToggleSwitchLabel;

/// Spawns a read-only standard toggle switch.
pub fn spawn_toggle_switch(
    parent: &mut ChildSpawnerCommands,
    config: ToggleSwitchConfig,
) -> Entity {
    let palette = ToggleSwitchPalette::default();
    parent
        .spawn(toggle_switch_root_bundle(&config, palette))
        .with_children(|switch| spawn_toggle_switch_children(switch, &config, palette))
        .id()
}

/// Spawns an interactive standard toggle switch with caller-provided action components.
pub fn spawn_toggle_switch_button(
    parent: &mut ChildSpawnerCommands,
    config: ToggleSwitchConfig,
    action_bundle: impl Bundle,
) -> Entity {
    let palette = ToggleSwitchPalette::default();
    parent
        .spawn((
            Button,
            toggle_switch_root_bundle(&config, palette),
            action_bundle,
        ))
        .with_children(|switch| spawn_toggle_switch_children(switch, &config, palette))
        .id()
}

fn toggle_switch_root_bundle(
    config: &ToggleSwitchConfig,
    palette: ToggleSwitchPalette,
) -> (Node, BackgroundColor, ToggleSwitchRoot) {
    (
        Node {
            width: Val::Px(154.0),
            height: Val::Px(38.0),
            display: Display::Flex,
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
            column_gap: Val::Px(8.0),
            ..default()
        },
        BackgroundColor(palette.root_bg),
        ToggleSwitchRoot {
            on: config.on,
            interactive: config.interactive,
            label: config.label.clone(),
        },
    )
}

fn spawn_toggle_switch_children(
    parent: &mut ChildSpawnerCommands,
    config: &ToggleSwitchConfig,
    palette: ToggleSwitchPalette,
) {
    let track_bg = if !config.interactive {
        palette.track_disabled
    } else if config.on {
        palette.track_on
    } else {
        palette.track_off
    };
    let knob_alignment = if config.on {
        JustifyContent::FlexEnd
    } else {
        JustifyContent::FlexStart
    };
    parent
        .spawn((
            Node {
                width: Val::Px(56.0),
                height: Val::Px(28.0),
                padding: UiRect::all(Val::Px(3.0)),
                justify_content: knob_alignment,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(track_bg),
            ToggleSwitchTrack,
        ))
        .with_children(|track| {
            track.spawn((
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(22.0),
                    ..default()
                },
                BackgroundColor(palette.knob),
                ToggleSwitchKnob,
            ));
        });

    if !config.label.is_empty() {
        parent.spawn((
            Text::new(config.label.clone()),
            TextFont::from_font_size(13.0),
            TextColor(if config.interactive {
                palette.text
            } else {
                palette.text_disabled
            }),
            ToggleSwitchLabel,
        ));
    }
}

fn refresh_toggle_switch_visuals(
    roots: Query<(&ToggleSwitchRoot, &Children), Changed<ToggleSwitchRoot>>,
    mut tracks: Query<(&mut Node, &mut BackgroundColor), With<ToggleSwitchTrack>>,
    mut labels: Query<(&mut Text, &mut TextColor), With<ToggleSwitchLabel>>,
) {
    let palette = ToggleSwitchPalette::default();
    for (root, children) in &roots {
        let track_bg = if !root.interactive {
            palette.track_disabled
        } else if root.on {
            palette.track_on
        } else {
            palette.track_off
        };
        let knob_alignment = if root.on {
            JustifyContent::FlexEnd
        } else {
            JustifyContent::FlexStart
        };
        for child in children.iter() {
            if let Ok((mut node, mut background)) = tracks.get_mut(child) {
                node.justify_content = knob_alignment;
                background.0 = track_bg;
            }
            if let Ok((mut text, mut color)) = labels.get_mut(child) {
                text.0 = root.label.clone();
                color.0 = if root.interactive {
                    palette.text
                } else {
                    palette.text_disabled
                };
            }
        }
    }
}

/// Stores `ToggleSwitchPlugin` state.
pub struct ToggleSwitchPlugin;

impl Plugin for ToggleSwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, refresh_toggle_switch_visuals);
    }
}

#[cfg(test)]
mod tests {
    use super::ToggleSwitchConfig;

    #[test]
    fn toggle_switch_config_keeps_visual_state() {
        let config = ToggleSwitchConfig::new(true, false, "Locked");
        assert!(config.on);
        assert!(!config.interactive);
        assert_eq!(config.label, "Locked");
    }
}
