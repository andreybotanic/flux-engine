use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt, fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    config::GasRegistry,
    plugins::{
        default_plugin::pipe_runtime::{
            PipeContainerKind, PipeFluxField, PipeGasField, PipeGasSnapshot,
            PipeNodeGasSnapshotEntry, PipeNodeKey,
        },
        ContentId, ContentRegistry, EnabledPluginSet, PluginId, SaveChunkStore, SubstanceId,
    },
    render::OverlayMode,
    simulation::{
        gas::{GasField, GasFieldSnapshot},
        SimulationStep,
    },
    world::{
        grid::{CellKind, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{
            PlacedStructureMap, PlacedStructureSnapshot, PlacedStructureSnapshotEntry,
            StructureParams, StructureRotation,
        },
        WorldCellChanged,
    },
};

const SCHEMA_VERSION: u32 = 6;
const WORLD_CELLS_MAGIC: &[u8; 4] = b"FXWC";
const GAS_STATE_MAGIC: &[u8; 4] = b"FXGS";
const PLACED_STRUCTURES_MAGIC: &[u8; 4] = b"FXPS";
const PIPE_GAS_MAGIC: &[u8; 4] = b"FXPG";
const WORLD_CELLS_VERSION: u16 = 2;
const GAS_STATE_VERSION: u16 = 2;
const PIPE_GAS_VERSION: u16 = 2;
const PLACED_STRUCTURES_VERSION: u16 = 2;
const CHUNK_WORLD_CELLS_ID: &str = "world_cells";
const CHUNK_GAS_STATE_ID: &str = "gas_state";
const CHUNK_PIPE_GAS_ID: &str = "pipe_gas";
const CHUNK_PLACED_STRUCTURES_ID: &str = "placed_structures";
const CHUNK_PREVIEW_PNG_ID: &str = "preview_png";
const PLUGIN_CHUNK_ID_PREFIX: &str = "plugin:";
const WORLD_CELLS_FILE: &str = "world_cells.bin";
const GAS_STATE_FILE: &str = "gas_state.bin";
const PIPE_GAS_FILE: &str = "pipe_gas.bin";
const PLACED_STRUCTURES_FILE: &str = "placed_structures.bin";
const PREVIEW_PNG_FILE: &str = "preview.png";
const META_FILE: &str = "meta.toml";

static SAVE_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Resource, Default, Clone, Debug)]
/// Stores `SaveSessionState` state.
pub struct SaveSessionState {
    pub last_persisted_step: u64,
    pub current_save_id: Option<String>,
}

impl SaveSessionState {
    /// Runs `has_unsaved_changes` logic.
    pub fn has_unsaved_changes(&self, current_step: u64) -> bool {
        current_step > self.last_persisted_step
    }

    /// Runs `mark_persisted` logic.
    pub fn mark_persisted(&mut self, step: u64, save_id: Option<String>) {
        self.last_persisted_step = step;
        self.current_save_id = save_id;
    }
}

#[derive(Resource, Clone, Copy, Debug)]
/// Stores `WorldLoadState` state.
pub struct WorldLoadState {
    pub has_world: bool,
}

impl Default for WorldLoadState {
    fn default() -> Self {
        Self { has_world: false }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MainMenuScreen {
    #[default]
    Root,
    Plugins,
    Save,
    Load,
    Confirm,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MainMenuMode {
    #[default]
    Main,
    InGame,
    Hidden,
}

#[derive(Clone, Debug)]
pub enum MainMenuDeferredAction {
    ExitToMainMenu,
    ExitApp,
}

#[derive(Clone, Debug)]
pub enum MainMenuConfirmState {
    OverwriteSave(String),
    DeleteSave(String),
    UnsavedChanges(MainMenuDeferredAction),
}

#[derive(Resource, Clone, Debug)]
/// Stores `MainMenuUiState` state.
pub struct MainMenuUiState {
    pub mode: MainMenuMode,
    pub screen: MainMenuScreen,
    pub return_screen: MainMenuScreen,
    pub confirm_state: Option<MainMenuConfirmState>,
    pub post_save_action: Option<MainMenuDeferredAction>,
    pub confirm_text: String,
    pub status_text: String,
    pub saves: Vec<SaveDescriptor>,
    pub list_item_entities: Vec<Entity>,
    pub plugin_item_entities: Vec<Entity>,
    pub needs_save_list_refresh: bool,
    pub needs_plugin_list_refresh: bool,
}

impl Default for MainMenuUiState {
    fn default() -> Self {
        Self {
            mode: MainMenuMode::Main,
            screen: MainMenuScreen::Root,
            return_screen: MainMenuScreen::Root,
            confirm_state: None,
            post_save_action: None,
            confirm_text: String::new(),
            status_text: String::new(),
            saves: Vec::new(),
            list_item_entities: Vec::new(),
            plugin_item_entities: Vec::new(),
            needs_save_list_refresh: false,
            needs_plugin_list_refresh: false,
        }
    }
}

#[derive(Clone, Debug)]
/// Stores `SaveDescriptor` state.
pub struct SaveDescriptor {
    pub id: String,
    pub display_name: String,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
    pub preview_path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
/// Describes one pending save-preview capture request.
pub struct SavePreviewRequest {
    pub descriptor: SaveDescriptor,
    pub target_path: PathBuf,
    pub post_save_action: Option<MainMenuDeferredAction>,
    pub success_status_text: String,
}

#[derive(Resource, Default, Clone, Debug)]
/// Stores queued and active save-preview capture work.
pub struct SavePreviewQueueState {
    pub pending: Option<SavePreviewRequest>,
    pub active: bool,
}

impl SavePreviewQueueState {
    /// Returns `true` when a preview capture is queued or currently running.
    pub fn is_busy(&self) -> bool {
        self.active || self.pending.is_some()
    }
}

#[derive(Event, Clone, Debug)]
/// Reports the result of one save-preview capture attempt.
pub struct SavePreviewCaptureFinished {
    pub request: SavePreviewRequest,
    pub result: Result<(), String>,
}

#[derive(Clone, Debug)]
/// Stores `RuntimeWorldState` state.
pub struct RuntimeWorldState {
    pub world_cells: Vec<CellKind>,
    pub gas_snapshot: GasFieldSnapshot,
    pub placed_structures_snapshot: PlacedStructureSnapshot,
    pub pipe_gas_snapshot: PipeGasSnapshot,
    pub plugin_save_chunks: SaveChunkStore,
    pub simulation_step: u64,
}

#[derive(Clone, Debug)]
/// Stores `LoadedSave` state.
pub struct LoadedSave {
    pub descriptor: SaveDescriptor,
    pub state: RuntimeWorldState,
}

#[derive(Debug)]
pub enum SaveError {
    Io(String),
    Parse(String),
    Validation(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SaveError::Io(msg) | SaveError::Parse(msg) | SaveError::Validation(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl std::error::Error for SaveError {}

/// Restores runtime world resources from one loaded save snapshot.
pub fn restore_runtime_world_state(
    state: RuntimeWorldState,
    world: &mut WorldGrid,
    structures: &mut PlacedStructureMap,
    gas: &mut GasField,
    pipe_gas: &mut PipeGasField,
    pipe_flux: &mut PipeFluxField,
    step: &mut SimulationStep,
) -> Result<(), String> {
    world.restore_cells(&state.world_cells)?;
    structures.restore_state(&state.placed_structures_snapshot, world)?;
    gas.restore_state(&state.gas_snapshot)?;
    pipe_gas.restore_state(&state.pipe_gas_snapshot, structures)?;
    pipe_flux.clear_all();
    step.0 = state.simulation_step;
    Ok(())
}

/// Emits `WorldCellChanged` for every cell so render state can fully resync.
pub fn emit_full_world_changed(world_changed: &mut EventWriter<WorldCellChanged>) {
    for y in 0..WORLD_HEIGHT {
        for x in 0..WORLD_WIDTH {
            world_changed.write(WorldCellChanged {
                cell: UVec2::new(x, y),
            });
        }
    }
}

/// Applies the canonical post-load presentation preset used by `New Game` and `Load`.
pub fn apply_loaded_world_preset(
    control: &mut crate::simulation::SimulationControl,
    overlay_mode: &mut OverlayMode,
    camera_transform: &mut Transform,
    camera_projection: &mut Projection,
) {
    control.paused = true;
    control.speed = crate::simulation::SimulationSpeed::X1;
    *overlay_mode = OverlayMode::Main;
    crate::input::camera::reset_camera_to_default(camera_transform, camera_projection);
}

include!("save_api_block.rs");
include!("save_meta_io_block.rs");
include!("save_content_gate_block.rs");
include!("save_gas_io_block.rs");
include!("save_pipe_gas_io_block.rs");
include!("save_tests_block.rs");
