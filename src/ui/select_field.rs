use bevy::{prelude::*, window::PrimaryWindow};
use std::collections::HashMap;

use crate::ui::palette;
use crate::ui::panels::PanelManager;

const SELECT_BG: Color = palette::SELECT_BG;
const SELECT_BG_OPEN: Color = palette::SELECT_BG_OPEN;
const SELECT_OPTION_HOVER: Color = palette::SELECT_OPTION_HOVER;
const SELECT_OPTION_SELECTED: Color = palette::SELECT_OPTION_SELECTED;
const SELECT_BORDER: Color = palette::SELECT_BORDER;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Stores `SelectFieldId` state.
pub struct SelectFieldId(&'static str);

impl SelectFieldId {
    pub const fn new(id: &'static str) -> Self {
        Self(id)
    }
}

#[derive(Clone, Debug)]
/// Stores `SelectFieldConfig` state.
pub struct SelectFieldConfig {
    pub id: SelectFieldId,
    pub options: Vec<String>,
    pub selected: usize,
}

#[derive(Clone, Debug, Default)]
struct SelectFieldEntry {
    options: Vec<String>,
    selected: usize,
    open: bool,
}

#[derive(Resource, Default)]
/// Stores `SelectFieldState` state.
pub struct SelectFieldState {
    entries: HashMap<SelectFieldId, SelectFieldEntry>,
}

impl SelectFieldState {
    /// Runs `register_field` logic.
    pub fn register_field(&mut self, config: SelectFieldConfig) {
        let selected = if config.options.is_empty() {
            0
        } else {
            config.selected.min(config.options.len() - 1)
        };
        self.entries.insert(
            config.id,
            SelectFieldEntry {
                options: config.options,
                selected,
                open: false,
            },
        );
    }

    /// Runs `set_selected` logic.
    pub fn set_selected(&mut self, id: SelectFieldId, index: usize) {
        let Some(entry) = self.entries.get_mut(&id) else {
            return;
        };
        if entry.options.is_empty() {
            entry.selected = 0;
        } else {
            entry.selected = index.min(entry.options.len() - 1);
        }
    }

    /// Replaces the option list and preserves the selected label when possible.
    pub fn set_options_preserving_selection(&mut self, id: SelectFieldId, options: Vec<String>) {
        let Some(entry) = self.entries.get_mut(&id) else {
            return;
        };
        let previous_label = entry.options.get(entry.selected).cloned();
        let selected = previous_label
            .as_ref()
            .and_then(|label| options.iter().position(|option| option == label))
            .unwrap_or(0);
        entry.options = options;
        entry.selected = if entry.options.is_empty() {
            0
        } else {
            selected.min(entry.options.len() - 1)
        };
        entry.open = false;
    }

    /// Runs `selected_index` logic.
    pub fn selected_index(&self, id: SelectFieldId) -> Option<usize> {
        self.entries.get(&id).map(|entry| entry.selected)
    }

    /// Runs `selected_label` logic.
    pub fn selected_label(&self, id: SelectFieldId) -> Option<&str> {
        let entry = self.entries.get(&id)?;
        entry.options.get(entry.selected).map(|item| item.as_str())
    }

    /// Runs `is_open` logic.
    pub fn is_open(&self, id: SelectFieldId) -> bool {
        self.entries
            .get(&id)
            .map(|entry| entry.open)
            .unwrap_or(false)
    }

    /// Runs `close_all` logic.
    pub fn close_all(&mut self) {
        for entry in self.entries.values_mut() {
            entry.open = false;
        }
    }

    fn toggle_open(&mut self, id: SelectFieldId) {
        let was_open = self.is_open(id);
        self.close_all();
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.open = !was_open;
        }
    }

    fn any_open(&self) -> bool {
        self.entries.values().any(|entry| entry.open)
    }
}

#[derive(Component, Clone, Copy)]
/// Stores `SelectFieldLabel` state.
pub struct SelectFieldLabel {
    pub id: SelectFieldId,
}

#[derive(Component, Clone, Copy)]
/// Stores `SelectFieldArrow` state.
pub struct SelectFieldArrow {
    pub id: SelectFieldId,
}

#[derive(Component, Clone, Copy)]
/// Stores `SelectFieldOptionsRoot` state.
pub struct SelectFieldOptionsRoot {
    pub id: SelectFieldId,
}

#[derive(Component, Clone)]
struct SelectFieldRenderedOptions {
    options: Vec<String>,
}

#[derive(Component, Clone, Copy)]
struct SelectFieldToggleButton {
    id: SelectFieldId,
}

#[derive(Component, Clone, Copy)]
struct SelectFieldOptionButton {
    id: SelectFieldId,
    option_index: usize,
}

/// Runs `spawn_select_field` logic.
pub fn spawn_select_field(
    parent: &mut ChildSpawnerCommands,
    config: &SelectFieldConfig,
    caption: &str,
    select_arrow: Handle<Image>,
) {
    parent
        .spawn((Node {
            width: Val::Px(220.0),
            position_type: PositionType::Relative,
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(6.0),
            ..default()
        },))
        .with_children(|select| {
            select.spawn((
                Text::new(caption.to_string()),
                TextFont::from_font_size(13.0),
                TextColor(palette::TEXT_PRIMARY),
            ));

            let initial_label = config
                .options
                .get(config.selected)
                .cloned()
                .unwrap_or_else(|| "N/A".to_string());

            select
                .spawn((
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(32.0),
                        border: UiRect::all(Val::Px(1.0)),
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        justify_content: JustifyContent::SpaceBetween,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(SELECT_BG),
                    BorderColor(SELECT_BORDER),
                    SelectFieldToggleButton { id: config.id },
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(initial_label),
                        TextFont::from_font_size(13.0),
                        TextColor(palette::TEXT_PRIMARY),
                        SelectFieldLabel { id: config.id },
                    ));
                    button.spawn((
                        ImageNode::new(select_arrow),
                        Node {
                            width: Val::Px(10.0),
                            height: Val::Px(6.0),
                            ..default()
                        },
                        SelectFieldArrow { id: config.id },
                    ));
                });

            select
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(52.0),
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(0.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    BackgroundColor(SELECT_BORDER),
                    GlobalZIndex(1200),
                    SelectFieldOptionsRoot { id: config.id },
                    SelectFieldRenderedOptions {
                        options: config.options.clone(),
                    },
                ))
                .with_children(|options| {
                    spawn_option_buttons(options, config.id, &config.options);
                });
        });
}

fn spawn_option_buttons(parent: &mut ChildSpawnerCommands, id: SelectFieldId, options: &[String]) {
    for (option_index, option_label) in options.iter().enumerate() {
        parent
            .spawn((
                Button,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(28.0),
                    justify_content: JustifyContent::FlexStart,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                    ..default()
                },
                BackgroundColor(SELECT_BG),
                SelectFieldOptionButton { id, option_index },
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(option_label.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(palette::TEXT_PRIMARY),
                ));
            });
    }
}

/// Stores `SelectFieldPlugin` state.
pub struct SelectFieldPlugin;

impl Plugin for SelectFieldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectFieldState>().add_systems(
            Update,
            (
                handle_select_field_interactions,
                collapse_open_selects_on_panel_click,
                sync_select_field_visuals,
            ),
        );
    }
}

fn handle_select_field_interactions(
    mut state: ResMut<SelectFieldState>,
    toggle_buttons: Query<
        (&Interaction, &SelectFieldToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    option_buttons: Query<
        (&Interaction, &SelectFieldOptionButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, toggle) in &toggle_buttons {
        if *interaction == Interaction::Pressed {
            state.toggle_open(toggle.id);
        }
    }

    for (interaction, option) in &option_buttons {
        if *interaction == Interaction::Pressed {
            state.set_selected(option.id, option.option_index);
            if let Some(entry) = state.entries.get_mut(&option.id) {
                entry.open = false;
            }
        }
    }
}

fn collapse_open_selects_on_panel_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    panel_manager: Res<PanelManager>,
    mut state: ResMut<SelectFieldState>,
    toggle_buttons: Query<&Interaction, (With<Button>, With<SelectFieldToggleButton>)>,
    option_buttons: Query<&Interaction, (With<Button>, With<SelectFieldOptionButton>)>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) || !state.any_open() {
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    if !panel_manager.is_cursor_over_any_panel(cursor) {
        return;
    }
    if toggle_buttons.iter().any(|i| *i == Interaction::Pressed)
        || option_buttons.iter().any(|i| *i == Interaction::Pressed)
    {
        return;
    }
    state.close_all();
}

fn sync_select_field_visuals(
    mut commands: Commands,
    state: Res<SelectFieldState>,
    mut labels: Query<(&SelectFieldLabel, &mut Text)>,
    mut options_roots: Query<(
        Entity,
        &SelectFieldOptionsRoot,
        &mut SelectFieldRenderedOptions,
        &mut Node,
    )>,
    mut arrows: Query<(&SelectFieldArrow, &mut ImageNode)>,
    mut button_sets: ParamSet<(
        Query<(&SelectFieldToggleButton, &Interaction, &mut BackgroundColor), With<Button>>,
        Query<(&SelectFieldOptionButton, &Interaction, &mut BackgroundColor), With<Button>>,
    )>,
) {
    for (label, mut text) in &mut labels {
        text.0 = state.selected_label(label.id).unwrap_or("N/A").to_string();
    }

    for (root_entity, options_root, mut rendered, mut node) in &mut options_roots {
        if let Some(entry) = state.entries.get(&options_root.id) {
            if rendered.options != entry.options {
                commands.entity(root_entity).despawn_related::<Children>();
                let options = entry.options.clone();
                commands.entity(root_entity).with_children(|parent| {
                    spawn_option_buttons(parent, options_root.id, &options)
                });
                rendered.options = options;
            }
        }
        node.display = if state.is_open(options_root.id) {
            Display::Flex
        } else {
            Display::None
        };
    }

    for (arrow, mut image_node) in &mut arrows {
        image_node.flip_y = state.is_open(arrow.id);
    }

    for (toggle, interaction, mut bg) in &mut button_sets.p0() {
        bg.0 = if state.is_open(toggle.id) {
            SELECT_BG_OPEN
        } else if *interaction == Interaction::Hovered {
            SELECT_OPTION_HOVER
        } else {
            SELECT_BG
        };
    }

    for (option, interaction, mut bg) in &mut button_sets.p1() {
        let is_selected = state
            .selected_index(option.id)
            .map(|selected| selected == option.option_index)
            .unwrap_or(false);
        bg.0 = if *interaction == Interaction::Hovered {
            SELECT_OPTION_HOVER
        } else if is_selected {
            SELECT_OPTION_SELECTED
        } else {
            SELECT_BG
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{SelectFieldConfig, SelectFieldId, SelectFieldState};

    #[test]
    fn set_selected_is_clamped_by_options_len() {
        let id = SelectFieldId::new("test");
        let mut state = SelectFieldState::default();
        state.register_field(SelectFieldConfig {
            id,
            options: vec!["A".into(), "B".into()],
            selected: 0,
        });
        state.set_selected(id, 99);
        assert_eq!(state.selected_index(id), Some(1));
    }

    #[test]
    fn toggle_open_closes_other_fields() {
        let id_a = SelectFieldId::new("a");
        let id_b = SelectFieldId::new("b");
        let mut state = SelectFieldState::default();
        state.register_field(SelectFieldConfig {
            id: id_a,
            options: vec!["A".into()],
            selected: 0,
        });
        state.register_field(SelectFieldConfig {
            id: id_b,
            options: vec!["B".into()],
            selected: 0,
        });

        state.toggle_open(id_a);
        assert!(state.is_open(id_a));
        assert!(!state.is_open(id_b));

        state.toggle_open(id_b);
        assert!(!state.is_open(id_a));
        assert!(state.is_open(id_b));
    }
}
