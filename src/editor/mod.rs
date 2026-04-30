use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    debug::{DebugMode, DebugOverlaySettings},
    input::camera::MainCamera,
    render::GasVisualSettings,
    simulation::{
        gas::{GasField, GasKind},
        GasSimulationConfig, SimulationStep,
    },
    ui::input_field::{TextInputDisplay, TextInputField, TextInputStyle},
    world::{
        grid::{cell_center, world_to_cell, WorldGrid, CELL_SIZE},
        WorldCellChanged,
    },
};

const PANEL_BG: Color = Color::srgba(0.05, 0.07, 0.10, 0.86);
const BUTTON_IDLE: Color = Color::srgba(0.20, 0.22, 0.25, 0.94);
const BUTTON_ACTIVE: Color = Color::srgba(0.28, 0.47, 0.26, 0.96);
const INPUT_FOCUSED: Color = Color::srgba(0.23, 0.40, 0.56, 0.96);

const TOP_LEFT_SIM_PANEL_WIDTH: f32 = 320.0;
const TOP_LEFT_SIM_PANEL_HEIGHT: f32 = 112.0;

const MAIN_TOOLBAR_LEFT: f32 = 12.0;
const MAIN_TOOLBAR_BOTTOM: f32 = 12.0;
const MAIN_TOOLBAR_WIDTH: f32 = 286.0;
const MAIN_TOOLBAR_HEIGHT: f32 = 48.0;

const DEBUG_TOOLBAR_LEFT: f32 = 306.0;
const DEBUG_TOOLBAR_TOP: f32 = 12.0;
const DEBUG_TOOLBAR_WIDTH: f32 = 286.0;
const DEBUG_TOOLBAR_HEIGHT: f32 = 48.0;

const DEBUG_PANEL_RIGHT: f32 = 12.0;
const DEBUG_PANEL_TOP: f32 = 12.0;
const DEBUG_PANEL_WIDTH: f32 = 286.0;
const DEBUG_PANEL_HEIGHT: f32 = 252.0;
const DEBUG_AND_GAS_PANEL_GAP: f32 = 12.0;

const GAS_PANEL_RIGHT: f32 = 12.0;
const GAS_PANEL_TOP: f32 = DEBUG_PANEL_TOP + DEBUG_PANEL_HEIGHT + DEBUG_AND_GAS_PANEL_GAP;
const GAS_PANEL_WIDTH: f32 = 286.0;
const GAS_PANEL_HEIGHT: f32 = 156.0;

#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EditorTool {
    #[default]
    None,
    BuildSolid,
    EraseSolid,
    AddGas,
    ClearGas,
}

#[derive(Resource, Default)]
pub struct MainToolbarState {
    pub selected: EditorTool,
}

#[derive(Resource)]
pub struct GasToolSettings {
    pub gas_kind: GasKind,
    pub amount: u32,
    pub replace: bool,
}

impl Default for GasToolSettings {
    fn default() -> Self {
        Self {
            gas_kind: GasKind::Hydrogen,
            amount: 100,
            replace: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct SelectionDragState {
    pub active: bool,
    pub start: Option<UVec2>,
    pub current: Option<UVec2>,
}

#[derive(Resource, Default)]
struct BrushDragState {
    active: bool,
    last_cell: Option<UVec2>,
}

#[derive(Component, Clone, Copy)]
enum EditorUiAction {
    SelectTool(EditorTool),
    ToggleGasKind,
    ToggleReplace,
    ToggleLbmVelocity,
    ToggleDiffusion,
    ToggleShowDiffusionCells,
}

#[derive(Component)]
struct DebugToolbarRoot;

#[derive(Component)]
struct DebugPanelRoot;

#[derive(Component)]
struct GasToolPanelRoot;

#[derive(Component)]
struct GasReplaceLabel;

#[derive(Component)]
struct GasKindLabel;

#[derive(Component)]
struct GasAmountInputField;

#[derive(Component)]
struct LbmToggleLabel;

#[derive(Component)]
struct DiffusionToggleLabel;

#[derive(Component)]
struct DiffusionCellsToggleLabel;

#[derive(Component)]
struct DiffusionCellsToggleButton;

#[derive(Component)]
struct GasGammaInputField;

#[derive(Component)]
struct GasMaxColorParticlesInputField;

#[derive(Component)]
struct BlueprintGhost;

#[derive(Component)]
struct EraseCursorOverlay;

#[derive(Component)]
struct EraseCellHighlight;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorTool>()
            .init_resource::<MainToolbarState>()
            .init_resource::<GasToolSettings>()
            .init_resource::<SelectionDragState>()
            .init_resource::<BrushDragState>()
            .add_systems(Startup, (setup_editor_ui, setup_editor_overlays))
            .add_systems(
                Update,
                (
                    handle_editor_ui_actions,
                    refresh_editor_ui,
                    update_editor_cursor_overlays,
                    handle_editor_mouse_input,
                    draw_selection_overlay,
                )
                    .chain(),
            );
    }
}

fn setup_editor_ui(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(MAIN_TOOLBAR_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(MAIN_TOOLBAR_WIDTH),
                height: Val::Px(MAIN_TOOLBAR_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|parent| {
            spawn_tool_button(parent, "None", EditorTool::None);
            spawn_tool_button(parent, "Build", EditorTool::BuildSolid);
            spawn_tool_button(parent, "Erase", EditorTool::EraseSolid);
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(DEBUG_TOOLBAR_LEFT),
                top: Val::Px(DEBUG_TOOLBAR_TOP),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(DEBUG_TOOLBAR_WIDTH),
                height: Val::Px(DEBUG_TOOLBAR_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DebugToolbarRoot,
        ))
        .with_children(|parent| {
            spawn_tool_button(parent, "Add Gas", EditorTool::AddGas);
            spawn_tool_button(parent, "Clear Gas", EditorTool::ClearGas);
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(DEBUG_PANEL_RIGHT),
                top: Val::Px(DEBUG_PANEL_TOP),
                width: Val::Px(DEBUG_PANEL_WIDTH),
                height: Val::Px(DEBUG_PANEL_HEIGHT),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            DebugPanelRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Debug Panel"),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgba(0.95, 0.97, 1.0, 1.0)),
            ));

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    EditorUiAction::ToggleLbmVelocity,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("LBM/Velocity: On"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                        LbmToggleLabel,
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    EditorUiAction::ToggleDiffusion,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Diffusion: On"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                        DiffusionToggleLabel,
                    ));
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(220.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    EditorUiAction::ToggleShowDiffusionCells,
                    DiffusionCellsToggleButton,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Show diffusion cells: On"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                        DiffusionCellsToggleLabel,
                    ));
                });

            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Gamma:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));

                    row
                        .spawn((
                            Button,
                            Node {
                                min_width: Val::Px(92.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_IDLE),
                            TextInputField::new_f32(1.0, 0.0, 10.0, 6, 3),
                            TextInputStyle {
                                idle_bg: BUTTON_IDLE,
                                focused_bg: INPUT_FOCUSED,
                            },
                            bevy::ui::RelativeCursorPosition::default(),
                            GasGammaInputField,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("1"),
                                TextFont::from_font_size(13.0),
                                TextColor(Color::WHITE),
                                TextInputDisplay,
                            ));
                        });
                });

            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Max color at:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));

                    row
                        .spawn((
                            Button,
                            Node {
                                min_width: Val::Px(92.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_IDLE),
                            TextInputField::new_u32(1000, 1, 10_000, 5),
                            TextInputStyle {
                                idle_bg: BUTTON_IDLE,
                                focused_bg: INPUT_FOCUSED,
                            },
                            bevy::ui::RelativeCursorPosition::default(),
                            GasMaxColorParticlesInputField,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("1000"),
                                TextFont::from_font_size(13.0),
                                TextColor(Color::WHITE),
                                TextInputDisplay,
                            ));
                        });
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(GAS_PANEL_RIGHT),
                top: Val::Px(GAS_PANEL_TOP),
                width: Val::Px(GAS_PANEL_WIDTH),
                height: Val::Px(GAS_PANEL_HEIGHT),
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            GasToolPanelRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(190.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    EditorUiAction::ToggleGasKind,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Gas: Hydrogen"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                        GasKindLabel,
                    ));
                });

            parent
                .spawn((
                    Node {
                        display: Display::Flex,
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Amount:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));

                    row
                        .spawn((
                            Button,
                            Node {
                                min_width: Val::Px(112.0),
                                height: Val::Px(30.0),
                                justify_content: JustifyContent::FlexStart,
                                align_items: AlignItems::Center,
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                ..default()
                            },
                            BackgroundColor(BUTTON_IDLE),
                            TextInputField::new_u32(100, 1, 1_000_000, 7),
                            TextInputStyle {
                                idle_bg: BUTTON_IDLE,
                                focused_bg: INPUT_FOCUSED,
                            },
                            bevy::ui::RelativeCursorPosition::default(),
                            GasAmountInputField,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("100"),
                                TextFont::from_font_size(13.0),
                                TextColor(Color::WHITE),
                                TextInputDisplay,
                            ));
                        });
                });

            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(190.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    EditorUiAction::ToggleReplace,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Replace: Off"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                        GasReplaceLabel,
                    ));
                });
        });
}

fn spawn_tool_button(parent: &mut ChildSpawnerCommands, label: &str, tool: EditorTool) {
    parent
        .spawn((
            Button,
            Node {
                min_width: Val::Px(82.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectTool(tool),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label),
                TextFont::from_font_size(13.0),
                TextColor(Color::WHITE),
            ));
        });
}

fn setup_editor_overlays(mut commands: Commands) {
    commands.spawn((
        Sprite::from_color(Color::srgba(0.15, 0.9, 0.25, 0.16), Vec2::splat(CELL_SIZE - 2.0)),
        Transform::from_xyz(0.0, 0.0, 1.8),
        Visibility::Hidden,
        BlueprintGhost,
    ));

    commands.spawn((
        Sprite::from_color(Color::srgba(1.0, 0.24, 0.24, 0.28), Vec2::splat(CELL_SIZE - 2.0)),
        Transform::from_xyz(0.0, 0.0, 1.79),
        Visibility::Hidden,
        EraseCellHighlight,
    ));

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                ..default()
            },
            Visibility::Hidden,
            EraseCursorOverlay,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("X"),
                TextFont::from_font_size(24.0),
                TextColor(Color::srgba(1.0, 0.25, 0.25, 0.95)),
            ));
        });
}

fn handle_editor_ui_actions(
    mut interactions: Query<(&Interaction, &EditorUiAction), (Changed<Interaction>, With<Button>)>,
    mut tool: ResMut<EditorTool>,
    mut main_toolbar: ResMut<MainToolbarState>,
    mut gas_settings: ResMut<GasToolSettings>,
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut debug_overlay: ResMut<DebugOverlaySettings>,
    mut input_set: ParamSet<(
        Single<&mut TextInputField, With<GasAmountInputField>>,
        Single<&mut TextInputField, With<GasGammaInputField>>,
        Single<&mut TextInputField, With<GasMaxColorParticlesInputField>>,
    )>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
) {
    let mut unfocus_inputs = || {
        let mut gas_input = input_set.p0();
        gas_input.focused = false;
        let mut gamma_input = input_set.p1();
        gamma_input.focused = false;
        let mut max_color_particles_input = input_set.p2();
        max_color_particles_input.focused = false;
    };

    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *action {
            EditorUiAction::SelectTool(next_tool) => {
                *tool = next_tool;
                main_toolbar.selected = next_tool;
                unfocus_inputs();
                selection_drag.active = false;
                selection_drag.start = None;
                selection_drag.current = None;
                brush_drag.active = false;
                brush_drag.last_cell = None;
            }
            EditorUiAction::ToggleGasKind => {
                unfocus_inputs();
                gas_settings.gas_kind = gas_settings.gas_kind.next();
            }
            EditorUiAction::ToggleReplace => {
                unfocus_inputs();
                gas_settings.replace = !gas_settings.replace;
            }
            EditorUiAction::ToggleLbmVelocity => {
                unfocus_inputs();
                gas_simulation.enable_lbm_velocity = !gas_simulation.enable_lbm_velocity;
            }
            EditorUiAction::ToggleDiffusion => {
                unfocus_inputs();
                gas_simulation.enable_diffusion = !gas_simulation.enable_diffusion;
            }
            EditorUiAction::ToggleShowDiffusionCells => {
                unfocus_inputs();
                debug_overlay.show_diffusion_cells = !debug_overlay.show_diffusion_cells;
            }
        }
    }
}

fn refresh_editor_ui(
    tool: Res<EditorTool>,
    debug_mode: Res<DebugMode>,
    gas_simulation: Res<GasSimulationConfig>,
    debug_overlay: Res<DebugOverlaySettings>,
    mut gas_visual_settings: ResMut<GasVisualSettings>,
    mut gas_settings: ResMut<GasToolSettings>,
    gas_input: Single<&TextInputField, With<GasAmountInputField>>,
    gamma_input: Single<&TextInputField, With<GasGammaInputField>>,
    max_color_particles_input: Single<&TextInputField, With<GasMaxColorParticlesInputField>>,
    mut button_query: Query<(&EditorUiAction, &mut BackgroundColor), With<Button>>,
    mut visibility_set: ParamSet<(
        Single<&mut Visibility, With<DebugToolbarRoot>>,
        Single<&mut Visibility, With<DebugPanelRoot>>,
        Single<&mut Visibility, With<GasToolPanelRoot>>,
        Single<&mut Visibility, With<DiffusionCellsToggleButton>>,
    )>,
    mut text_set: ParamSet<(
        Single<&mut Text, With<GasReplaceLabel>>,
        Single<&mut Text, With<GasKindLabel>>,
        Single<&mut Text, With<LbmToggleLabel>>,
        Single<&mut Text, With<DiffusionToggleLabel>>,
        Single<&mut Text, With<DiffusionCellsToggleLabel>>,
    )>,
) {
    if let Some(amount) = gas_input.parsed_u32() {
        gas_settings.amount = amount;
    }
    if let Some(gamma) = gamma_input.parsed_f32() {
        let next_gamma = gamma.clamp(0.0, 10.0);
        if (gas_visual_settings.gamma - next_gamma).abs() > f32::EPSILON {
            gas_visual_settings.gamma = next_gamma;
        }
    }
    if let Some(max_particles) = max_color_particles_input.parsed_u32() {
        let next_max_particles = max_particles.clamp(1, 10_000);
        if gas_visual_settings.max_particles_for_max_color != next_max_particles {
            gas_visual_settings.max_particles_for_max_color = next_max_particles;
        }
    }

    for (action, mut bg) in &mut button_query {
        bg.0 = match action {
            EditorUiAction::SelectTool(action_tool) if *action_tool == *tool => BUTTON_ACTIVE,
            EditorUiAction::ToggleGasKind if *tool == EditorTool::AddGas => BUTTON_ACTIVE,
            EditorUiAction::ToggleReplace if gas_settings.replace => BUTTON_ACTIVE,
            EditorUiAction::ToggleLbmVelocity if gas_simulation.enable_lbm_velocity => BUTTON_ACTIVE,
            EditorUiAction::ToggleDiffusion if gas_simulation.enable_diffusion => BUTTON_ACTIVE,
            EditorUiAction::ToggleShowDiffusionCells if debug_overlay.show_diffusion_cells => BUTTON_ACTIVE,
            _ => BUTTON_IDLE,
        };
    }

    {
        let mut debug_toolbar_root = visibility_set.p0();
        **debug_toolbar_root = if debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut debug_panel = visibility_set.p1();
        **debug_panel = if debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut gas_tool_panel = visibility_set.p2();
        **gas_tool_panel = if debug_mode.active && *tool == EditorTool::AddGas {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut diffusion_cells_toggle_button = visibility_set.p3();
        **diffusion_cells_toggle_button = if debug_mode.active && gas_simulation.enable_diffusion {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut gas_kind_text = text_set.p1();
        gas_kind_text.0 = format!("Gas: {}", gas_settings.gas_kind.label());
    }

    {
        let mut replace_text = text_set.p0();
        replace_text.0 = if gas_settings.replace {
            "Replace: On".to_string()
        } else {
            "Replace: Off".to_string()
        };
    }

    {
        let mut lbm_toggle_text = text_set.p2();
        lbm_toggle_text.0 = if gas_simulation.enable_lbm_velocity {
            "LBM/Velocity: On".to_string()
        } else {
            "LBM/Velocity: Off".to_string()
        };
    }

    {
        let mut diffusion_toggle_text = text_set.p3();
        diffusion_toggle_text.0 = if gas_simulation.enable_diffusion {
            "Diffusion: On".to_string()
        } else {
            "Diffusion: Off".to_string()
        };
    }

    {
        let mut diffusion_cells_toggle_text = text_set.p4();
        diffusion_cells_toggle_text.0 = if debug_overlay.show_diffusion_cells {
            "Show diffusion cells: On".to_string()
        } else {
            "Show diffusion cells: Off".to_string()
        };
    }
}

fn handle_editor_mouse_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    tool: Res<EditorTool>,
    debug_mode: Res<DebugMode>,
    gas_settings: Res<GasToolSettings>,
    gas_input: Single<&TextInputField, With<GasAmountInputField>>,
    mut world: ResMut<WorldGrid>,
    mut gas: ResMut<GasField>,
    mut step: ResMut<SimulationStep>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    let cursor_position = window.cursor_position();
    let blocked_by_ui = cursor_position
        .map(|cursor| is_cursor_over_ui(cursor, &window, debug_mode.active, *tool))
        .unwrap_or(false);

    let hovered_cell = cursor_position
        .and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    match *tool {
        EditorTool::BuildSolid => {
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    if world.set_solid(cell.x, cell.y) {
                        gas.clear_cell(cell.x, cell.y);
                        step.0 = step.0.wrapping_add(1);
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        EditorTool::EraseSolid => {
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    if world.set_empty(cell.x, cell.y) {
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        EditorTool::AddGas | EditorTool::ClearGas => {
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    selection_drag.active = true;
                    selection_drag.start = Some(cell);
                    selection_drag.current = Some(cell);
                }
            }

            if selection_drag.active && mouse_buttons.pressed(MouseButton::Left) {
                if let Some(cell) = hovered_cell {
                    selection_drag.current = Some(cell);
                }
            }

            if selection_drag.active && mouse_buttons.just_released(MouseButton::Left) {
                if let (Some(start), Some(end)) = (selection_drag.start, selection_drag.current) {
                    let (min, max) = normalized_rect(start, end);
                    let amount = gas_input.parsed_u32().unwrap_or(gas_settings.amount);
                    match *tool {
                        EditorTool::AddGas => {
                            gas.apply_rect(
                                min,
                                max,
                                gas_settings.gas_kind,
                                amount,
                                gas_settings.replace,
                                &world,
                            );
                            step.0 = step.0.wrapping_add(1);
                        }
                        EditorTool::ClearGas => {
                            gas.clear_rect(min, max);
                            step.0 = step.0.wrapping_add(1);
                        }
                        _ => {}
                    }
                }

                selection_drag.active = false;
                selection_drag.start = None;
                selection_drag.current = None;
            }
        }
        EditorTool::None => {
            brush_drag.active = false;
            brush_drag.last_cell = None;
            selection_drag.active = false;
            selection_drag.start = None;
            selection_drag.current = None;
        }
    }
}

fn apply_brush_tool(
    mouse_buttons: &ButtonInput<MouseButton>,
    blocked_by_ui: bool,
    hovered_cell: Option<UVec2>,
    brush_drag: &mut BrushDragState,
    mut apply_cell: impl FnMut(UVec2),
) {
    if mouse_buttons.just_released(MouseButton::Left) {
        brush_drag.active = false;
        brush_drag.last_cell = None;
        return;
    }

    if blocked_by_ui {
        return;
    }

    if mouse_buttons.just_pressed(MouseButton::Left) {
        brush_drag.active = true;
        brush_drag.last_cell = None;
    }

    if !brush_drag.active || !mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    let Some(cell) = hovered_cell else {
        return;
    };

    if brush_drag.last_cell == Some(cell) {
        return;
    }

    apply_cell(cell);
    brush_drag.last_cell = Some(cell);
}

fn update_editor_cursor_overlays(
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    tool: Res<EditorTool>,
    debug_mode: Res<DebugMode>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut overlay_set: ParamSet<(
        Single<(&mut Transform, &mut Visibility), With<BlueprintGhost>>,
        Single<(&mut Transform, &mut Visibility), With<EraseCellHighlight>>,
        Single<(&mut Node, &mut Visibility), With<EraseCursorOverlay>>,
    )>,
) {
    let cursor_position = window.cursor_position();
    let is_on_ui = cursor_position
        .map(|cursor| is_cursor_over_ui(cursor, &window, debug_mode.active, *tool))
        .unwrap_or(false);

    let world_cell = cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    {
        let mut blueprint = overlay_set.p0();
        let (ghost_transform, ghost_visibility) = &mut *blueprint;
        if *tool == EditorTool::BuildSolid && !is_on_ui && !mouse_buttons.pressed(MouseButton::Left) {
            if let Some(cell) = world_cell {
                **ghost_transform = Transform::from_translation(cell_center(cell.x, cell.y).extend(1.8));
                **ghost_visibility = Visibility::Visible;
            } else {
                **ghost_visibility = Visibility::Hidden;
            }
        } else {
            **ghost_visibility = Visibility::Hidden;
        }
    }

    {
        let mut erase_highlight = overlay_set.p1();
        let (highlight_transform, highlight_visibility) = &mut *erase_highlight;
        if *tool == EditorTool::EraseSolid && !is_on_ui {
            if let Some(cell) = world_cell {
                **highlight_transform = Transform::from_translation(cell_center(cell.x, cell.y).extend(1.79));
                **highlight_visibility = Visibility::Visible;
            } else {
                **highlight_visibility = Visibility::Hidden;
            }
        } else {
            **highlight_visibility = Visibility::Hidden;
        }
    }

    {
        let mut erase_overlay = overlay_set.p2();
        let (erase_node, erase_visibility) = &mut *erase_overlay;
        if *tool == EditorTool::EraseSolid {
            if let Some(cursor) = cursor_position {
                erase_node.left = Val::Px(cursor.x + 10.0);
                erase_node.top = Val::Px(cursor.y + 8.0);
                **erase_visibility = Visibility::Visible;
            } else {
                **erase_visibility = Visibility::Hidden;
            }
        } else {
            **erase_visibility = Visibility::Hidden;
        }
    }
}

fn draw_selection_overlay(selection_drag: Res<SelectionDragState>, mut gizmos: Gizmos) {
    if !selection_drag.active {
        return;
    }

    let (Some(start), Some(current)) = (selection_drag.start, selection_drag.current) else {
        return;
    };

    let (min, max) = normalized_rect(start, current);

    let min_center = cell_center(min.x, min.y);
    let max_center = cell_center(max.x, max.y);
    let center = (min_center + max_center) * 0.5;

    let size = Vec2::new(
        (max.x - min.x + 1) as f32 * CELL_SIZE - 1.0,
        (max.y - min.y + 1) as f32 * CELL_SIZE - 1.0,
    );

    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size,
        Color::srgba(0.97, 0.89, 0.20, 0.95),
    );
}

fn normalized_rect(a: UVec2, b: UVec2) -> (UVec2, UVec2) {
    let min = UVec2::new(a.x.min(b.x), a.y.min(b.y));
    let max = UVec2::new(a.x.max(b.x), a.y.max(b.y));
    (min, max)
}

fn viewport_cursor_to_cell(cursor: Vec2, camera_query: &Single<(&Camera, &GlobalTransform), With<MainCamera>>) -> Option<UVec2> {
    let (camera, camera_transform) = **camera_query;
    let world_pos = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    world_to_cell(world_pos)
}

fn is_cursor_over_ui(cursor: Vec2, window: &Window, debug_mode_active: bool, tool: EditorTool) -> bool {
    let mut rects = vec![
        UiRectPx::top_left(12.0, 12.0, TOP_LEFT_SIM_PANEL_WIDTH, TOP_LEFT_SIM_PANEL_HEIGHT),
        UiRectPx::top_left(
            MAIN_TOOLBAR_LEFT,
            window.height() - MAIN_TOOLBAR_BOTTOM - MAIN_TOOLBAR_HEIGHT,
            MAIN_TOOLBAR_WIDTH,
            MAIN_TOOLBAR_HEIGHT,
        ),
    ];

    if debug_mode_active {
        rects.push(UiRectPx::top_left(
            DEBUG_TOOLBAR_LEFT,
            DEBUG_TOOLBAR_TOP,
            DEBUG_TOOLBAR_WIDTH,
            DEBUG_TOOLBAR_HEIGHT,
        ));

        rects.push(UiRectPx::top_left(
            window.width() - DEBUG_PANEL_RIGHT - DEBUG_PANEL_WIDTH,
            DEBUG_PANEL_TOP,
            DEBUG_PANEL_WIDTH,
            DEBUG_PANEL_HEIGHT,
        ));

        if tool == EditorTool::AddGas {
            rects.push(UiRectPx::top_left(
                window.width() - GAS_PANEL_RIGHT - GAS_PANEL_WIDTH,
                GAS_PANEL_TOP,
                GAS_PANEL_WIDTH,
                GAS_PANEL_HEIGHT,
            ));
        }
    }

    rects.into_iter().any(|rect| rect.contains(cursor))
}

#[derive(Clone, Copy)]
struct UiRectPx {
    left: f32,
    top: f32,
    width: f32,
    height: f32,
}

impl UiRectPx {
    fn top_left(left: f32, top: f32, width: f32, height: f32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }

    fn contains(self, point: Vec2) -> bool {
        point.x >= self.left
            && point.x <= self.left + self.width
            && point.y >= self.top
            && point.y <= self.top + self.height
    }
}
