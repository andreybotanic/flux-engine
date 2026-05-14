use bevy::{app::AppExit, ecs::system::SystemParam, prelude::*, window::PrimaryWindow};

use crate::{
    config::{AudioSettingsState, GasRegistry},
    debug::{DebugGasMetrics, DebugMode, DebugOverlaySettings},
    input::camera::MainCamera,
    plugins::{
        ContentId, InputModifiers, MouseButton as PluginMouseButton, MouseCellEvent, PluginId,
        PluginRuntimeEvent, RuntimeHostContext, SaveChunkStore,
    },
    render::{GasVisualSettings, OverlayMode},
    save::{
        apply_loaded_world_preset, create_save_with_plugin_chunks, delete_save,
        emit_full_world_changed, list_saves, load_save, new_game_snapshot,
        overwrite_save_with_plugin_chunks, restore_runtime_world_state, save_preview_target_path,
        saves_root_default, MainMenuConfirmState, MainMenuDeferredAction, MainMenuMode,
        MainMenuScreen, MainMenuSettingsTab, MainMenuUiState, SavePreviewCaptureFinished,
        SavePreviewQueueState, SavePreviewRequest, SaveSessionState, WorldLoadState,
    },
    simulation::{
        gas::GasField, GasSimulationConfig, SimulationControl, SimulationPerfStats,
        SimulationRateConfig, SimulationStep,
    },
    ui::{
        input_field::{TextInputDisplay, TextInputField, TextInputStyle},
        modal::{
            modal_panel_box_shadow, spawn_modal_backdrop_chrome, spawn_modal_panel_backdrop,
            ModalBackdropController, ModalBackdropSource, ModalBackdropSpec, ModalPanelSurface,
            ModalRoot,
        },
        palette,
        panels::{
            PanelControls, PanelCorner, PanelId, PanelManager, PanelOpenOrder, PanelScrollPolicy,
            PanelSpec, DEFAULT_PANEL_STACK_GAP,
        },
        scroll_area::{spawn_scroll_area_scrollbar, ScrollAreaViewport, UiScrollBlockState},
        select_field::{spawn_select_field, SelectFieldConfig, SelectFieldId, SelectFieldState},
        slider::{spawn_slider, SliderConfig, SliderId, SliderState, SliderValueChanged},
    },
    world::{
        grid::{cell_center, world_to_cell, CellMaterial, WorldGrid, CELL_SIZE},
        structures::{PlacedStructureMap, StructureParams, StructureRotation},
        WorldCellChanged,
    },
};

const PANEL_BG: Color = palette::PANEL_BG;
const BUTTON_IDLE: Color = palette::BUTTON_IDLE;
const BUTTON_ACTIVE: Color = palette::BUTTON_ACTIVE;
const INPUT_FOCUSED: Color = palette::INPUT_FOCUSED;
const DEBUG_PANEL_TEXT_COLOR: Color = palette::TEXT_PRIMARY;
const TOOL_BUTTON_SIZE: f32 = 40.0;
const TOOL_ICON_SIZE: f32 = 20.0;
const TOOLTIP_BG: Color = palette::TOOLTIP_BG;
const MODAL_BUTTON_BG: Color = palette::MENU_MODAL_BUTTON_BG;
const MODAL_BUTTON_HOVER: Color = palette::MENU_MODAL_BUTTON_HOVER;
const MENU_MODAL_CARD_BG: Color = palette::MENU_MODAL_CARD_BG;
const MENU_MODAL_CARD_HOVER: Color = palette::MENU_MODAL_CARD_HOVER;
const MENU_MODAL_INPUT_BG: Color = palette::MENU_MODAL_INPUT_BG;
const MENU_MODAL_INPUT_FOCUSED: Color = palette::MENU_MODAL_INPUT_FOCUSED;
const MENU_MODAL_PREVIEW_BG: Color = palette::MENU_MODAL_PREVIEW_BG;

const TOP_LEFT_SIM_PANEL_WIDTH: f32 = 320.0;
const TOP_LEFT_SIM_PANEL_HEIGHT: f32 = 112.0;

const MAIN_TOOLBAR_LEFT: f32 = 12.0;
const MAIN_TOOLBAR_BOTTOM: f32 = 12.0;
const MAIN_TOOLBAR_WIDTH: f32 = 176.0;
const MAIN_TOOLBAR_HEIGHT: f32 = 56.0;
const CELL_TYPE_PANEL_HEIGHT: f32 = 56.0;
const CELL_TYPE_PANEL_BOTTOM: f32 = MAIN_TOOLBAR_BOTTOM + MAIN_TOOLBAR_HEIGHT + 10.0;
const CELL_TYPE_PANEL_WIDTH: f32 = 176.0;

const DEBUG_TOOLBAR_LEFT: f32 = 306.0;
const DEBUG_TOOLBAR_TOP: f32 = 12.0;
const DEBUG_TOOLBAR_WIDTH: f32 = 200.0;
const DEBUG_TOOLBAR_HEIGHT: f32 = 56.0;

const DEBUG_PANEL_RIGHT: f32 = 12.0;
const DEBUG_PANEL_TOP: f32 = 12.0;
const DEBUG_PANEL_WIDTH: f32 = 286.0;

const GAS_PANEL_RIGHT: f32 = 12.0;
const GAS_PANEL_WIDTH: f32 = 286.0;
const STRUCTURE_PANEL_RIGHT: f32 = 12.0;
const STRUCTURE_PANEL_WIDTH: f32 = 286.0;
const DEBUG_PANEL_ID: PanelId = PanelId::new("debug_panel");
const GAS_TOOL_PANEL_ID: PanelId = PanelId::new("gas_tool_panel");
const STRUCTURE_TOOL_PANEL_ID: PanelId = PanelId::new("structure_tool_panel");
const GAS_SELECT_ADD_ID: SelectFieldId = SelectFieldId::new("gas_select_add");
const GAS_SELECT_SOURCE_ID: SelectFieldId = SelectFieldId::new("gas_select_source");
const SETTINGS_MUSIC_VOLUME_SLIDER_ID: SliderId =
    SliderId::new("settings_music_volume_slider");

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    BuildSolid,
    Gases,
    EraseSolid,
    Scissors,
    AddGas,
    ClearGas,
    CreateGasSource,
    CreateGasSink,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PipeToolKind {
    Pipe,
    Vent,
    Bridge,
}

#[derive(Resource, Default)]
/// Stores `ActiveEditorTool` state.
pub struct ActiveEditorTool {
    pub selected: Option<EditorTool>,
}

#[derive(Resource)]
/// Stores `CellToolSettings` state.
pub struct CellToolSettings {
    pub material: CellMaterial,
}

impl Default for CellToolSettings {
    fn default() -> Self {
        Self {
            material: crate::plugins::default_plugin::brick_cell_material(),
        }
    }
}

#[derive(Resource)]
/// Stores `PipeToolSettings` state.
pub struct PipeToolSettings {
    selected: PipeToolKind,
}

impl Default for PipeToolSettings {
    fn default() -> Self {
        Self {
            selected: PipeToolKind::Pipe,
        }
    }
}

#[derive(Resource)]
/// Stores `BridgePlacementState` state.
pub struct BridgePlacementState {
    pub rotation: StructureRotation,
}

impl Default for BridgePlacementState {
    fn default() -> Self {
        Self {
            rotation: StructureRotation::Deg0,
        }
    }
}

#[derive(Resource)]
/// Stores `MainMenuState` state.
pub struct MainMenuState {
    pub open: bool,
}

impl Default for MainMenuState {
    fn default() -> Self {
        Self { open: true }
    }
}

#[derive(Resource)]
/// Stores `GasToolSettings` state.
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
/// Stores `SourceStructureToolSettings` state.
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
/// Stores `SinkStructureToolSettings` state.
pub struct SinkStructureToolSettings {
    pub amount: u32,
}

impl Default for SinkStructureToolSettings {
    fn default() -> Self {
        Self { amount: 100 }
    }
}

#[derive(Resource, Default, Clone, Copy)]
/// Stores `StructureEditState` state.
pub struct StructureEditState {
    pub selected_cell: Option<UVec2>,
}

#[derive(Resource, Default)]
/// Stores `SelectionDragState` state.
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
    SelectPipeTool(PipeToolKind),
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
struct GasesTypePanelRoot;

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
struct SimulationIterationsLabel;

#[derive(Component)]
struct SimulationStepMsLabel;

#[derive(Component)]
struct SimulationStepAvgMsLabel;

#[derive(Component)]
struct SimulationPipeMsLabel;

#[derive(Component)]
struct SimulationPipeAvgMsLabel;

#[derive(Component)]
struct SimulationActualHzLabel;

#[derive(Component)]
struct SimulationGpuComputeMsLabel;

#[derive(Component)]
struct SimulationGpuUploadMsLabel;

#[derive(Component)]
struct SimulationGpuReadbackMsLabel;

#[derive(Component)]
struct SimulationGpuTotalMsLabel;

#[derive(Component)]
struct SimulationMassErrorLabel;

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
struct DebugGasOverlayBlockRoot;

#[derive(Component)]
struct DebugGpuTimeRow;

#[derive(Component)]
struct DebugBuoyancySwitch;

#[derive(Component)]
struct DebugImpulseSwitch;

#[derive(Component)]
struct BlueprintGhost;

#[derive(Component)]
struct EraseCursorOverlay;

#[derive(Component)]
struct EraseCursorOverlayText;

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
struct MainMenuPluginsActions;

#[derive(Component)]
struct MainMenuSettingsActions;

#[derive(Component)]
struct MainMenuSettingsFooterActions;

#[derive(Component)]
struct MainMenuSettingsGraphicsContent;

#[derive(Component)]
struct MainMenuSettingsSoundContent;

#[derive(Component)]
struct MainMenuSettingsMusicVolumeValueText;

#[derive(Component)]
struct MainMenuPluginsListRoot;

#[derive(Component)]
struct MainMenuPluginsListViewport;

#[derive(Component)]
struct MainMenuPluginsListContent;

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
struct MainMenuSaveListViewport;

#[derive(Component)]
struct MainMenuSaveListContent;

#[derive(Component)]
struct MainMenuConfirmPrimaryLabel;

#[derive(Component)]
struct MainMenuConfirmSecondaryLabel;

#[derive(Component)]
struct MainMenuConfirmCancelLabel;

#[derive(Component, Clone)]
struct MainMenuActionButton(MainMenuButtonAction);

#[derive(Component, Clone, Copy)]
struct MainMenuSettingsTabButton;

#[derive(Component, Clone, Copy)]
struct MainMenuButtonPalette {
    idle: Color,
    hover: Color,
}

#[derive(Component, Clone)]
struct MainMenuSaveCard {
    primary_action: MainMenuButtonAction,
}

#[derive(Component)]
struct MainMenuSaveCardDeleteButton;

#[derive(Component, Clone)]
struct MainMenuPluginStatusText {
    plugin_id: Option<PluginId>,
    source_name: String,
}

#[derive(Component, Clone)]
struct MainMenuPluginToggle {
    plugin_id: PluginId,
}

#[derive(Event, Clone)]
struct MainMenuActionRequest(MainMenuButtonAction);

#[derive(Clone, Debug, PartialEq, Eq)]
enum MainMenuButtonAction {
    Continue,
    NewGame,
    OpenPluginsScreen,
    OpenSettingsScreen,
    OpenSaveScreen,
    OpenLoadScreen,
    ExitToMainMenu,
    ExitApp,
    BackToRoot,
    CreateNewSave,
    SelectOverwrite(String),
    SelectLoad(String),
    SelectDelete(String),
    TogglePlugin(PluginId),
    ReloadPlugins,
    SettingsTabGraphics,
    SettingsTabSound,
    SaveSettings,
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
    gases: Handle<Image>,
    erase: Handle<Image>,
    pipe: Handle<Image>,
    vent: Handle<Image>,
    bridge: Handle<Image>,
    add_gas: Handle<Image>,
    clear_gas: Handle<Image>,
    source: Handle<Image>,
    sink: Handle<Image>,
    brick: Handle<Image>,
    metal: Handle<Image>,
    brick_silhouette: Handle<Image>,
    metal_silhouette: Handle<Image>,
    pipe_silhouette: Handle<Image>,
    vent_silhouette: Handle<Image>,
    bridge_silhouette: Handle<Image>,
    source_silhouette: Handle<Image>,
    sink_silhouette: Handle<Image>,
    select_arrow: Handle<Image>,
    main_menu_background: Handle<Image>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EscAction {
    BackToRoot,
    CloseMenuKeepPaused,
    CloseStructureEditor,
    ClearSelectedTool,
    OpenMenuAndPause,
    Ignore,
}

/// Stores `EditorPlugin` state.
pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActiveEditorTool>()
            .init_resource::<CellToolSettings>()
            .init_resource::<PipeToolSettings>()
            .init_resource::<BridgePlacementState>()
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
            .add_event::<MainMenuActionRequest>()
            .add_systems(Startup, (setup_editor_ui, setup_editor_overlays))
            .add_systems(Update, handle_escape_and_main_menu)
            .add_systems(Update, emit_plugin_keyboard_events)
            .add_systems(Update, handle_editor_ui_actions)
            .add_systems(Update, refresh_editor_ui)
            .add_systems(Update, handle_settings_slider_changes)
            .add_systems(
                Update,
                (
                    emit_main_menu_button_actions,
                    update_main_menu_save_card_interactions,
                    handle_main_menu_actions,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                sync_main_menu_plugin_rows.after(handle_main_menu_actions),
            )
            .add_systems(
                Update,
                handle_save_preview_capture_finished.after(handle_main_menu_actions),
            )
            .add_systems(
                Update,
                refresh_main_menu_ui.after(handle_save_preview_capture_finished),
            )
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

include!("ui_setup_block.rs");
include!("overlay_setup_block.rs");
include!("main_menu_block.rs");
include!("editor_ui_block.rs");
include!("input_block.rs");

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
    use super::{escape_action, open_delete_confirmation, EscAction};
    use crate::save::{MainMenuConfirmState, MainMenuMode, MainMenuScreen, MainMenuUiState};

    #[test]
    fn escape_closes_in_game_menu_when_open() {
        assert_eq!(
            escape_action(
                MainMenuMode::InGame,
                MainMenuScreen::Root,
                true,
                false,
                true
            ),
            EscAction::CloseMenuKeepPaused,
            "Esc should close in-game menu first even if a tool is selected"
        );
    }

    #[test]
    fn escape_does_not_close_main_menu() {
        assert_eq!(
            escape_action(
                MainMenuMode::Main,
                MainMenuScreen::Root,
                false,
                false,
                false
            ),
            EscAction::Ignore,
            "Esc must not close main menu"
        );
    }

    #[test]
    fn escape_clears_selected_tool_before_opening_menu() {
        assert_eq!(
            escape_action(
                MainMenuMode::Hidden,
                MainMenuScreen::Root,
                true,
                false,
                true
            ),
            EscAction::ClearSelectedTool,
            "Esc should clear selected tool before opening menu"
        );
    }

    #[test]
    fn escape_opens_main_menu_and_pauses_when_no_tool_selected() {
        assert_eq!(
            escape_action(
                MainMenuMode::Hidden,
                MainMenuScreen::Root,
                false,
                false,
                true
            ),
            EscAction::OpenMenuAndPause,
            "Esc should open in-game menu and pause when no tool is selected"
        );
    }

    #[test]
    fn escape_ignores_hidden_mode_when_world_not_loaded() {
        assert_eq!(
            escape_action(
                MainMenuMode::Hidden,
                MainMenuScreen::Root,
                false,
                false,
                false
            ),
            EscAction::Ignore,
            "Esc should do nothing when no world is loaded"
        );
    }

    #[test]
    fn escape_closes_structure_editor_before_menu_actions() {
        assert_eq!(
            escape_action(
                MainMenuMode::Hidden,
                MainMenuScreen::Root,
                false,
                true,
                true
            ),
            EscAction::CloseStructureEditor,
            "Esc must close structure editor first"
        );
    }

    #[test]
    fn escape_returns_plugins_screen_to_root() {
        assert_eq!(
            escape_action(
                MainMenuMode::Main,
                MainMenuScreen::Plugins,
                false,
                false,
                false
            ),
            EscAction::BackToRoot,
            "Esc should leave the Plugins screen open at the root menu"
        );
    }

    #[test]
    fn escape_returns_settings_screen_to_root() {
        assert_eq!(
            escape_action(
                MainMenuMode::Main,
                MainMenuScreen::Settings,
                false,
                false,
                false
            ),
            EscAction::BackToRoot,
            "Esc should leave the Settings screen open at the root menu"
        );
    }

    #[test]
    fn delete_action_opens_confirm_screen() {
        let mut ui = MainMenuUiState::default();
        ui.screen = MainMenuScreen::Load;
        open_delete_confirmation(&mut ui, "slot-1".to_string());
        assert_eq!(ui.screen, MainMenuScreen::Confirm);
        assert_eq!(ui.return_screen, MainMenuScreen::Load);
        assert!(matches!(
            ui.confirm_state,
            Some(MainMenuConfirmState::DeleteSave(ref id)) if id == "slot-1"
        ));
    }
}
