use bevy::{app::AppExit, ecs::system::SystemParam, prelude::*, window::PrimaryWindow};

use crate::{
    config::GasRegistry,
    debug::{DebugGasMetrics, DebugMode, DebugOverlaySettings},
    input::camera::MainCamera,
    render::GasVisualSettings,
    save::{
        create_save, list_saves, load_save, new_game_snapshot, overwrite_save, saves_root_default,
        MainMenuConfirmState, MainMenuDeferredAction, MainMenuMode, MainMenuScreen,
        MainMenuUiState, SaveSessionState, WorldLoadState,
    },
    simulation::{
        gas::GasField, GasSimulationConfig, SimulationControl, SimulationPerfStats,
        SimulationRateConfig, SimulationStep,
    },
    ui::{
        input_field::{TextInputDisplay, TextInputField, TextInputStyle},
        panels::{
            PanelControls, PanelCorner, PanelId, PanelManager, PanelOpenOrder, PanelScrollPolicy,
            PanelSpec,
        },
        select_field::{spawn_select_field, SelectFieldConfig, SelectFieldId, SelectFieldState},
    },
    world::{
        gas_structures::{GasStructureCell, GasStructureGrid},
        grid::{
            cell_center, world_to_cell, CellMaterial, WorldGrid, CELL_SIZE, WORLD_HEIGHT,
            WORLD_WIDTH,
        },
        WorldCellChanged,
    },
};

const PANEL_BG: Color = Color::srgba(0.91, 0.92, 0.93, 0.96);
const BUTTON_IDLE: Color = Color::srgba(0.78, 0.80, 0.83, 0.95);
const BUTTON_ACTIVE: Color = Color::srgba(0.58, 0.68, 0.58, 0.96);
const INPUT_FOCUSED: Color = Color::srgba(0.66, 0.76, 0.86, 0.96);
const DEBUG_PANEL_TEXT_COLOR: Color = Color::srgba(0.10, 0.10, 0.12, 1.0);
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
const DEBUG_TOOLBAR_WIDTH: f32 = 200.0;
const DEBUG_TOOLBAR_HEIGHT: f32 = 56.0;

const DEBUG_PANEL_RIGHT: f32 = 12.0;
const DEBUG_PANEL_TOP: f32 = 12.0;
const DEBUG_PANEL_WIDTH: f32 = 286.0;
const DEBUG_AND_GAS_PANEL_GAP: f32 = 12.0;

const GAS_PANEL_RIGHT: f32 = 12.0;
const GAS_PANEL_WIDTH: f32 = 286.0;
const STRUCTURE_PANEL_RIGHT: f32 = 12.0;
const STRUCTURE_PANEL_WIDTH: f32 = 286.0;
const DEBUG_PANEL_ID: PanelId = PanelId::new("debug_panel");
const GAS_TOOL_PANEL_ID: PanelId = PanelId::new("gas_tool_panel");
const STRUCTURE_TOOL_PANEL_ID: PanelId = PanelId::new("structure_tool_panel");
const GAS_SELECT_ADD_ID: SelectFieldId = SelectFieldId::new("gas_select_add");
const GAS_SELECT_SOURCE_ID: SelectFieldId = SelectFieldId::new("gas_select_source");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    BuildSolid,
    EraseSolid,
    AddGas,
    ClearGas,
    CreateGasSource,
    CreateGasSink,
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

#[derive(Resource)]
pub struct MainMenuState {
    pub open: bool,
}

impl Default for MainMenuState {
    fn default() -> Self {
        Self { open: true }
    }
}

#[derive(Resource)]
pub struct GasToolSettings {
    pub gas_index: usize,
    pub amount: u32,
    pub replace: bool,
}

impl Default for GasToolSettings {
    fn default() -> Self {
        Self {
            gas_index: 0,
            amount: 100,
            replace: false,
        }
    }
}

#[derive(Resource)]
pub struct SourceStructureToolSettings {
    pub gas_index: usize,
    pub amount: u32,
}

impl Default for SourceStructureToolSettings {
    fn default() -> Self {
        Self {
            gas_index: 0,
            amount: 100,
        }
    }
}

#[derive(Resource)]
pub struct SinkStructureToolSettings {
    pub amount: u32,
}

impl Default for SinkStructureToolSettings {
    fn default() -> Self {
        Self { amount: 100 }
    }
}

#[derive(Resource, Default, Clone, Copy)]
pub struct StructureEditState {
    pub selected_cell: Option<UVec2>,
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
    ToggleReplace,
    ToggleBuoyancy,
    ToggleShowMomentumVectors,
}

#[derive(Component)]
struct MainToolbarRoot;

#[derive(Component)]
struct DebugToolbarRoot;

#[derive(Component)]
struct CellTypePanelRoot;

#[derive(Component)]
struct GasReplaceLabel;

#[derive(Component)]
struct GasAmountInputField;

#[derive(Component)]
struct SourceAmountInputField;

#[derive(Component)]
struct SinkAmountInputField;

#[derive(Component)]
struct StructureSourceSection;

#[derive(Component)]
struct StructureSinkSection;

#[derive(Component)]
struct StructureModeLabel;

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
struct MainMenuTitleText;

#[derive(Component)]
struct MainMenuStatusText;

#[derive(Component)]
struct MainMenuRootActions;

#[derive(Component)]
struct MainMenuSaveActions;

#[derive(Component)]
struct MainMenuLoadActions;

#[derive(Component)]
struct MainMenuConfirmActions;

#[derive(Component)]
struct MainMenuSaveNameRow;

#[derive(Component)]
struct MainMenuSaveNameInputField;

#[derive(Component)]
struct MainMenuSaveListRoot;

#[derive(Component)]
struct MainMenuConfirmPrimaryLabel;

#[derive(Component)]
struct MainMenuConfirmSecondaryLabel;

#[derive(Component)]
struct MainMenuConfirmCancelLabel;

#[derive(Component, Clone)]
struct MainMenuActionButton(MainMenuButtonAction);

#[derive(Clone)]
enum MainMenuButtonAction {
    Continue,
    NewGame,
    OpenSaveScreen,
    OpenLoadScreen,
    ExitToMainMenu,
    ExitApp,
    BackToRoot,
    CreateNewSave,
    SelectOverwrite(String),
    SelectLoad(String),
    ConfirmPrimary,
    ConfirmSecondary,
    ConfirmCancel,
}

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
    source: Handle<Image>,
    sink: Handle<Image>,
    brick: Handle<Image>,
    metal: Handle<Image>,
    brick_silhouette: Handle<Image>,
    metal_silhouette: Handle<Image>,
    source_silhouette: Handle<Image>,
    sink_silhouette: Handle<Image>,
    select_arrow: Handle<Image>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EscAction {
    CloseMenuKeepPaused,
    CloseStructureEditor,
    ClearSelectedTool,
    OpenMenuAndPause,
    Ignore,
}

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEditorTool>()
            .init_resource::<CellToolSettings>()
            .init_resource::<MainMenuState>()
            .init_resource::<MainMenuUiState>()
            .init_resource::<SaveSessionState>()
            .init_resource::<WorldLoadState>()
            .init_resource::<GasToolSettings>()
            .init_resource::<SourceStructureToolSettings>()
            .init_resource::<SinkStructureToolSettings>()
            .init_resource::<StructureEditState>()
            .init_resource::<SelectionDragState>()
            .init_resource::<BrushDragState>()
            .add_systems(Startup, (setup_editor_ui, setup_editor_overlays))
            .add_systems(Update, handle_escape_and_main_menu)
            .add_systems(Update, handle_main_menu_actions)
            .add_systems(Update, handle_editor_ui_actions)
            .add_systems(Update, refresh_editor_ui)
            .add_systems(Update, refresh_main_menu_ui)
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

fn setup_editor_ui(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    gas_registry: Res<GasRegistry>,
    sim_rate: Res<SimulationRateConfig>,
    gas_simulation: Res<GasSimulationConfig>,
    gas_visual_settings: Res<GasVisualSettings>,
    mut panel_manager: ResMut<PanelManager>,
    mut panel_open_order: ResMut<PanelOpenOrder>,
    mut select_fields: ResMut<SelectFieldState>,
) {
    let fmt_f32 = |v: f32| {
        let s = format!("{:.3}", v);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    let sim_hz_initial = sim_rate.target_hz.clamp(1, 1000);
    let buoyancy_strength_initial = gas_simulation
        .solver_tuning
        .buoyancy_strength
        .clamp(0.0, 5.0);
    let buoyancy_radius_initial = u32::from(
        gas_simulation
            .solver_tuning
            .buoyancy_window_radius
            .clamp(1, 3),
    );
    let buoyancy_sigma_initial = gas_simulation
        .solver_tuning
        .buoyancy_window_sigma
        .clamp(0.5, 3.0);
    let buoyancy_gain_initial = gas_simulation.solver_tuning.buoyancy_gain.clamp(0.0, 10.0);
    let buoyancy_alpha_initial = gas_simulation.solver_tuning.buoyancy_alpha.clamp(0.0, 4.0);
    let buoyancy_cap_initial = gas_simulation
        .solver_tuning
        .buoyancy_force_cap
        .clamp(0.0, 2.0);
    let gamma_initial = gas_visual_settings.gamma.clamp(0.0, 10.0);
    let max_color_initial = gas_visual_settings
        .max_particles_for_max_color
        .clamp(1, 10_000);
    let gas_select_options: Vec<String> = gas_registry
        .all()
        .iter()
        .map(|gas| gas.id.to_uppercase())
        .collect();
    let sim_hz_initial_text = sim_hz_initial.to_string();
    let buoyancy_strength_initial_text = fmt_f32(buoyancy_strength_initial);
    let buoyancy_radius_initial_text = buoyancy_radius_initial.to_string();
    let buoyancy_sigma_initial_text = fmt_f32(buoyancy_sigma_initial);
    let buoyancy_gain_initial_text = fmt_f32(buoyancy_gain_initial);
    let buoyancy_alpha_initial_text = fmt_f32(buoyancy_alpha_initial);
    let buoyancy_cap_initial_text = fmt_f32(buoyancy_cap_initial);
    let gamma_initial_text = fmt_f32(gamma_initial);
    let max_color_initial_text = max_color_initial.to_string();
    let icon_set = EditorIconSet {
        build: asset_server.load("sprites/ui/tool_build.png"),
        erase: asset_server.load("sprites/ui/tool_erase.png"),
        add_gas: asset_server.load("sprites/ui/tool_add_gas.png"),
        clear_gas: asset_server.load("sprites/ui/tool_clear_gas.png"),
        source: asset_server.load("sprites/ui/tool_gas_source.png"),
        sink: asset_server.load("sprites/ui/tool_gas_sink.png"),
        brick: asset_server.load("sprites/ui/tool_brick.png"),
        metal: asset_server.load("sprites/ui/tool_metal.png"),
        brick_silhouette: asset_server.load("sprites/ui/silhouette_brick.png"),
        metal_silhouette: asset_server.load("sprites/ui/silhouette_metal.png"),
        source_silhouette: asset_server.load("sprites/world/tile_gas_source.png"),
        sink_silhouette: asset_server.load("sprites/world/tile_gas_sink.png"),
        select_arrow: asset_server.load("sprites/ui/select_arrow.png"),
    };
    commands.insert_resource(icon_set.clone());

    select_fields.register_field(SelectFieldConfig {
        id: GAS_SELECT_ADD_ID,
        options: gas_select_options.clone(),
        selected: 0,
    });
    select_fields.register_field(SelectFieldConfig {
        id: GAS_SELECT_SOURCE_ID,
        options: gas_select_options.clone(),
        selected: 0,
    });

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
            MainToolbarRoot,
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
            spawn_tool_button(
                parent,
                "Create Gas Source",
                EditorTool::CreateGasSource,
                icon_set.source.clone(),
            );
            spawn_tool_button(
                parent,
                "Create Gas Sink",
                EditorTool::CreateGasSink,
                icon_set.sink.clone(),
            );
        });

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: DEBUG_PANEL_ID,
            title: "Debug Panel".to_string(),
            corner: PanelCorner::TopRight,
            width: DEBUG_PANEL_WIDTH,
            margin_x: DEBUG_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::Never,
            background: PANEL_BG,
            header_background: Color::srgba(0.82, 0.84, 0.87, 0.98),
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_debug_panel_content(
                parent,
                sim_hz_initial,
                sim_hz_initial_text.clone(),
                buoyancy_strength_initial,
                buoyancy_strength_initial_text.clone(),
                buoyancy_radius_initial,
                buoyancy_radius_initial_text.clone(),
                buoyancy_sigma_initial,
                buoyancy_sigma_initial_text.clone(),
                buoyancy_gain_initial,
                buoyancy_gain_initial_text.clone(),
                buoyancy_alpha_initial,
                buoyancy_alpha_initial_text.clone(),
                buoyancy_cap_initial,
                buoyancy_cap_initial_text.clone(),
                gamma_initial,
                gamma_initial_text.clone(),
                max_color_initial,
                max_color_initial_text.clone(),
            );
        },
    );

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: GAS_TOOL_PANEL_ID,
            title: "Gas Panel".to_string(),
            corner: PanelCorner::TopRight,
            width: GAS_PANEL_WIDTH,
            margin_x: GAS_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::AutoHalfScreen,
            background: PANEL_BG,
            header_background: Color::srgba(0.82, 0.84, 0.87, 0.98),
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_gas_tool_panel_content(parent, &gas_select_options, icon_set.select_arrow.clone());
        },
    );

    panel_manager.spawn_panel(
        &mut commands,
        &mut panel_open_order,
        PanelSpec {
            id: STRUCTURE_TOOL_PANEL_ID,
            title: "Structure Panel".to_string(),
            corner: PanelCorner::TopRight,
            width: STRUCTURE_PANEL_WIDTH,
            margin_x: STRUCTURE_PANEL_RIGHT,
            margin_y: DEBUG_PANEL_TOP,
            stack_gap: DEBUG_AND_GAS_PANEL_GAP,
            controls: PanelControls {
                show_collapse: true,
                show_close: false,
                custom_actions: Vec::new(),
            },
            scroll_policy: PanelScrollPolicy::Never,
            background: PANEL_BG,
            header_background: Color::srgba(0.82, 0.84, 0.87, 0.98),
            initial_visible: false,
            initial_collapsed: false,
        },
        |parent| {
            spawn_structure_tool_panel_content(
                parent,
                &gas_select_options,
                icon_set.select_arrow.clone(),
            );
        },
    );

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
                        width: Val::Px(780.0),
                        height: Val::Px(640.0),
                        display: Display::Flex,
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        align_items: AlignItems::Center,
                        row_gap: Val::Px(10.0),
                        padding: UiRect::all(Val::Px(18.0)),
                        ..default()
                    },
                    BackgroundColor(MODAL_BG),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Text::new("Main Menu"),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::srgba(0.08, 0.09, 0.11, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                        MainMenuTitleText,
                    ));
                    panel.spawn((
                        Node {
                            width: Val::Percent(100.0),
                            justify_content: JustifyContent::Center,
                            ..default()
                        },
                        Text::new(""),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgba(0.20, 0.22, 0.26, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                        MainMenuStatusText,
                    ));

                    panel
                        .spawn((
                            Node {
                                display: Display::Flex,
                                flex_direction: FlexDirection::Column,
                                width: Val::Percent(100.0),
                                align_items: AlignItems::Center,
                                row_gap: Val::Px(8.0),
                                ..default()
                            },
                            MainMenuRootActions,
                        ))
                        .with_children(|actions| {
                            spawn_main_menu_action_button(
                                actions,
                                "Continue",
                                MainMenuButtonAction::Continue,
                                220.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "New Game",
                                MainMenuButtonAction::NewGame,
                                220.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "Save",
                                MainMenuButtonAction::OpenSaveScreen,
                                220.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "Load",
                                MainMenuButtonAction::OpenLoadScreen,
                                220.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "Exit To Main",
                                MainMenuButtonAction::ExitToMainMenu,
                                220.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "Exit",
                                MainMenuButtonAction::ExitApp,
                                220.0,
                            );
                        });

                    panel
                        .spawn((
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            MainMenuSaveNameRow,
                        ))
                        .with_children(|row| {
                            row.spawn((
                                Text::new("Save Name:"),
                                TextFont::from_font_size(14.0),
                                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                                TextLayout::new_with_justify(JustifyText::Center),
                            ));
                            row.spawn((
                                Button,
                                Node {
                                    min_width: Val::Px(520.0),
                                    height: Val::Px(34.0),
                                    justify_content: JustifyContent::FlexStart,
                                    align_items: AlignItems::Center,
                                    padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                                    ..default()
                                },
                                BackgroundColor(BUTTON_IDLE),
                                TextInputField::new_string(
                                    "New Save",
                                    64,
                                    crate::ui::input_field::InputAllowedChars::Any,
                                ),
                                TextInputStyle {
                                    idle_bg: BUTTON_IDLE,
                                    focused_bg: INPUT_FOCUSED,
                                },
                                bevy::ui::RelativeCursorPosition::default(),
                                MainMenuSaveNameInputField,
                            ))
                            .with_children(|button| {
                                button.spawn((
                                    Text::new("New Save"),
                                    TextFont::from_font_size(14.0),
                                    TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                                    TextInputDisplay,
                                ));
                            });
                        });

                    panel
                        .spawn((
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            MainMenuSaveActions,
                        ))
                        .with_children(|actions| {
                            spawn_main_menu_action_button(
                                actions,
                                "Back",
                                MainMenuButtonAction::BackToRoot,
                                140.0,
                            );
                            spawn_main_menu_action_button(
                                actions,
                                "Create New Save",
                                MainMenuButtonAction::CreateNewSave,
                                220.0,
                            );
                        });

                    panel
                        .spawn((
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            MainMenuLoadActions,
                        ))
                        .with_children(|actions| {
                            spawn_main_menu_action_button(
                                actions,
                                "Back",
                                MainMenuButtonAction::BackToRoot,
                                140.0,
                            );
                        });

                    panel
                        .spawn((
                            Node {
                                display: Display::None,
                                flex_direction: FlexDirection::Row,
                                width: Val::Percent(100.0),
                                justify_content: JustifyContent::Center,
                                column_gap: Val::Px(8.0),
                                ..default()
                            },
                            MainMenuConfirmActions,
                        ))
                        .with_children(|actions| {
                            actions
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(38.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(MODAL_BUTTON_BG),
                                    MainMenuActionButton(MainMenuButtonAction::ConfirmPrimary),
                                    MainMenuConfirmPrimaryLabel,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Yes"),
                                        TextFont::from_font_size(15.0),
                                        TextColor(Color::WHITE),
                                        TextLayout::new_with_justify(JustifyText::Center),
                                    ));
                                });
                            actions
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(38.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(MODAL_BUTTON_BG),
                                    MainMenuActionButton(MainMenuButtonAction::ConfirmSecondary),
                                    MainMenuConfirmSecondaryLabel,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("No"),
                                        TextFont::from_font_size(15.0),
                                        TextColor(Color::WHITE),
                                        TextLayout::new_with_justify(JustifyText::Center),
                                    ));
                                });
                            actions
                                .spawn((
                                    Button,
                                    Node {
                                        width: Val::Px(160.0),
                                        height: Val::Px(38.0),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    BackgroundColor(MODAL_BUTTON_BG),
                                    MainMenuActionButton(MainMenuButtonAction::ConfirmCancel),
                                    MainMenuConfirmCancelLabel,
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Cancel"),
                                        TextFont::from_font_size(15.0),
                                        TextColor(Color::WHITE),
                                        TextLayout::new_with_justify(JustifyText::Center),
                                    ));
                                });
                        });

                    panel.spawn((
                        Node {
                            display: Display::None,
                            flex_direction: FlexDirection::Column,
                            width: Val::Percent(100.0),
                            align_items: AlignItems::Center,
                            row_gap: Val::Px(6.0),
                            height: Val::Px(350.0),
                            overflow: Overflow::clip_y(),
                            ..default()
                        },
                        MainMenuSaveListRoot,
                    ));
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

#[allow(clippy::too_many_arguments)]
fn spawn_debug_panel_content(
    parent: &mut ChildSpawnerCommands,
    sim_hz_initial: u32,
    sim_hz_initial_text: String,
    buoyancy_strength_initial: f32,
    buoyancy_strength_initial_text: String,
    buoyancy_radius_initial: u32,
    buoyancy_radius_initial_text: String,
    buoyancy_sigma_initial: f32,
    buoyancy_sigma_initial_text: String,
    buoyancy_gain_initial: f32,
    buoyancy_gain_initial_text: String,
    buoyancy_alpha_initial: f32,
    buoyancy_alpha_initial_text: String,
    buoyancy_cap_initial: f32,
    buoyancy_cap_initial_text: String,
    gamma_initial: f32,
    gamma_initial_text: String,
    max_color_initial: u32,
    max_color_initial_text: String,
) {
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
                TextColor(DEBUG_PANEL_TEXT_COLOR),
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
                TextColor(DEBUG_PANEL_TEXT_COLOR),
            ));
        });

    parent.spawn((
        Text::new(
            "Iterations: 0 | Step ms: 0.000 | avg: 0.000 | Target Hz: 30 x 1 = 30 | Actual Hz: 0",
        ),
        TextFont::from_font_size(13.0),
        TextColor(DEBUG_PANEL_TEXT_COLOR),
        SimulationPerfLabel,
    ));

    parent.spawn((
        Text::new(
            "Anisotropy: 0.0000 | Radial waves: 0.0000 | Mass err H2/O2/CO2: 0.0000 / 0.0000 / 0.0000",
        ),
        TextFont::from_font_size(13.0),
        TextColor(DEBUG_PANEL_TEXT_COLOR),
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
                TextColor(DEBUG_PANEL_TEXT_COLOR),
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
                TextInputField::new_u32(sim_hz_initial, 1, 1000, 4),
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                SimulationHzInputField,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(sim_hz_initial_text.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(DEBUG_PANEL_TEXT_COLOR),
                    TextInputDisplay,
                ));
            });
        });

    spawn_debug_f32_row(
        parent,
        "Buoyancy strength:",
        buoyancy_strength_initial,
        buoyancy_strength_initial_text,
        TextInputField::new_f32(buoyancy_strength_initial, 0.0, 5.0, 6, 3),
        BuoyancyStrengthInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_u32_row(
        parent,
        "Buoyancy radius:",
        buoyancy_radius_initial,
        buoyancy_radius_initial_text,
        TextInputField::new_u32(buoyancy_radius_initial, 1, 3, 1),
        BuoyancyWindowRadiusInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy sigma:",
        buoyancy_sigma_initial,
        buoyancy_sigma_initial_text,
        TextInputField::new_f32(buoyancy_sigma_initial, 0.5, 3.0, 6, 3),
        BuoyancyWindowSigmaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy gain:",
        buoyancy_gain_initial,
        buoyancy_gain_initial_text,
        TextInputField::new_f32(buoyancy_gain_initial, 0.0, 10.0, 6, 3),
        BuoyancyGainInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy alpha:",
        buoyancy_alpha_initial,
        buoyancy_alpha_initial_text,
        TextInputField::new_f32(buoyancy_alpha_initial, 0.0, 4.0, 6, 3),
        BuoyancyAlphaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Buoyancy cap:",
        buoyancy_cap_initial,
        buoyancy_cap_initial_text,
        TextInputField::new_f32(buoyancy_cap_initial, 0.0, 2.0, 6, 3),
        BuoyancyForceCapInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_f32_row(
        parent,
        "Gamma:",
        gamma_initial,
        gamma_initial_text,
        TextInputField::new_f32(gamma_initial, 0.0, 10.0, 6, 3),
        GasGammaInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
    spawn_debug_u32_row(
        parent,
        "Max color at:",
        max_color_initial,
        max_color_initial_text,
        TextInputField::new_u32(max_color_initial, 1, 10_000, 5),
        GasMaxColorParticlesInputField,
        DEBUG_PANEL_TEXT_COLOR,
    );
}

fn spawn_debug_f32_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    _initial: f32,
    initial_text: String,
    field: TextInputField,
    marker: M,
    label_color: Color,
) {
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
                Text::new(label),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
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
                field,
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                marker,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(initial_text.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(label_color),
                    TextInputDisplay,
                ));
            });
        });
}

fn spawn_debug_u32_row<M: Component>(
    parent: &mut ChildSpawnerCommands,
    label: &'static str,
    _initial: u32,
    initial_text: String,
    field: TextInputField,
    marker: M,
    label_color: Color,
) {
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
                Text::new(label),
                TextFont::from_font_size(13.0),
                TextColor(label_color),
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
                field,
                TextInputStyle {
                    idle_bg: BUTTON_IDLE,
                    focused_bg: INPUT_FOCUSED,
                },
                bevy::ui::RelativeCursorPosition::default(),
                marker,
            ))
            .with_children(|button| {
                button.spawn((
                    Text::new(initial_text.clone()),
                    TextFont::from_font_size(13.0),
                    TextColor(label_color),
                    TextInputDisplay,
                ));
            });
        });
}

fn spawn_gas_tool_panel_content(
    parent: &mut ChildSpawnerCommands,
    gas_options: &[String],
    select_arrow: Handle<Image>,
) {
    spawn_select_field(
        parent,
        &SelectFieldConfig {
            id: GAS_SELECT_ADD_ID,
            options: gas_options.to_vec(),
            selected: 0,
        },
        "Gas:",
        select_arrow,
    );

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
}

fn spawn_structure_tool_panel_content(
    parent: &mut ChildSpawnerCommands,
    gas_options: &[String],
    select_arrow: Handle<Image>,
) {
    parent.spawn((
        Text::new("No structure selected"),
        TextFont::from_font_size(13.0),
        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
        StructureModeLabel,
    ));

    parent
        .spawn((
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            StructureSourceSection,
        ))
        .with_children(|source| {
            spawn_select_field(
                source,
                &SelectFieldConfig {
                    id: GAS_SELECT_SOURCE_ID,
                    options: gas_options.to_vec(),
                    selected: 0,
                },
                "Source gas:",
                select_arrow,
            );
            source
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
                        SourceAmountInputField,
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
        });

    parent
        .spawn((
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(8.0),
                ..default()
            },
            StructureSinkSection,
        ))
        .with_children(|sink| {
            sink.spawn((
                Text::new("Sink amount per step"),
                TextFont::from_font_size(13.0),
                TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
            ));
            sink.spawn((
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
                SinkAmountInputField,
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

fn spawn_main_menu_action_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    action: MainMenuButtonAction,
    width: f32,
) -> Entity {
    parent
        .spawn((
            Button,
            Node {
                width: Val::Px(width),
                height: Val::Px(38.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(MODAL_BUTTON_BG),
            MainMenuActionButton(action),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(label.to_string()),
                TextFont::from_font_size(15.0),
                TextColor(Color::WHITE),
                TextLayout::new_with_justify(JustifyText::Center),
            ));
        })
        .id()
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

fn escape_action(
    menu_mode: MainMenuMode,
    has_selected_tool: bool,
    has_structure_editor: bool,
    has_world: bool,
) -> EscAction {
    match menu_mode {
        MainMenuMode::Main => EscAction::Ignore,
        MainMenuMode::InGame => EscAction::CloseMenuKeepPaused,
        MainMenuMode::Hidden => {
            if !has_world {
                EscAction::Ignore
            } else if has_structure_editor {
                EscAction::CloseStructureEditor
            } else if has_selected_tool {
                EscAction::ClearSelectedTool
            } else {
                EscAction::OpenMenuAndPause
            }
        }
    }
}

fn handle_escape_and_main_menu(
    keys: Res<ButtonInput<KeyCode>>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut main_menu: ResMut<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    world_load_state: Res<WorldLoadState>,
    mut structure_edit: ResMut<StructureEditState>,
    mut control: ResMut<crate::simulation::SimulationControl>,
    mut input_fields: Query<&mut TextInputField>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
) {
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

    match escape_action(
        menu_ui.mode,
        active_tool.selected.is_some(),
        structure_edit.selected_cell.is_some(),
        world_load_state.has_world,
    ) {
        EscAction::CloseMenuKeepPaused => {
            menu_ui.mode = MainMenuMode::Hidden;
            menu_ui.screen = MainMenuScreen::Root;
            main_menu.open = false;
        }
        EscAction::CloseStructureEditor => {
            structure_edit.selected_cell = None;
        }
        EscAction::ClearSelectedTool => {
            active_tool.selected = None;
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
        }
        EscAction::OpenMenuAndPause => {
            control.paused = true;
            menu_ui.mode = MainMenuMode::InGame;
            menu_ui.screen = MainMenuScreen::Root;
            main_menu.open = true;
        }
        EscAction::Ignore => {}
    }
}

fn handle_main_menu_actions(
    mut interactions: Query<
        (&Interaction, &MainMenuActionButton, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
    mut main_menu: ResMut<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    mut control: ResMut<SimulationControl>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<GasStructureGrid>,
    mut gas: ResMut<GasField>,
    mut step: ResMut<SimulationStep>,
    mut save_session: ResMut<SaveSessionState>,
    mut world_load_state: ResMut<WorldLoadState>,
    save_name_input: Single<&TextInputField, With<MainMenuSaveNameInputField>>,
    mut world_changed: EventWriter<WorldCellChanged>,
    mut exit_writer: EventWriter<AppExit>,
) {
    if !main_menu.open {
        return;
    }

    for (interaction, action_button, mut bg) in &mut interactions {
        match *interaction {
            Interaction::Pressed | Interaction::Hovered => bg.0 = MODAL_BUTTON_HOVER,
            Interaction::None => bg.0 = MODAL_BUTTON_BG,
        }
        if *interaction != Interaction::Pressed {
            continue;
        }

        match &action_button.0 {
            MainMenuButtonAction::Continue => {
                if menu_ui.mode != MainMenuMode::InGame || !world_load_state.has_world {
                    continue;
                }
                menu_ui.mode = MainMenuMode::Hidden;
                menu_ui.screen = MainMenuScreen::Root;
                menu_ui.confirm_state = None;
                menu_ui.post_save_action = None;
                main_menu.open = false;
                menu_ui.status_text.clear();
            }
            MainMenuButtonAction::NewGame => {
                let state = new_game_snapshot(&gas_registry);
                match apply_runtime_world_state(
                    state,
                    &mut world,
                    &mut structures,
                    &mut gas,
                    &mut step,
                    &mut world_changed,
                ) {
                    Ok(_) => {
                        control.paused = true;
                        save_session.mark_persisted(step.0, None);
                        world_load_state.has_world = true;
                        menu_ui.mode = MainMenuMode::Hidden;
                        menu_ui.screen = MainMenuScreen::Root;
                        menu_ui.confirm_state = None;
                        menu_ui.post_save_action = None;
                        main_menu.open = false;
                        menu_ui.status_text.clear();
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("New game failed: {}", err);
                    }
                }
            }
            MainMenuButtonAction::OpenSaveScreen => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                menu_ui.screen = MainMenuScreen::Save;
                refresh_saves_cache(&mut menu_ui);
            }
            MainMenuButtonAction::OpenLoadScreen => {
                menu_ui.screen = MainMenuScreen::Load;
                refresh_saves_cache(&mut menu_ui);
            }
            MainMenuButtonAction::ExitToMainMenu => {
                if menu_ui.mode != MainMenuMode::InGame {
                    continue;
                }
                if save_session.has_unsaved_changes(step.0) {
                    menu_ui.return_screen = MainMenuScreen::Root;
                    menu_ui.confirm_state = Some(MainMenuConfirmState::UnsavedChanges(
                        MainMenuDeferredAction::ExitToMainMenu,
                    ));
                    menu_ui.confirm_text = "Save changes before exiting to main menu?".to_string();
                    menu_ui.screen = MainMenuScreen::Confirm;
                } else {
                    let state = new_game_snapshot(&gas_registry);
                    match apply_runtime_world_state(
                        state,
                        &mut world,
                        &mut structures,
                        &mut gas,
                        &mut step,
                        &mut world_changed,
                    ) {
                        Ok(_) => {
                            control.paused = true;
                            save_session.mark_persisted(step.0, None);
                            world_load_state.has_world = false;
                            menu_ui.mode = MainMenuMode::Main;
                            menu_ui.screen = MainMenuScreen::Root;
                            menu_ui.confirm_state = None;
                            menu_ui.post_save_action = None;
                            main_menu.open = true;
                            menu_ui.status_text.clear();
                        }
                        Err(err) => {
                            menu_ui.status_text = format!("Exit to main failed: {}", err);
                        }
                    }
                }
            }
            MainMenuButtonAction::ExitApp => {
                if world_load_state.has_world && save_session.has_unsaved_changes(step.0) {
                    menu_ui.return_screen = MainMenuScreen::Root;
                    menu_ui.confirm_state = Some(MainMenuConfirmState::UnsavedChanges(
                        MainMenuDeferredAction::ExitApp,
                    ));
                    menu_ui.confirm_text = "Save changes before exiting the game?".to_string();
                    menu_ui.screen = MainMenuScreen::Confirm;
                } else {
                    exit_writer.write(AppExit::Success);
                }
            }
            MainMenuButtonAction::BackToRoot => {
                menu_ui.screen = MainMenuScreen::Root;
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                menu_ui.post_save_action = None;
                menu_ui.status_text.clear();
            }
            MainMenuButtonAction::CreateNewSave => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                let name = save_name_input.text.clone();
                match create_save(
                    &saves_root_default(),
                    &name,
                    &world,
                    &gas,
                    &structures,
                    &gas_registry,
                    step.0,
                ) {
                    Ok(descriptor) => {
                        save_session.mark_persisted(step.0, Some(descriptor.id.clone()));
                        refresh_saves_cache(&mut menu_ui);
                        if let Some(post_action) = menu_ui.post_save_action.take() {
                            match post_action {
                                MainMenuDeferredAction::ExitToMainMenu => {
                                    let state = new_game_snapshot(&gas_registry);
                                    match apply_runtime_world_state(
                                        state,
                                        &mut world,
                                        &mut structures,
                                        &mut gas,
                                        &mut step,
                                        &mut world_changed,
                                    ) {
                                        Ok(_) => {
                                            control.paused = true;
                                            save_session.mark_persisted(step.0, None);
                                            world_load_state.has_world = false;
                                            menu_ui.mode = MainMenuMode::Main;
                                            menu_ui.screen = MainMenuScreen::Root;
                                            main_menu.open = true;
                                            menu_ui.status_text.clear();
                                        }
                                        Err(err) => {
                                            menu_ui.status_text =
                                                format!("Exit to main failed: {}", err);
                                            menu_ui.screen = MainMenuScreen::Save;
                                        }
                                    }
                                }
                                MainMenuDeferredAction::ExitApp => {
                                    exit_writer.write(AppExit::Success);
                                }
                            }
                        } else {
                            menu_ui.status_text = format!("Saved '{}'.", descriptor.display_name);
                        }
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("Create save failed: {}", err);
                    }
                }
            }
            MainMenuButtonAction::SelectOverwrite(save_id) => {
                if !world_load_state.has_world {
                    menu_ui.status_text =
                        "No world loaded. Start or load a world before saving.".to_string();
                    continue;
                }
                menu_ui.return_screen = MainMenuScreen::Save;
                menu_ui.confirm_state = Some(MainMenuConfirmState::OverwriteSave(save_id.clone()));
                menu_ui.confirm_text =
                    "This save slot will be fully overwritten. Continue?".to_string();
                menu_ui.screen = MainMenuScreen::Confirm;
            }
            MainMenuButtonAction::SelectLoad(save_id) => {
                match load_save(&saves_root_default(), save_id, &gas_registry) {
                    Ok(loaded) => {
                        match apply_runtime_world_state(
                            loaded.state,
                            &mut world,
                            &mut structures,
                            &mut gas,
                            &mut step,
                            &mut world_changed,
                        ) {
                            Ok(_) => {
                                control.paused = true;
                                save_session.mark_persisted(step.0, Some(loaded.descriptor.id));
                                world_load_state.has_world = true;
                                menu_ui.mode = MainMenuMode::Hidden;
                                menu_ui.screen = MainMenuScreen::Root;
                                menu_ui.confirm_state = None;
                                menu_ui.post_save_action = None;
                                main_menu.open = false;
                                menu_ui.status_text.clear();
                            }
                            Err(err) => {
                                menu_ui.status_text = format!("Load apply failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Load;
                            }
                        }
                    }
                    Err(err) => {
                        menu_ui.status_text = format!("Load failed: {}", err);
                        menu_ui.screen = MainMenuScreen::Load;
                    }
                }
            }
            MainMenuButtonAction::ConfirmPrimary => {
                let confirm = menu_ui.confirm_state.clone();
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                match confirm {
                    Some(MainMenuConfirmState::OverwriteSave(save_id)) => {
                        match overwrite_save(
                            &saves_root_default(),
                            &save_id,
                            &world,
                            &gas,
                            &structures,
                            &gas_registry,
                            step.0,
                        ) {
                            Ok(descriptor) => {
                                save_session.mark_persisted(step.0, Some(descriptor.id.clone()));
                                refresh_saves_cache(&mut menu_ui);
                                if let Some(post_action) = menu_ui.post_save_action.take() {
                                    match post_action {
                                        MainMenuDeferredAction::ExitToMainMenu => {
                                            let state = new_game_snapshot(&gas_registry);
                                            match apply_runtime_world_state(
                                                state,
                                                &mut world,
                                                &mut structures,
                                                &mut gas,
                                                &mut step,
                                                &mut world_changed,
                                            ) {
                                                Ok(_) => {
                                                    control.paused = true;
                                                    save_session.mark_persisted(step.0, None);
                                                    world_load_state.has_world = false;
                                                    menu_ui.mode = MainMenuMode::Main;
                                                    menu_ui.screen = MainMenuScreen::Root;
                                                    main_menu.open = true;
                                                    menu_ui.status_text.clear();
                                                }
                                                Err(err) => {
                                                    menu_ui.status_text =
                                                        format!("Exit to main failed: {}", err);
                                                    menu_ui.screen = MainMenuScreen::Save;
                                                }
                                            }
                                        }
                                        MainMenuDeferredAction::ExitApp => {
                                            exit_writer.write(AppExit::Success);
                                        }
                                    }
                                } else {
                                    menu_ui.status_text =
                                        format!("Overwritten '{}'.", descriptor.display_name);
                                    menu_ui.screen = MainMenuScreen::Save;
                                }
                            }
                            Err(err) => {
                                menu_ui.status_text = format!("Overwrite failed: {}", err);
                                menu_ui.screen = MainMenuScreen::Save;
                            }
                        }
                    }
                    Some(MainMenuConfirmState::UnsavedChanges(action)) => {
                        menu_ui.post_save_action = Some(action);
                        menu_ui.screen = MainMenuScreen::Save;
                        refresh_saves_cache(&mut menu_ui);
                    }
                    None => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                }
            }
            MainMenuButtonAction::ConfirmSecondary => {
                let confirm = menu_ui.confirm_state.clone();
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                match confirm {
                    Some(MainMenuConfirmState::OverwriteSave(_)) => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                    Some(MainMenuConfirmState::UnsavedChanges(action)) => match action {
                        MainMenuDeferredAction::ExitToMainMenu => {
                            let state = new_game_snapshot(&gas_registry);
                            match apply_runtime_world_state(
                                state,
                                &mut world,
                                &mut structures,
                                &mut gas,
                                &mut step,
                                &mut world_changed,
                            ) {
                                Ok(_) => {
                                    control.paused = true;
                                    save_session.mark_persisted(step.0, None);
                                    world_load_state.has_world = false;
                                    menu_ui.mode = MainMenuMode::Main;
                                    menu_ui.screen = MainMenuScreen::Root;
                                    menu_ui.post_save_action = None;
                                    main_menu.open = true;
                                    menu_ui.status_text.clear();
                                }
                                Err(err) => {
                                    menu_ui.status_text = format!("Exit to main failed: {}", err);
                                    menu_ui.screen = MainMenuScreen::Root;
                                }
                            }
                        }
                        MainMenuDeferredAction::ExitApp => {
                            exit_writer.write(AppExit::Success);
                        }
                    },
                    None => {
                        menu_ui.screen = menu_ui.return_screen;
                    }
                }
            }
            MainMenuButtonAction::ConfirmCancel => {
                menu_ui.confirm_state = None;
                menu_ui.confirm_text.clear();
                menu_ui.post_save_action = None;
                menu_ui.screen = menu_ui.return_screen;
            }
        }
    }
}

fn refresh_saves_cache(menu_ui: &mut MainMenuUiState) {
    match list_saves(&saves_root_default()) {
        Ok(saves) => {
            menu_ui.saves = saves;
            menu_ui.needs_save_list_refresh = true;
        }
        Err(err) => {
            menu_ui.saves.clear();
            menu_ui.status_text = format!("Failed to read save list: {}", err);
            menu_ui.needs_save_list_refresh = true;
        }
    }
}

fn apply_runtime_world_state(
    state: crate::save::RuntimeWorldState,
    world: &mut WorldGrid,
    structures: &mut GasStructureGrid,
    gas: &mut GasField,
    step: &mut SimulationStep,
    world_changed: &mut EventWriter<WorldCellChanged>,
) -> Result<(), String> {
    world.restore_from_cell_codes(&state.world_cell_codes)?;
    structures.restore_state(&state.gas_structures_snapshot, world)?;
    gas.restore_state(&state.gas_snapshot)?;
    step.0 = state.simulation_step;
    emit_full_world_changed(world_changed);
    Ok(())
}

fn emit_full_world_changed(world_changed: &mut EventWriter<WorldCellChanged>) {
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            world_changed.write(WorldCellChanged {
                cell: UVec2::new(x, y),
            });
        }
    }
}

fn refresh_main_menu_ui(
    mut commands: Commands,
    main_menu: Res<MainMenuState>,
    mut menu_ui: ResMut<MainMenuUiState>,
    mut root_visibility: Single<&mut Visibility, With<MainMenuRoot>>,
    mut text_set: ParamSet<(
        Single<&mut Text, With<MainMenuTitleText>>,
        Single<&mut Text, With<MainMenuStatusText>>,
        Query<&mut Text>,
    )>,
    mut node_set: ParamSet<(
        Single<&mut Node, With<MainMenuRootActions>>,
        Single<&mut Node, With<MainMenuSaveActions>>,
        Single<&mut Node, With<MainMenuLoadActions>>,
        Single<&mut Node, With<MainMenuConfirmActions>>,
        Single<&mut Node, With<MainMenuSaveNameRow>>,
        Single<&mut Node, With<MainMenuSaveListRoot>>,
        Query<(&MainMenuActionButton, &mut Node), With<Button>>,
    )>,
    confirm_button_set: (
        Single<&Children, With<MainMenuConfirmPrimaryLabel>>,
        Single<&Children, With<MainMenuConfirmSecondaryLabel>>,
        Single<&Children, With<MainMenuConfirmCancelLabel>>,
    ),
    save_list_root: Single<Entity, With<MainMenuSaveListRoot>>,
) {
    **root_visibility = if main_menu.open {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if !main_menu.open {
        return;
    }

    let screen = menu_ui.screen;
    let mode = menu_ui.mode;
    {
        let mut title_text = text_set.p0();
        title_text.0 = match screen {
            MainMenuScreen::Root => match mode {
                MainMenuMode::Main => "Main Menu".to_string(),
                MainMenuMode::InGame => "Game Menu".to_string(),
                MainMenuMode::Hidden => "Menu".to_string(),
            },
            MainMenuScreen::Save => "Save World".to_string(),
            MainMenuScreen::Load => "Load World".to_string(),
            MainMenuScreen::Confirm => "Confirm Action".to_string(),
        };
    }

    let status = if screen == MainMenuScreen::Confirm {
        menu_ui.confirm_text.clone()
    } else if !menu_ui.status_text.is_empty() {
        menu_ui.status_text.clone()
    } else {
        String::new()
    };
    {
        let mut status_text = text_set.p1();
        status_text.0 = status;
    }

    {
        let mut root_actions_visibility = node_set.p0();
        root_actions_visibility.display = if screen == MainMenuScreen::Root {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (action_button, mut node) in &mut node_set.p6() {
        node.display = match screen {
            MainMenuScreen::Root => match (&action_button.0, mode) {
                (_, MainMenuMode::Hidden) => Display::None,
                (MainMenuButtonAction::Continue, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::OpenSaveScreen, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitToMainMenu, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::InGame) => Display::Flex,
                (MainMenuButtonAction::NewGame, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::OpenLoadScreen, MainMenuMode::Main) => Display::Flex,
                (MainMenuButtonAction::ExitApp, MainMenuMode::Main) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Save => match action_button.0 {
                MainMenuButtonAction::BackToRoot
                | MainMenuButtonAction::CreateNewSave
                | MainMenuButtonAction::SelectOverwrite(_) => Display::Flex,
                _ => Display::None,
            },
            MainMenuScreen::Load => match action_button.0 {
                MainMenuButtonAction::BackToRoot | MainMenuButtonAction::SelectLoad(_) => {
                    Display::Flex
                }
                _ => Display::None,
            },
            MainMenuScreen::Confirm => match action_button.0 {
                MainMenuButtonAction::ConfirmPrimary
                | MainMenuButtonAction::ConfirmSecondary
                | MainMenuButtonAction::ConfirmCancel => Display::Flex,
                _ => Display::None,
            },
        };
    }
    {
        let mut save_actions_visibility = node_set.p1();
        save_actions_visibility.display = if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut load_actions_visibility = node_set.p2();
        load_actions_visibility.display = if screen == MainMenuScreen::Load {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut confirm_actions_visibility = node_set.p3();
        confirm_actions_visibility.display = if screen == MainMenuScreen::Confirm {
            Display::Flex
        } else {
            Display::None
        };
    }

    if screen == MainMenuScreen::Confirm {
        let mut primary_label = "Yes";
        let mut secondary_label = "No";
        let mut show_cancel = false;
        match menu_ui.confirm_state.as_ref() {
            Some(MainMenuConfirmState::UnsavedChanges(_)) => {
                primary_label = "Save";
                secondary_label = "Don't Save";
                show_cancel = true;
            }
            Some(MainMenuConfirmState::OverwriteSave(_)) => {
                primary_label = "Overwrite";
                secondary_label = "Cancel";
                show_cancel = false;
            }
            None => {}
        }

        let primary_text_entity = confirm_button_set.0.iter().next();
        if let Some(entity) = primary_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = primary_label.to_string();
            }
        }
        let secondary_text_entity = confirm_button_set.1.iter().next();
        if let Some(entity) = secondary_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = secondary_label.to_string();
            }
        }
        let cancel_text_entity = confirm_button_set.2.iter().next();
        if let Some(entity) = cancel_text_entity {
            if let Ok(mut text) = text_set.p2().get_mut(entity) {
                text.0 = "Cancel".to_string();
            }
        }

        for (action_button, mut node) in &mut node_set.p6() {
            if matches!(action_button.0, MainMenuButtonAction::ConfirmCancel) {
                node.display = if show_cancel {
                    Display::Flex
                } else {
                    Display::None
                };
            }
        }
    }
    {
        let mut save_name_row_visibility = node_set.p4();
        save_name_row_visibility.display = if screen == MainMenuScreen::Save {
            Display::Flex
        } else {
            Display::None
        };
    }
    {
        let mut save_list_visibility = node_set.p5();
        save_list_visibility.display =
            if matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
                Display::Flex
            } else {
                Display::None
            };
    }

    if !matches!(screen, MainMenuScreen::Save | MainMenuScreen::Load) {
        return;
    }
    if !menu_ui.needs_save_list_refresh {
        return;
    }
    menu_ui.needs_save_list_refresh = false;

    for entity in menu_ui.list_item_entities.drain(..) {
        commands.entity(entity).despawn();
    }

    let saves = menu_ui.saves.clone();
    let mut created = Vec::new();
    let screen_for_buttons = screen;
    commands.entity(*save_list_root).with_children(|parent| {
        if saves.is_empty() {
            let row = parent
                .spawn((
                    Node {
                        width: Val::Px(730.0),
                        height: Val::Px(32.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|row| {
                    row.spawn((
                        Text::new("No saves found."),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgba(0.15, 0.16, 0.18, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(row);
            return;
        }

        for descriptor in saves {
            let action = match screen_for_buttons {
                MainMenuScreen::Save => {
                    MainMenuButtonAction::SelectOverwrite(descriptor.id.clone())
                }
                MainMenuScreen::Load => MainMenuButtonAction::SelectLoad(descriptor.id.clone()),
                _ => continue,
            };

            let label = format!(
                "{}  |  id={}  |  updated={}",
                descriptor.display_name, descriptor.id, descriptor.updated_at_unix_ms
            );
            let entity = parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(730.0),
                        height: Val::Px(34.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(0.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON_IDLE),
                    MainMenuActionButton(action),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new(label),
                        TextFont::from_font_size(13.0),
                        TextColor(Color::srgba(0.10, 0.10, 0.12, 1.0)),
                        TextLayout::new_with_justify(JustifyText::Center),
                    ));
                })
                .id();
            created.push(entity);
        }
    });
    menu_ui.list_item_entities = created;
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
    mut structure_edit: ResMut<StructureEditState>,
    mut select_fields: ResMut<SelectFieldState>,
    mut input_set: ParamSet<(
        Single<&mut TextInputField, With<GasAmountInputField>>,
        Single<&mut TextInputField, With<SourceAmountInputField>>,
        Single<&mut TextInputField, With<SinkAmountInputField>>,
        Single<&mut TextInputField, With<GasGammaInputField>>,
        Single<&mut TextInputField, With<GasMaxColorParticlesInputField>>,
    )>,
    mut selection_drag: ResMut<SelectionDragState>,
    mut brush_drag: ResMut<BrushDragState>,
) {
    let mut unfocus_inputs = || {
        let mut gas_input = input_set.p0();
        gas_input.focused = false;
        let mut source_input = input_set.p1();
        source_input.focused = false;
        let mut sink_input = input_set.p2();
        sink_input.focused = false;
        let mut gamma_input = input_set.p3();
        gamma_input.focused = false;
        let mut max_color_particles_input = input_set.p4();
        max_color_particles_input.focused = false;
    };

    for (interaction, action) in &mut interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match *action {
            EditorUiAction::SelectTool(next_tool) => {
                active_tool.selected = Some(next_tool);
                structure_edit.selected_cell = None;
                select_fields.close_all();
                unfocus_inputs();
                clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            }
            EditorUiAction::SelectCellMaterial(next_material) => {
                unfocus_inputs();
                cell_settings.material = next_material;
            }
            EditorUiAction::ToggleReplace => {
                unfocus_inputs();
                gas_settings.replace = !gas_settings.replace;
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
        Res<WorldLoadState>,
    ),
    sim_metrics: (
        Res<DebugGasMetrics>,
        Res<SimulationControl>,
        Res<SimulationPerfStats>,
    ),
    sim_step: Res<SimulationStep>,
    mut gas_simulation: ResMut<GasSimulationConfig>,
    mut sim_rate: ResMut<SimulationRateConfig>,
    debug_overlay: Res<DebugOverlaySettings>,
    mut gas_visual_settings: ResMut<GasVisualSettings>,
    mut gas_settings: ResMut<GasToolSettings>,
    mut source_settings: ResMut<SourceStructureToolSettings>,
    mut sink_settings: ResMut<SinkStructureToolSettings>,
    mut structure_edit: ResMut<StructureEditState>,
    mut structures: ResMut<GasStructureGrid>,
    select_fields: Res<SelectFieldState>,
    world: Res<WorldGrid>,
    gas_registry: Res<GasRegistry>,
    mut ui: RefreshEditorUiSystemParams,
) {
    let (active_tool, cell_settings, main_menu, debug_mode, world_load_state) = ui_state;
    let (debug_metrics, sim_control, sim_perf) = sim_metrics;
    let selected_tool = active_tool.selected;

    if let Some(selected) = select_fields.selected_index(GAS_SELECT_ADD_ID) {
        gas_settings.gas_index = selected;
    }
    if let Some(selected) = select_fields.selected_index(GAS_SELECT_SOURCE_ID) {
        source_settings.gas_index = selected;
    }

    if let Some(amount) = ui.gas_input.parsed_u32() {
        gas_settings.amount = amount;
    }
    if let Some(amount) = ui.source_input.parsed_u32() {
        source_settings.amount = amount.max(1);
    }
    if let Some(amount) = ui.sink_input.parsed_u32() {
        sink_settings.amount = amount.max(1);
    }
    if gas_registry.count() > 0 && gas_settings.gas_index >= gas_registry.count() {
        gas_settings.gas_index = gas_registry.count() - 1;
    }
    if gas_registry.count() > 0 && source_settings.gas_index >= gas_registry.count() {
        source_settings.gas_index = gas_registry.count() - 1;
    }
    if let Some(gamma) = ui.input_set.p0().parsed_f32() {
        let next_gamma = gamma.clamp(0.0, 10.0);
        if (gas_visual_settings.gamma - next_gamma).abs() > f32::EPSILON {
            gas_visual_settings.gamma = next_gamma;
        }
    }
    if let Some(max_particles) = ui.input_set.p1().parsed_u32() {
        let next_max_particles = max_particles.clamp(1, 10_000);
        if gas_visual_settings.max_particles_for_max_color != next_max_particles {
            gas_visual_settings.max_particles_for_max_color = next_max_particles;
        }
    }
    if let Some(target_hz) = ui.input_set.p2().parsed_u32() {
        sim_rate.target_hz = target_hz.clamp(1, 1000);
    }
    if let Some(value) = ui.input_set.p3().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_strength = value.clamp(0.0, 5.0);
    }
    if let Some(value) = ui.input_set.p4().parsed_u32() {
        gas_simulation.solver_tuning.buoyancy_window_radius = value.clamp(1, 3) as u8;
    }
    if let Some(value) = ui.input_set.p5().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_window_sigma = value.clamp(0.5, 3.0);
    }
    if let Some(value) = ui.input_set.p6().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_gain = value.clamp(0.0, 10.0);
    }
    if let Some(value) = ui.input_set.p7().parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_alpha = value.clamp(0.0, 4.0);
    }
    if let Some(value) = ui.buoyancy_cap_input.parsed_f32() {
        gas_simulation.solver_tuning.buoyancy_force_cap = value.clamp(0.0, 2.0);
    }

    if let Some(cell) = structure_edit.selected_cell {
        match structures.cell(cell.x, cell.y) {
            Some(GasStructureCell::Source { .. }) => {
                let gas_index = if gas_registry.count() == 0 {
                    0
                } else {
                    source_settings.gas_index.min(gas_registry.count() - 1)
                };
                let _ = structures.update_source(
                    cell.x,
                    cell.y,
                    gas_index,
                    source_settings.amount.max(1),
                    &world,
                );
            }
            Some(GasStructureCell::Sink { .. }) => {
                let _ = structures.update_sink(cell.x, cell.y, sink_settings.amount.max(1), &world);
            }
            None => {
                structure_edit.selected_cell = None;
            }
        }
    }

    for (action, mut bg) in &mut ui.button_query {
        bg.0 = match action {
            EditorUiAction::SelectTool(action_tool) if Some(*action_tool) == selected_tool => {
                BUTTON_ACTIVE
            }
            EditorUiAction::SelectCellMaterial(material) if *material == cell_settings.material => {
                BUTTON_ACTIVE
            }
            EditorUiAction::ToggleReplace if gas_settings.replace => BUTTON_ACTIVE,
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
        let mut main_toolbar_root = ui.visibility_set.p0();
        **main_toolbar_root = if world_load_state.has_world {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut cell_type_panel_root = ui.visibility_set.p1();
        **cell_type_panel_root =
            if world_load_state.has_world && selected_tool == Some(EditorTool::BuildSolid) {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
    }

    {
        let mut debug_toolbar_root = ui.visibility_set.p2();
        **debug_toolbar_root = if world_load_state.has_world && debug_mode.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    let debug_panel_visible = world_load_state.has_world && debug_mode.active;
    ui.panel_manager.set_visible(
        DEBUG_PANEL_ID,
        debug_panel_visible,
        &mut ui.panel_open_order,
    );

    let gas_tool_panel_visible = world_load_state.has_world
        && debug_mode.active
        && selected_tool == Some(EditorTool::AddGas);
    ui.panel_manager.set_visible(
        GAS_TOOL_PANEL_ID,
        gas_tool_panel_visible,
        &mut ui.panel_open_order,
    );
    let editing_structure = structure_edit
        .selected_cell
        .and_then(|cell| structures.cell(cell.x, cell.y).map(|s| (cell, s)));
    let structure_panel_visible =
        world_load_state.has_world && debug_mode.active && editing_structure.is_some();
    ui.panel_manager.set_visible(
        STRUCTURE_TOOL_PANEL_ID,
        structure_panel_visible,
        &mut ui.panel_open_order,
    );

    {
        let mut main_menu_root = ui.visibility_set.p3();
        **main_menu_root = if main_menu.open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }

    {
        let mut replace_text = ui.text_set_primary.p0();
        replace_text.0 = if gas_settings.replace {
            "Replace: On".to_string()
        } else {
            "Replace: Off".to_string()
        };
    }

    {
        let mut buoyancy_toggle_text = ui.text_set_primary.p1();
        buoyancy_toggle_text.0 = if gas_simulation.solver_tuning.enable_buoyancy {
            "Buoyancy: On".to_string()
        } else {
            "Buoyancy: Off".to_string()
        };
    }

    {
        let mut perf_text = ui.text_set_primary.p2();
        let speed_mult = sim_control.speed.multiplier();
        perf_text.0 = format!(
            "Iterations: {} | Step ms: {:.3} | avg: {:.3} | Target Hz: {} x {} = {:.1} | Actual Hz: {:.1} | GPU compute/upload/readback/total: {:.3}/{:.3}/{:.3}/{:.3} ms",
            sim_step.0,
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
        let mut metrics_text = ui.text_set_primary.p3();
        let vectors_mode = if debug_overlay.show_momentum_vectors {
            "Impulse vectors: On"
        } else {
            "Impulse vectors: Off"
        };
        metrics_text.0 = format!(
            "{} | Anisotropy: {:.4} | Radial waves: {:.4} | Mass err H2/O2/CO2: {:.4} / {:.4} / {:.4}",
            vectors_mode,
            debug_metrics.anisotropy_score,
            debug_metrics.radial_wave_score,
            debug_metrics.mass_error_h2,
            debug_metrics.mass_error_o2,
            debug_metrics.mass_error_co2
        );
    }

    let structure_mode = match (selected_tool, editing_structure) {
        (_, Some((cell, GasStructureCell::Source { .. }))) => {
            let mut src = ui.node_set.p0();
            src.display = Display::Flex;
            let mut sink = ui.node_set.p1();
            sink.display = Display::None;
            format!("Editing Source at ({}, {})", cell.x, cell.y)
        }
        (_, Some((cell, GasStructureCell::Sink { .. }))) => {
            let mut src = ui.node_set.p0();
            src.display = Display::None;
            let mut sink = ui.node_set.p1();
            sink.display = Display::Flex;
            format!("Editing Sink at ({}, {})", cell.x, cell.y)
        }
        _ => {
            let mut src = ui.node_set.p0();
            src.display = Display::None;
            let mut sink = ui.node_set.p1();
            sink.display = Display::None;
            "No structure selected".to_string()
        }
    };
    {
        let mut mode_text = ui.text_set_primary.p4();
        mode_text.0 = structure_mode;
    }

}

#[derive(SystemParam)]
struct RefreshEditorUiSystemParams<'w, 's> {
    gas_input: Single<'w, &'static TextInputField, With<GasAmountInputField>>,
    source_input: Single<'w, &'static TextInputField, With<SourceAmountInputField>>,
    sink_input: Single<'w, &'static TextInputField, With<SinkAmountInputField>>,
    input_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static TextInputField, With<GasGammaInputField>>,
            Single<'w, &'static TextInputField, With<GasMaxColorParticlesInputField>>,
            Single<'w, &'static TextInputField, With<SimulationHzInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyStrengthInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyWindowRadiusInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyWindowSigmaInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyGainInputField>>,
            Single<'w, &'static TextInputField, With<BuoyancyAlphaInputField>>,
        ),
    >,
    buoyancy_cap_input: Single<'w, &'static TextInputField, With<BuoyancyForceCapInputField>>,
    button_query: Query<
        'w,
        's,
        (&'static EditorUiAction, &'static mut BackgroundColor),
        With<Button>,
    >,
    panel_manager: ResMut<'w, PanelManager>,
    panel_open_order: ResMut<'w, PanelOpenOrder>,
    visibility_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Visibility, With<MainToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<CellTypePanelRoot>>,
            Single<'w, &'static mut Visibility, With<DebugToolbarRoot>>,
            Single<'w, &'static mut Visibility, With<MainMenuRoot>>,
        ),
    >,
    text_set_primary: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Text, With<GasReplaceLabel>>,
            Single<'w, &'static mut Text, With<BuoyancyToggleLabel>>,
            Single<'w, &'static mut Text, With<SimulationPerfLabel>>,
            Single<'w, &'static mut Text, With<WaveMetricsLabel>>,
            Single<'w, &'static mut Text, With<StructureModeLabel>>,
        ),
    >,
    node_set: ParamSet<
        'w,
        's,
        (
            Single<'w, &'static mut Node, With<StructureSourceSection>>,
            Single<'w, &'static mut Node, With<StructureSinkSection>>,
        ),
    >,
}

fn handle_editor_mouse_input(
    input_state: (
        Res<ButtonInput<MouseButton>>,
        Single<&Window, With<PrimaryWindow>>,
        Single<(&Camera, &GlobalTransform), With<MainCamera>>,
    ),
    tool_state: (
        Res<ActiveEditorTool>,
        Res<CellToolSettings>,
        ResMut<SourceStructureToolSettings>,
        ResMut<SinkStructureToolSettings>,
        Res<MainMenuState>,
        Res<WorldLoadState>,
        Res<DebugMode>,
    ),
    ui_tool_state: (
        Res<GasToolSettings>,
        Res<GasRegistry>,
        Res<PanelManager>,
        ResMut<SelectFieldState>,
    ),
    mut field_state: ParamSet<(
        Query<&TextInputField, With<GasAmountInputField>>,
        Query<&mut TextInputField, With<SourceAmountInputField>>,
        Query<&mut TextInputField, With<SinkAmountInputField>>,
    )>,
    data_state: (
        ResMut<WorldGrid>,
        ResMut<GasStructureGrid>,
        ResMut<GasField>,
        ResMut<StructureEditState>,
        ResMut<SelectionDragState>,
        ResMut<BrushDragState>,
        EventWriter<WorldCellChanged>,
    ),
) {
    let (mouse_buttons, window, camera_query) = input_state;
    let (
        active_tool,
        cell_settings,
        mut source_settings,
        mut sink_settings,
        main_menu,
        world_load_state,
        debug_mode,
    ) = tool_state;
    let (gas_settings, gas_registry, panel_manager, mut select_fields) = ui_tool_state;
    let (
        mut world,
        mut structures,
        mut gas,
        mut structure_edit,
        mut selection_drag,
        mut brush_drag,
        mut world_changed,
    ) = data_state;

    if main_menu.open || !world_load_state.has_world {
        structure_edit.selected_cell = None;
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
                Some(&panel_manager),
            )
        })
        .unwrap_or(false);

    let hovered_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    match active_tool.selected {
        Some(EditorTool::BuildSolid) => {
            structure_edit.selected_cell = None;
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    if structures.blocks_solid_placement(cell.x, cell.y) {
                        return;
                    }
                    if world.set_solid_with_material(cell.x, cell.y, cell_settings.material) {
                        gas.clear_cell(cell.x, cell.y);
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::EraseSolid) => {
            structure_edit.selected_cell = None;
            apply_brush_tool(
                &mouse_buttons,
                blocked_by_ui,
                hovered_cell,
                &mut brush_drag,
                |cell| {
                    let _ = structures.clear(cell.x, cell.y);
                    if world.set_empty(cell.x, cell.y) {
                        world_changed.write(WorldCellChanged { cell });
                    }
                },
            );
        }
        Some(EditorTool::AddGas) | Some(EditorTool::ClearGas) => {
            structure_edit.selected_cell = None;
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
                    let amount = field_state
                        .p0()
                        .single()
                        .ok()
                        .and_then(|f| f.parsed_u32())
                        .unwrap_or(gas_settings.amount);
                    match active_tool.selected {
                        Some(EditorTool::AddGas) => {
                            if gas_registry.count() == 0 {
                                selection_drag.active = false;
                                selection_drag.start = None;
                                selection_drag.current = None;
                                return;
                            }
                            gas.apply_rect(
                                min,
                                max,
                                gas_settings.gas_index.min(gas_registry.count() - 1),
                                amount,
                                gas_settings.replace,
                                &world,
                            );
                        }
                        Some(EditorTool::ClearGas) => {
                            gas.clear_rect(min, max);
                        }
                        _ => {}
                    }
                }

                selection_drag.active = false;
                selection_drag.start = None;
                selection_drag.current = None;
            }
        }
        Some(EditorTool::CreateGasSource) => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    if gas_registry.count() > 0 {
                        let gas_index = source_settings.gas_index.min(gas_registry.count() - 1);
                        if structures.set_source(
                            cell.x,
                            cell.y,
                            gas_index,
                            source_settings.amount.max(1),
                            &world,
                        ) {
                            structure_edit.selected_cell = Some(cell);
                        }
                    }
                }
            }
        }
        Some(EditorTool::CreateGasSink) => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if mouse_buttons.just_pressed(MouseButton::Left) && !blocked_by_ui {
                if let Some(cell) = hovered_cell {
                    if structures.set_sink(cell.x, cell.y, sink_settings.amount.max(1), &world) {
                        structure_edit.selected_cell = Some(cell);
                    }
                }
            }
        }
        None => {
            clear_active_tool_state(&mut selection_drag, &mut brush_drag);
            if debug_mode.active
                && mouse_buttons.just_pressed(MouseButton::Left)
                && !blocked_by_ui
            {
                structure_edit.selected_cell =
                    hovered_cell.filter(|cell| structures.cell(cell.x, cell.y).is_some());
                if let Some(cell) = structure_edit.selected_cell {
                    match structures.cell(cell.x, cell.y) {
                        Some(GasStructureCell::Source { gas_index, amount }) => {
                            source_settings.gas_index = gas_index;
                            select_fields.set_selected(GAS_SELECT_SOURCE_ID, gas_index);
                            source_settings.amount = amount.max(1);
                            if let Ok(mut source_amount_input) = field_state.p1().single_mut() {
                                source_amount_input.text = source_settings.amount.to_string();
                                source_amount_input.cursor = source_amount_input.text.chars().count();
                                source_amount_input.value =
                                    crate::ui::input_field::ParsedInputValue::U32(
                                        source_settings.amount,
                                    );
                            }
                        }
                        Some(GasStructureCell::Sink { amount }) => {
                            sink_settings.amount = amount.max(1);
                            if let Ok(mut sink_amount_input) = field_state.p2().single_mut() {
                                sink_amount_input.text = sink_settings.amount.to_string();
                                sink_amount_input.cursor = sink_amount_input.text.chars().count();
                                sink_amount_input.value =
                                    crate::ui::input_field::ParsedInputValue::U32(sink_settings.amount);
                            }
                        }
                        None => {}
                    }
                }
            }
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
    world_load_state: Res<WorldLoadState>,
    overlay_ui_state: (Res<EditorIconSet>, Res<DebugMode>, Res<PanelManager>),
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut overlay_set: ParamSet<(
        Single<(&mut Transform, &mut Visibility, &mut Sprite), With<BlueprintGhost>>,
        Single<(&mut Transform, &mut Visibility), With<EraseCellHighlight>>,
        Single<(&mut Node, &mut Visibility), With<EraseCursorOverlay>>,
    )>,
) {
    let (icon_set, debug_mode, panel_manager) = overlay_ui_state;

    if !world_load_state.has_world {
        {
            let mut blueprint = overlay_set.p0();
            let (_, ghost_visibility, _) = &mut *blueprint;
            **ghost_visibility = Visibility::Hidden;
        }
        {
            let mut erase_highlight = overlay_set.p1();
            let (_, highlight_visibility) = &mut *erase_highlight;
            **highlight_visibility = Visibility::Hidden;
        }
        {
            let mut erase_overlay = overlay_set.p2();
            let (_, erase_visibility) = &mut *erase_overlay;
            **erase_visibility = Visibility::Hidden;
        }
        return;
    }

    let cursor_position = window.cursor_position();
    let is_on_ui = cursor_position
        .map(|cursor| {
            is_cursor_over_ui(
                cursor,
                &window,
                debug_mode.active,
                active_tool.selected,
                main_menu.open,
                Some(&panel_manager),
            )
        })
        .unwrap_or(false);

    let world_cell =
        cursor_position.and_then(|cursor| viewport_cursor_to_cell(cursor, &camera_query));

    {
        let mut blueprint = overlay_set.p0();
        let (ghost_transform, ghost_visibility, ghost_sprite) = &mut *blueprint;
        let ghost_image = match active_tool.selected {
            Some(EditorTool::BuildSolid) => Some(match cell_settings.material {
                CellMaterial::Brick => icon_set.brick_silhouette.clone(),
                CellMaterial::Metal => icon_set.metal_silhouette.clone(),
                CellMaterial::Boundary => icon_set.brick_silhouette.clone(),
            }),
            Some(EditorTool::CreateGasSource) => Some(icon_set.source_silhouette.clone()),
            Some(EditorTool::CreateGasSink) => Some(icon_set.sink_silhouette.clone()),
            _ => None,
        };
        if !is_on_ui && !main_menu.open && !mouse_buttons.pressed(MouseButton::Left) {
            if let (Some(image), Some(cell)) = (ghost_image, world_cell) {
                ghost_sprite.image = image;
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
    panel_manager: Option<&PanelManager>,
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
        || panel_manager
            .map(|panels| panels.is_cursor_over_any_panel(cursor))
            .unwrap_or(false)
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
    use crate::save::MainMenuMode;

    #[test]
    fn escape_closes_in_game_menu_when_open() {
        assert_eq!(
            escape_action(MainMenuMode::InGame, true, false, true),
            EscAction::CloseMenuKeepPaused,
            "Esc should close in-game menu first even if a tool is selected"
        );
    }

    #[test]
    fn escape_does_not_close_main_menu() {
        assert_eq!(
            escape_action(MainMenuMode::Main, false, false, false),
            EscAction::Ignore,
            "Esc must not close main menu"
        );
    }

    #[test]
    fn escape_clears_selected_tool_before_opening_menu() {
        assert_eq!(
            escape_action(MainMenuMode::Hidden, true, false, true),
            EscAction::ClearSelectedTool,
            "Esc should clear selected tool before opening menu"
        );
    }

    #[test]
    fn escape_opens_main_menu_and_pauses_when_no_tool_selected() {
        assert_eq!(
            escape_action(MainMenuMode::Hidden, false, false, true),
            EscAction::OpenMenuAndPause,
            "Esc should open in-game menu and pause when no tool is selected"
        );
    }

    #[test]
    fn escape_ignores_hidden_mode_when_world_not_loaded() {
        assert_eq!(
            escape_action(MainMenuMode::Hidden, false, false, false),
            EscAction::Ignore,
            "Esc should do nothing when no world is loaded"
        );
    }

    #[test]
    fn escape_closes_structure_editor_before_menu_actions() {
        assert_eq!(
            escape_action(MainMenuMode::Hidden, false, true, true),
            EscAction::CloseStructureEditor,
            "Esc must close structure editor first"
        );
    }
}
