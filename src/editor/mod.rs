use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};

use crate::{
    debug::{DebugGasMetrics, DebugMode, DebugOverlaySettings},
    input::camera::MainCamera,
    render::GasVisualSettings,
    simulation::{
        gas::{GasField, GasKind},
        GasSimulationConfig, SimulationControl, SimulationPerfStats, SimulationRateConfig,
        SimulationStep,
    },
    ui::input_field::{TextInputDisplay, TextInputField, TextInputStyle},
    world::{
        grid::{cell_center, world_to_cell, CellMaterial, WorldGrid, CELL_SIZE},
        WorldCellChanged,
    },
};

const PANEL_BG: Color = Color::srgba(0.91, 0.92, 0.93, 0.96);
const BUTTON_IDLE: Color = Color::srgba(0.78, 0.80, 0.83, 0.95);
const BUTTON_ACTIVE: Color = Color::srgba(0.58, 0.68, 0.58, 0.96);
const INPUT_FOCUSED: Color = Color::srgba(0.66, 0.76, 0.86, 0.96);
const TOOL_BUTTON_SIZE: f32 = 40.0;
const TOOL_ICON_SIZE: f32 = 20.0;
const TOOLTIP_BG: Color = Color::srgba(0.12, 0.14, 0.16, 0.94);
const MODAL_OVERLAY_BG: Color = Color::srgba(0.02, 0.02, 0.03, 0.60);
const MODAL_BG: Color = Color::srgba(0.96, 0.96, 0.97, 0.98);
const MODAL_BUTTON_BG: Color = Color::srgba(0.78, 0.34, 0.32, 0.95);
const MODAL_BUTTON_HOVER: Color = Color::srgba(0.86, 0.42, 0.38, 0.98);

const TOP_LEFT_SIM_PANEL_WIDTH: f32 = 320.0;
const TOP_LEFT_SIM_PANEL_HEIGHT: f32 = 112.0;

const MAIN_TOOLBAR_LEFT: f32 = 12.0;
const MAIN_TOOLBAR_BOTTOM: f32 = 12.0;
const MAIN_TOOLBAR_WIDTH: f32 = 120.0;
const MAIN_TOOLBAR_HEIGHT: f32 = 56.0;
const CELL_TYPE_PANEL_HEIGHT: f32 = 56.0;
const CELL_TYPE_PANEL_BOTTOM: f32 = MAIN_TOOLBAR_BOTTOM + MAIN_TOOLBAR_HEIGHT + 10.0;
const CELL_TYPE_PANEL_WIDTH: f32 = 120.0;

const DEBUG_TOOLBAR_LEFT: f32 = 306.0;
const DEBUG_TOOLBAR_TOP: f32 = 12.0;
const DEBUG_TOOLBAR_WIDTH: f32 = 104.0;
const DEBUG_TOOLBAR_HEIGHT: f32 = 56.0;

const DEBUG_PANEL_RIGHT: f32 = 12.0;
const DEBUG_PANEL_TOP: f32 = 12.0;
const DEBUG_PANEL_WIDTH: f32 = 286.0;
const DEBUG_PANEL_HEIGHT: f32 = 620.0;
const DEBUG_AND_GAS_PANEL_GAP: f32 = 12.0;

const GAS_PANEL_RIGHT: f32 = 12.0;
const GAS_PANEL_TOP: f32 = DEBUG_PANEL_TOP + DEBUG_PANEL_HEIGHT + DEBUG_AND_GAS_PANEL_GAP;
const GAS_PANEL_WIDTH: f32 = 286.0;
const GAS_PANEL_HEIGHT: f32 = 156.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    BuildSolid,
    EraseSolid,
    AddGas,
    ClearGas,
}

#[derive(Resource, Default)]
pub struct ActiveEditorTool {
    pub selected: Option<EditorTool>,
}

#[derive(Resource)]
pub struct CellToolSettings {
    pub material: CellMaterial,
}

impl Default for CellToolSettings {
    fn default() -> Self {
        Self {
            material: CellMaterial::Brick,
        }
    }
}

#[derive(Resource, Default)]
pub struct MainMenuState {
    pub open: bool,
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
    SelectCellMaterial(CellMaterial),
    ToggleGasKind,
    ToggleReplace,
    ToggleLbmVelocity,
    ToggleDiffusion,
    ToggleBuoyancy,
    ToggleShowMomentumVectors,
}

#[derive(Component)]
struct DebugToolbarRoot;

#[derive(Component)]
struct CellTypePanelRoot;

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
struct BuoyancyToggleLabel;

#[derive(Component)]
struct SimulationPerfLabel;

#[derive(Component)]
struct GasGammaInputField;

#[derive(Component)]
struct GasMaxColorParticlesInputField;

#[derive(Component)]
struct SimulationHzInputField;

#[derive(Component)]
struct BuoyancyStrengthInputField;

#[derive(Component)]
struct BuoyancyWindowRadiusInputField;

#[derive(Component)]
struct BuoyancyWindowSigmaInputField;

#[derive(Component)]
struct BuoyancyGainInputField;

#[derive(Component)]
struct BuoyancyAlphaInputField;

#[derive(Component)]
struct BuoyancyForceCapInputField;

#[derive(Component)]
struct WaveMetricsLabel;

#[derive(Component)]
struct BlueprintGhost;

#[derive(Component)]
struct EraseCursorOverlay;

#[derive(Component)]
struct EraseCellHighlight;

#[derive(Component)]
struct MainMenuRoot;

#[derive(Component)]
struct MainMenuExitButton;

#[derive(Component)]
struct ToolButtonMeta {
    label: &'static str,
}

#[derive(Component)]
struct UiTooltipRoot;

#[derive(Component)]
struct UiTooltipText;

#[derive(Component)]
struct SelectionSizeTooltip;

#[derive(Component)]
struct SelectionSizeTooltipText;

#[derive(Resource, Clone)]
struct EditorIconSet {
    build: Handle<Image>,
    erase: Handle<Image>,
    add_gas: Handle<Image>,
    clear_gas: Handle<Image>,
    brick: Handle<Image>,
    metal: Handle<Image>,
    brick_silhouette: Handle<Image>,
    metal_silhouette: Handle<Image>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EscAction {
    CloseMenuKeepPaused,
    ClearSelectedTool,
    OpenMenuAndPause,
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEditorTool>()
            .init_resource::<CellToolSettings>()
            .init_resource::<MainMenuState>()
            .init_resource::<GasToolSettings>()
            .init_resource::<SelectionDragState>()
            .init_resource::<BrushDragState>()
            .add_systems(Startup, (setup_editor_ui, setup_editor_overlays))
            .add_systems(Update, handle_escape_and_main_menu)
            .add_systems(Update, handle_editor_ui_actions)
            .add_systems(Update, refresh_editor_ui)
            .add_systems(
                Update,
                (
                    update_tool_button_tooltip,
                    update_editor_cursor_overlays,
                    handle_editor_mouse_input,
                    draw_selection_overlay,
                    update_selection_size_tooltip,
                ),
            );
    }
}

fn setup_editor_ui(mut commands: Commands, asset_server: Res<AssetServer>) {
    let icon_set = EditorIconSet {
        build: asset_server.load("sprites/ui/tool_build.png"),
        erase: asset_server.load("sprites/ui/tool_erase.png"),
        add_gas: asset_server.load("sprites/ui/tool_add_gas.png"),
        clear_gas: asset_server.load("sprites/ui/tool_clear_gas.png"),
        brick: asset_server.load("sprites/ui/tool_brick.png"),
        metal: asset_server.load("sprites/ui/tool_metal.png"),
        brick_silhouette: asset_server.load("sprites/ui/silhouette_brick.png"),
        metal_silhouette: asset_server.load("sprites/ui/silhouette_metal.png"),
    };
    commands.insert_resource(icon_set.clone());

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
            spawn_tool_button(
                parent,
                "Build",
                EditorTool::BuildSolid,
                icon_set.build.clone(),
            );
            spawn_tool_button(
                parent,
                "Erase",
                EditorTool::EraseSolid,
                icon_set.erase.clone(),
            );
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(MAIN_TOOLBAR_LEFT),
                bottom: Val::Px(CELL_TYPE_PANEL_BOTTOM),
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(8.0),
                width: Val::Px(CELL_TYPE_PANEL_WIDTH),
                height: Val::Px(CELL_TYPE_PANEL_HEIGHT),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            CellTypePanelRoot,
        ))
        .with_children(|parent| {
            spawn_cell_material_button(
                parent,
                "Brick",
                CellMaterial::Brick,
                icon_set.brick.clone(),
            );
            spawn_cell_material_button(
                parent,
                "Metal",
                CellMaterial::Metal,
                icon_set.metal.clone(),
            );
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
            spawn_tool_button(
                parent,
                "Add Gas",
                EditorTool::AddGas,
                icon_set.add_gas.clone(),
            );
            spawn_tool_button(
                parent,
                "Clear Gas",
                EditorTool::ClearGas,
                icon_set.clear_gas.clone(),
            );
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
                TextColor(Color::srgba(0.13, 0.14, 0.16, 1.0)),
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
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                    EditorUiAction::ToggleBuoyancy,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Buoyancy: On"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                        BuoyancyToggleLabel,
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
                    EditorUiAction::ToggleShowMomentumVectors,
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("Show impulses"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                    ));
                });

            parent.spawn((
                Text::new("Step ms: 0.000 | avg: 0.000 | Target Hz: 30 x 1 = 30 | Actual Hz: 0"),
                TextFont::from_font_size(13.0),
                TextColor(Color::WHITE),
                SimulationPerfLabel,
            ));

            parent.spawn((
                Text::new(
                    "Anisotropy: 0.0000 | Radial waves: 0.0000 | Mass err H2/O2: 0.0000 / 0.0000",
                ),
                TextFont::from_font_size(13.0),
                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                WaveMetricsLabel,
            ));

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Simulation Hz:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));

                    row.spawn((
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
                        TextInputField::new_u32(30, 1, 1000, 4),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        SimulationHzInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("30"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy strength:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_f32(0.12, 0.0, 5.0, 6, 3),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyStrengthInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("0.12"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy radius:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_u32(2, 1, 3, 1),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyWindowRadiusInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("2"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy sigma:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_f32(1.2, 0.5, 3.0, 6, 3),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyWindowSigmaInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("1.2"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy gain:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_f32(2.2, 0.0, 10.0, 6, 3),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyGainInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("2.2"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy alpha:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_f32(0.9, 0.0, 4.0, 6, 3),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyAlphaInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("0.9"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Buoyancy cap:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::WHITE),
                    ));
                    row.spawn((
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
                        TextInputField::new_f32(0.2, 0.0, 2.0, 6, 3),
                        TextInputStyle {
                            idle_bg: BUTTON_IDLE,
                            focused_bg: INPUT_FOCUSED,
                        },
                        bevy::ui::RelativeCursorPosition::default(),
                        BuoyancyForceCapInputField,
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("0.2"),
                            TextFont::from_font_size(13.0),
                            TextColor(Color::WHITE),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Gamma:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                    ));

                    row.spawn((
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
                            TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                            TextInputDisplay,
                        ));
                    });
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Max color at:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                    ));

                    row.spawn((
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
                            TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                        GasKindLabel,
                    ));
                });

            parent
                .spawn((Node {
                    display: Display::Flex,
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                },))
                .with_children(|row| {
                    row.spawn((
                        Text::new("Amount:"),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                    ));

                    row.spawn((
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
                            TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
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
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                        GasReplaceLabel,
                    ));
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                top: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MODAL_OVERLAY_BG),
            GlobalZIndex(1500),
            Visibility::Hidden,
            MainMenuRoot,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(280.0),
                        height: Val::Px(160.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(14.0),
                        ..default()
                    },
                    BackgroundColor(MODAL_BG),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new("Main Menu"),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::srgba(0.08, 0.09, 0.11, 1.0)),
                    ));
                    panel
                        .spawn((
                            Button,
                            Node {
                                width: Val::Px(132.0),
                                height: Val::Px(42.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            BackgroundColor(MODAL_BUTTON_BG),
                            MainMenuExitButton,
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Exit"),
                                TextFont::from_font_size(16.0),
                                TextColor(Color::WHITE),
                            ));
                        });
                });
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                display: Display::None,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(TOOLTIP_BG),
            GlobalZIndex(2000),
            UiTooltipRoot,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(Color::WHITE),
                UiTooltipText,
            ));
        });

    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                display: Display::None,
                min_width: Val::Px(94.0),
                padding: UiRect::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(TOOLTIP_BG),
            GlobalZIndex(2000),
            SelectionSizeTooltip,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(""),
                TextFont::from_font_size(12.0),
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(JustifyText::Center),
                SelectionSizeTooltipText,
            ));
        });
}

fn spawn_tool_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    tool: EditorTool,
    icon: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(TOOL_BUTTON_SIZE),
                height: Val::Px(TOOL_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectTool(tool),
            ToolButtonMeta { label },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(TOOL_ICON_SIZE),
                    height: Val::Px(TOOL_ICON_SIZE),
                    ..default()
                },
            ));
        });
}

fn spawn_cell_material_button(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    material: CellMaterial,
    icon: Handle<Image>,
) {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(TOOL_BUTTON_SIZE),
                height: Val::Px(TOOL_BUTTON_SIZE),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(BUTTON_IDLE),
            EditorUiAction::SelectCellMaterial(material),
            ToolButtonMeta { label },
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(icon),
                Node {
                    width: Val::Px(TOOL_ICON_SIZE),
                    height: Val::Px(TOOL_ICON_SIZE),
                    ..default()
                },
            ));
        });
}

fn setup_editor_overlays(mut commands: Commands, asset_server: Res<AssetServer>) {
    let brick_silhouette = asset_server.load("sprites/ui/silhouette_brick.png");
    commands.spawn((
        Sprite {
            image: brick_silhouette,
            custom_size: Some(Vec2::splat(CELL_SIZE - 1.0)),
            color: Color::WHITE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.8),
        Visibility::Hidden,
        BlueprintGhost,
    ));

    commands.spawn((
        Sprite::from_color(
            Color::srgba(1.0, 0.24, 0.24, 0.28),
            Vec2::splat(CELL_SIZE - 2.0),
        ),
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

fn clear_active_tool_state(
    selection_drag: &mut ResMut<SelectionDragState>,
    brush_drag: &mut ResMut<BrushDragState>,
) {
    selection_drag.active = false;
    selection_drag.start = None;
    selection_drag.current = None;
    brush_drag.active = false;
    brush_drag.last_cell = None;
}

fn escape_action(main_menu_open: bool, has_selected_tool: bool) -> EscAction {
    if main_menu_open {
        EscAction::CloseMenuKeepPaused
    } else if has_selected_tool {
        EscAction::ClearSelectedTool
    } else {
        EscAction::OpenMenuAndPause
    }
}

fn handle_escape_and_main_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut main_menu: ResMut<MainMenuState>,
    mut control: ResMut<crate::simulation::SimulationControl>,
    mut input_fields: Query<&mut TextInputField>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
    mut exit_button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<MainMenuExitButton>),
    >,
    mut exit_writer: EventWriter<AppExit>,
) {
    for (interaction, mut bg) in &mut exit_button_query {
        match *interaction {
            Interaction::Pressed => {
                bg.0 = MODAL_BUTTON_HOVER;
                exit_writer.write(AppExit::Success);
            }
            Interaction::Hovered => {
                bg.0 = MODAL_BUTTON_HOVER;
            }
            Interaction::None => {
                bg.0 = MODAL_BUTTON_BG;
            }
        }
    }

    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    let mut had_focused_input = false;
    for mut field in &mut input_fields {
        if field.focused {
            field.focused = false;
            had_focused_input = true;
        }
    }
    if had_focused_input {
        return;
    }

    match escape_action(main_menu.open, active_tool.selected.is_some()) {
        EscAction::CloseMenuKeepPaused => {
            main_menu.open = false;
        }
        EscAction::ClearSelectedTool => {
            active_tool.selected = None;
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        }
        EscAction::OpenMenuAndPause => {
            control.paused = true;
            main_menu.open = true;
        }
    }
}

fn update_tool_button_tooltip(
    window: Single<&Window, With<PrimaryWindow>>,
    main_menu: Res<MainMenuState>,
    mut tooltip_node: Single<&mut Node, With<UiTooltipRoot>>,
    mut tooltip_text: Single<&mut Text, With<UiTooltipText>>,
    button_query: Query<(&Interaction, &ToolButtonMeta), With<Button>>,
) {
    let node = &mut *tooltip_node;
    if main_menu.open {
        node.display = Display::None;
        return;
    }

    let mut hovered_text = None;
    for (interaction, meta) in &button_query {
        if *interaction == Interaction::Hovered {
            hovered_text = Some(meta.label);
            break;
        }
    }

    let Some(label) = hovered_text else {
        node.display = Display::None;
        return;
    };
    let Some(cursor) = window.cursor_position() else {
        node.display = Display::None;
        return;
    };

    tooltip_text.0 = label.to_string();
    node.display = Display::Flex;
    node.left = Val::Px((cursor.x + 14.0).min(window.width() - 130.0));
    node.top = Val::Px((cursor.y + 16.0).min(window.height() - 34.0));
}

fn update_selection_size_tooltip(
    selection_drag: Res<SelectionDragState>,
    active_tool: Res<ActiveEditorTool>,
    main_menu: Res<MainMenuState>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut tooltip_node: Single<&mut Node, With<SelectionSizeTooltip>>,
    mut tooltip_text: Single<&mut Text, With<SelectionSizeTooltipText>>,
) {
    let node = &mut *tooltip_node;
    if main_menu.open {
        node.display = Display::None;
        return;
    }

    let Some(tool) = active_tool.selected else {
        node.display = Display::None;
        return;
    };
    if !matches!(tool, EditorTool::AddGas | EditorTool::ClearGas) || !selection_drag.active {
        node.display = Display::None;
        return;
    }

    let (Some(start), Some(end)) = (selection_drag.start, selection_drag.current) else {
        node.display = Display::None;
        return;
    };
    let (min, max) = normalized_rect(start, end);
    let w = max.x - min.x + 1;
    let h = max.y - min.y + 1;
    let area = w * h;

    let min_center = cell_center(min.x, min.y);
    let max_center = cell_center(max.x, max.y);
    let center_world = ((min_center + max_center) * 0.5).extend(0.0);

    let (camera, camera_transform) = *camera_query;
    let Ok(center_screen) = camera.world_to_viewport(camera_transform, center_world) else {
        node.display = Display::None;
        return;
    };

    tooltip_text.0 = format!("{w}X{h}\n{area}");
    node.display = Display::Flex;
    node.left = Val::Px(center_screen.x - 42.0);
    node.top = Val::Px(center_screen.y - 24.0);
}

fn handle_editor_ui_actions(
    mut interactions: Query<(&Interaction, &EditorUiAction), (Changed<Interaction>, With<Button>)>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut cell_settings: ResMut<CellToolSettings>,
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
                active_tool.selected = Some(next_tool);
                unfocus_inputs();
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::SelectCellMaterial(next_material) => {
                unfocus_inputs();
                cell_settings.material = next_material;
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
                gas_simulation.enable_species_relaxation =
                    !gas_simulation.enable_species_relaxation;
            }
            EditorUiAction::ToggleBuoyancy => {
                unfocus_inputs();
                gas_simulation.solver_tuning.enable_buoyancy =
                    !gas_simulation.solver_tuning.enable_buoyancy;
            }
            EditorUiAction::ToggleShowMomentumVectors => {
                unfocus_inputs();
                debug_overlay.show_momentum_vectors = !debug_overlay.show_momentum_vectors;
            }
        }
    }
}

fn refresh_editor_ui(
    ui_state: (
        Res<ActiveEditorTool>,
        Res<CellToolSettings>,
        Res<MainMenuState>,
        Res<DebugMode>,
    ),
    sim_metrics: (
        Res<DebugGasMetrics>,
        Res<SimulationControl>,
        Res<SimulationPerfStats>,
    ),
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut sim_rate: ResMut<SimulationRateConfig>,
    debug_overlay: Res<DebugOverlaySettings>,
    mut gas_visual_settings: ResMut<GasVisualSettings>,
    mut gas_settings: ResMut<GasToolSettings>,
    gas_input: Single<&TextInputField, With<GasAmountInputField>>,
    mut input_set: ParamSet<(
        Single<&TextInputField, With<GasGammaInputField>>,
        Single<&TextInputField, With<GasMaxColorParticlesInputField>>,
        Single<&TextInputField, With<SimulationHzInputField>>,
        Single<&TextInputField, With<BuoyancyStrengthInputField>>,
        Single<&TextInputField, With<BuoyancyWindowRadiusInputField>>,
        Single<&TextInputField, With<BuoyancyWindowSigmaInputField>>,
        Single<&TextInputField, With<BuoyancyGainInputField>>,
        Single<&TextInputField, With<BuoyancyAlphaInputField>>,
    )>,
    buoyancy_cap_input: Single<&TextInputField, With<BuoyancyForceCapInputField>>,
    mut button_query: Query<
        (&EditorUiAction, &mut BackgroundColor),
        (With<Button>, Without<MainMenuExitButton>),
    >,
    mut visibility_set: ParamSet<(
        Single<&mut Visibility, With<CellTypePanelRoot>>,
        Single<&mut Visibility, With<DebugToolbarRoot>>,
        Single<&mut Visibility, With<DebugPanelRoot>>,
        Single<&mut Visibility, With<GasToolPanelRoot>>,
        Single<&mut Visibility, With<MainMenuRoot>>,
    )>,
    mut text_set_primary: ParamSet<(
        Single<&mut Text, With<GasReplaceLabel>>,
        Single<&mut Text, With<GasKindLabel>>,
        Single<&mut Text, With<LbmToggleLabel>>,
        Single<&mut Text, With<DiffusionToggleLabel>>,
        Single<&mut Text, With<BuoyancyToggleLabel>>,
        Single<&mut Text, With<SimulationPerfLabel>>,
        Single<&mut Text, With<WaveMetricsLabel>>,
    )>,
) {
    let (active_tool, cell_settings, main_menu, debug_mode) = ui_state;
    let (debug_metrics, sim_control, sim_perf) = sim_metrics;
    let selected_tool = active_tool.selected;

    if let Some(amount) = gas_input.parsed_u32() {
        gas_settings.amount = amount;
    }
    if let Some(gamma) = input_set.p0().parsed_f32() {
        let next_gamma = gamma.clamp(0.0, 10.0);
        if (gas_visual_settings.gamma - next_gamma).abs() > f32::EPSILON {
            gas_visual_settings.gamma = next_gamma;
        }
    }
    if let Some(max_particles) = input_set.p1().parsed_u32() {
        let next_max_particles = max_particles.clamp(1, 10_000);
        if gas_visual_settings.max_particles_for_max_color != next_max_particles {
            gas_visual_settings.max_particles_for_max_color = next_max_particles;
        }
    }
    if let Some(target_hz) = input_set.p2().parsed_u32() {
        sim_rate.target_hz = target_hz.clamp(1, 1000);
    }
    if let Some(value) = input_set.p3().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_strength = value.clamp(0.0, 5.0);
    }
    if let Some(value) = input_set.p4().parsed_u32() {
        gas_simulation.solver_tuning.buoyancy_window_radius = value.clamp(1, 3) as u8;
    }
    if let Some(value) = input_set.p5().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_window_sigma = value.clamp(0.5, 3.0);
    }
    if let Some(value) = input_set.p6().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_gain = value.clamp(0.0, 10.0);
    }
    if let Some(value) = input_set.p7().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_alpha = value.clamp(0.0, 4.0);
    }
    if let Some(value) = buoyancy_cap_input.parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_force_cap = value.clamp(0.0, 2.0);
    }

    for (action, mut bg) in &mut button_query {
        bg.0 = match action {
            EditorUiAction::SelectTool(action_tool) if Some(*action_tool) == selected_tool => {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectCellMaterial(material) if *material == cell_settings.material => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleGasKind if selected_tool == Some(EditorTool::AddGas) => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleReplace if gas_settings.replace => BUTTON_ACTIVE,
            EditorUiAction::ToggleLbmVelocity if gas_simulation.enable_lbm_velocity => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleDiffusion if gas_simulation.enable_species_relaxation => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleBuoyancy if gas_simulation.solver_tuning.enable_buoyancy => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleShowMomentumVectors if debug_overlay.show_momentum_vectors => {
                BUTTON_ACTIVE
            }
            _ => BUTTON_IDLE,
        };
    }

    {
        let mut cell_type_panel_root = visibility_set.p0();
        **cell_type_panel_root = if selected_tool == Some(EditorTool::BuildSolid) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut debug_toolbar_root = visibility_set.p1();
        **debug_toolbar_root = if debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut debug_panel = visibility_set.p2();
        **debug_panel = if debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut gas_tool_panel = visibility_set.p3();
        **gas_tool_panel = if debug_mode.active && selected_tool == Some(EditorTool::AddGas) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut main_menu_root = visibility_set.p4();
        **main_menu_root = if main_menu.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    
    {
        let mut gas_kind_text = text_set_primary.p1();
        gas_kind_text.0 = format!("Gas: {}", gas_settings.gas_kind.label());
    }

    {
        let mut replace_text = text_set_primary.p0();
        replace_text.0 = if gas_settings.replace {
            "Replace: On".to_string()
        } else {
            "Replace: Off".to_string()
        };
    }

    {
        let mut lbm_toggle_text = text_set_primary.p2();
        lbm_toggle_text.0 = if gas_simulation.enable_lbm_velocity {
            "LBM/Velocity: On".to_string()
        } else {
            "LBM/Velocity: Off".to_string()
        };
    }

    {
        let mut diffusion_toggle_text = text_set_primary.p3();
        diffusion_toggle_text.0 = if gas_simulation.enable_species_relaxation {
            "Species relax: On".to_string()
        } else {
            "Species relax: Off".to_string()
        };
    }

    {
        let mut buoyancy_toggle_text = text_set_primary.p4();
        buoyancy_toggle_text.0 = if gas_simulation.solver_tuning.enable_buoyancy {
            "Buoyancy: On".to_string()
        } else {
            "Buoyancy: Off".to_string()
        };
    }

    {
        let mut perf_text = text_set_primary.p5();
        let speed_mult = sim_control.speed.multiplier();
        perf_text.0 = format!(
            "Step ms: {:.3} | avg: {:.3} | Target Hz: {} x {} = {:.1} | Actual Hz: {:.1} | GPU compute/upload/readback/total: {:.3}/{:.3}/{:.3}/{:.3} ms",
            sim_perf.last_step_ms,
            sim_perf.avg_step_ms,
            sim_rate.target_hz,
            speed_mult,
            sim_perf.target_hz_effective,
            sim_perf.actual_hz,
            sim_perf.last_gpu_compute_ms,
            sim_perf.last_upload_to_gpu_ms,
            sim_perf.last_readback_from_gpu_ms,
            sim_perf.last_step_total_ms
        );
    }

    {
        let mut metrics_text = text_set_primary.p6();
        let vectors_mode = if debug_overlay.show_momentum_vectors {
            "Impulse vectors: On"
        } else {
            "Impulse vectors: Off"
        };
        metrics_text.0 = format!(
            "{} | Anisotropy: {:.4} | Radial waves: {:.4} | Mass err H2/O2: {:.4} / {:.4}",
            vectors_mode,
            debug_metrics.anisotropy_score,
            debug_metrics.radial_wave_score,
            debug_metrics.mass_error_h2,
            debug_metrics.mass_error_o2
        );
    }
}

fn handle_editor_mouse_input(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera_query: Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    active_tool: Res<ActiveEditorTool>,
    cell_settings: Res<CellToolSettings>,
    main_menu: Res<MainMenuState>,
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
    if main_menu.open {
        clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        return;
    }

    let cursor_position = window.cursor_position();
    let blocked_by_ui = cursor_position
        .map(|cursor| {
            is_cursor_over_ui(
                cursor,
                &window,
                debug_mode.active,
                active_tool.selected,
                main_menu.open,
            )
        })
        .unwrap_or(false);

    let hovered_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    match active_tool.selected {
        Some(EditorTool::BuildSolid) => {
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    if world.set_solid_with_material(cell.x, cell.y, cell_settings.material) {
                        gas.clear_cell(cell.x, cell.y);
                        step.0 = step.0.wrapping_add(1);
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::EraseSolid) => {
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
        Some(EditorTool::AddGas) | Some(EditorTool::ClearGas) => {
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
                    match active_tool.selected {
                        Some(EditorTool::AddGas) => {
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
                        Some(EditorTool::ClearGas) => {
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
        None => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
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
    active_tool: Res<ActiveEditorTool>,
    cell_settings: Res<CellToolSettings>,
    main_menu: Res<MainMenuState>,
    icon_set: Res<EditorIconSet>,
    debug_mode: Res<DebugMode>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut overlay_set: ParamSet<(
        Single<(&mut Transform, &mut Visibility, &mut Sprite), With<BlueprintGhost>>,
        Single<(&mut Transform, &mut Visibility), With<EraseCellHighlight>>,
        Single<(&mut Node, &mut Visibility), With<EraseCursorOverlay>>,
    )>,
) {
    let cursor_position = window.cursor_position();
    let is_on_ui = cursor_position
        .map(|cursor| {
            is_cursor_over_ui(
                cursor,
                &window,
                debug_mode.active,
                active_tool.selected,
                main_menu.open,
            )
        })
        .unwrap_or(false);

    let world_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    {
        let mut blueprint = overlay_set.p0();
        let (ghost_transform, ghost_visibility, ghost_sprite) = &mut *blueprint;
        if active_tool.selected == Some(EditorTool::BuildSolid)
            && !is_on_ui
            && !main_menu.open
            && !mouse_buttons.pressed(MouseButton::Left)
        {
            if let Some(cell) = world_cell {
                ghost_sprite.image = match cell_settings.material {
                    CellMaterial::Brick => icon_set.brick_silhouette.clone(),
                    CellMaterial::Metal => icon_set.metal_silhouette.clone(),
                    CellMaterial::Boundary => icon_set.brick_silhouette.clone(),
                };
                **ghost_transform =
                    Transform::from_translation(cell_center(cell.x, cell.y).extend(1.8));
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
        if active_tool.selected == Some(EditorTool::EraseSolid) && !is_on_ui && !main_menu.open {
            if let Some(cell) = world_cell {
                **highlight_transform =
                    Transform::from_translation(cell_center(cell.x, cell.y).extend(1.79));
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
        if active_tool.selected == Some(EditorTool::EraseSolid) && !main_menu.open {
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

fn viewport_cursor_to_cell(
    cursor: Vec2,
    camera_query: &Single<(&Camera, &GlobalTransform), With<MainCamera>>,
) -> Option<UVec2> {
    let (camera, camera_transform) = **camera_query;
    let world_pos = camera.viewport_to_world_2d(camera_transform, cursor).ok()?;
    world_to_cell(world_pos)
}

pub(crate) fn is_cursor_over_ui(
    cursor: Vec2,
    window: &Window,
    debug_mode_active: bool,
    selected_tool: Option<EditorTool>,
    main_menu_open: bool,
) -> bool {
    let mut rects = vec![
        UiRectPx::top_left(
            12.0,
            12.0,
            TOP_LEFT_SIM_PANEL_WIDTH,
            TOP_LEFT_SIM_PANEL_HEIGHT,
        ),
        UiRectPx::top_left(
            MAIN_TOOLBAR_LEFT,
            window.height() - MAIN_TOOLBAR_BOTTOM - MAIN_TOOLBAR_HEIGHT,
            MAIN_TOOLBAR_WIDTH,
            MAIN_TOOLBAR_HEIGHT,
        ),
    ];

    if selected_tool == Some(EditorTool::BuildSolid) {
        rects.push(UiRectPx::top_left(
            MAIN_TOOLBAR_LEFT,
            window.height() - CELL_TYPE_PANEL_BOTTOM - CELL_TYPE_PANEL_HEIGHT,
            CELL_TYPE_PANEL_WIDTH,
            CELL_TYPE_PANEL_HEIGHT,
        ));
    }

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

        if selected_tool == Some(EditorTool::AddGas) {
            rects.push(UiRectPx::top_left(
                window.width() - GAS_PANEL_RIGHT - GAS_PANEL_WIDTH,
                GAS_PANEL_TOP,
                GAS_PANEL_WIDTH,
                GAS_PANEL_HEIGHT,
            ));
        }
    }

    if main_menu_open {
        rects.push(UiRectPx::top_left(
            0.0,
            0.0,
            window.width(),
            window.height(),
        ));
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

#[cfg(test)]
mod tests {
    use super::{escape_action, EscAction};

    #[test]
    fn escape_closes_main_menu_when_open() {
        assert_eq!(
            escape_action(true, true),
            EscAction::CloseMenuKeepPaused,
            "Esc should close main menu first even if a tool is selected"
        );
        assert_eq!(
            escape_action(true, false),
            EscAction::CloseMenuKeepPaused,
            "Esc should close main menu first when no tool is selected"
        );
    }

    #[test]
    fn escape_clears_selected_tool_before_opening_menu() {
        assert_eq!(
            escape_action(false, true),
            EscAction::ClearSelectedTool,
            "Esc should clear selected tool before opening menu"
        );
    }

    #[test]
    fn escape_opens_main_menu_and_pauses_when_no_tool_selected() {
        assert_eq!(
            escape_action(false, false),
            EscAction::OpenMenuAndPause,
            "Esc should open menu and pause when no tool is selected"
        );
    }
}
