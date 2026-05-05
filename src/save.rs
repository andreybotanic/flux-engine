use std::{
    collections::{HashMap, HashSet},
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
    simulation::gas::{GasField, GasFieldSnapshot},
    world::{
        gas_structures::{GasStructureGrid, GasStructureSnapshot},
        grid::{WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
    },
};

const SCHEMA_VERSION: u32 = 2;
const WORLD_CELLS_MAGIC: &[u8; 4] = b"FXWC";
const GAS_STATE_MAGIC: &[u8; 4] = b"FXGS";
const GAS_STRUCTURES_MAGIC: &[u8; 4] = b"FXST";
const WORLD_CELLS_VERSION: u16 = 1;
const GAS_STATE_VERSION: u16 = 2;
const GAS_STRUCTURES_VERSION: u16 = 1;
const CHUNK_WORLD_CELLS_ID: &str = "world_cells";
const CHUNK_GAS_STATE_ID: &str = "gas_state";
const CHUNK_GAS_STRUCTURES_ID: &str = "gas_structures";
const WORLD_CELLS_FILE: &str = "world_cells.bin";
const GAS_STATE_FILE: &str = "gas_state.bin";
const GAS_STRUCTURES_FILE: &str = "gas_structures.bin";
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
    pub needs_save_list_refresh: bool,
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
            needs_save_list_refresh: false,
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
}

#[derive(Clone, Debug)]
/// Stores `RuntimeWorldState` state.
pub struct RuntimeWorldState {
    pub world_cell_codes: Vec<u8>,
    pub gas_snapshot: GasFieldSnapshot,
    pub gas_structures_snapshot: GasStructureSnapshot,
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

include!("save_api_block.rs");
include!("save_meta_io_block.rs");
include!("save_gas_io_block.rs");
include!("save_tests_block.rs");
