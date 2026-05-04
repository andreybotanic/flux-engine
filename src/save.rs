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
pub struct SaveSessionState {
    pub last_persisted_step: u64,
    pub current_save_id: Option<String>,
}

impl SaveSessionState {
    pub fn has_unsaved_changes(&self, current_step: u64) -> bool {
        current_step > self.last_persisted_step
    }

    pub fn mark_persisted(&mut self, step: u64, save_id: Option<String>) {
        self.last_persisted_step = step;
        self.current_save_id = save_id;
    }
}

#[derive(Resource, Clone, Copy, Debug)]
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
pub struct SaveDescriptor {
    pub id: String,
    pub display_name: String,
    pub created_at_unix_ms: i64,
    pub updated_at_unix_ms: i64,
}

#[derive(Clone, Debug)]
pub struct RuntimeWorldState {
    pub world_cell_codes: Vec<u8>,
    pub gas_snapshot: GasFieldSnapshot,
    pub gas_structures_snapshot: GasStructureSnapshot,
    pub simulation_step: u64,
}

#[derive(Clone, Debug)]
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

pub fn saves_root_default() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("saves")
}

pub fn list_saves(root: &Path) -> Result<Vec<SaveDescriptor>, SaveError> {
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut saves = Vec::new();
    let entries = fs::read_dir(root).map_err(|err| {
        SaveError::Io(format!(
            "Failed to read saves root '{}': {}",
            root.display(),
            err
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            SaveError::Io(format!(
                "Failed to read entry from saves root '{}': {}",
                root.display(),
                err
            ))
        })?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let meta_path = path.join(META_FILE);
        if !meta_path.exists() {
            continue;
        }
        let meta = read_meta(&meta_path)?;
        saves.push(SaveDescriptor {
            id: meta.save_id,
            display_name: meta.display_name,
            created_at_unix_ms: meta.created_at_unix_ms,
            updated_at_unix_ms: meta.updated_at_unix_ms,
        });
    }

    saves.sort_by(|a, b| {
        b.updated_at_unix_ms
            .cmp(&a.updated_at_unix_ms)
            .then_with(|| b.created_at_unix_ms.cmp(&a.created_at_unix_ms))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(saves)
}

pub fn create_save(
    root: &Path,
    display_name: &str,
    world: &WorldGrid,
    gas: &GasField,
    structures: &GasStructureGrid,
    gas_registry: &GasRegistry,
    simulation_step: u64,
) -> Result<SaveDescriptor, SaveError> {
    validate_display_name(display_name)?;
    fs::create_dir_all(root).map_err(|err| {
        SaveError::Io(format!(
            "Failed to create saves root '{}': {}",
            root.display(),
            err
        ))
    })?;

    let mut save_id = generate_save_id();
    while root.join(&save_id).exists() {
        save_id = generate_save_id();
    }

    let now = now_unix_ms()?;
    let descriptor = SaveDescriptor {
        id: save_id.clone(),
        display_name: display_name.to_string(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };

    write_slot(
        root,
        &descriptor,
        world,
        gas,
        structures,
        gas_registry,
        simulation_step,
        false,
    )?;
    Ok(descriptor)
}

pub fn overwrite_save(
    root: &Path,
    save_id: &str,
    world: &WorldGrid,
    gas: &GasField,
    structures: &GasStructureGrid,
    gas_registry: &GasRegistry,
    simulation_step: u64,
) -> Result<SaveDescriptor, SaveError> {
    let slot_dir = root.join(save_id);
    let meta_path = slot_dir.join(META_FILE);
    if !meta_path.exists() {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' does not exist",
            save_id
        )));
    }
    let previous = read_meta(&meta_path)?;
    let descriptor = SaveDescriptor {
        id: previous.save_id,
        display_name: previous.display_name,
        created_at_unix_ms: previous.created_at_unix_ms,
        updated_at_unix_ms: now_unix_ms()?,
    };
    write_slot(
        root,
        &descriptor,
        world,
        gas,
        structures,
        gas_registry,
        simulation_step,
        true,
    )?;
    Ok(descriptor)
}

pub fn load_save(
    root: &Path,
    save_id: &str,
    gas_registry: &GasRegistry,
) -> Result<LoadedSave, SaveError> {
    let slot_dir = root.join(save_id);
    let meta_path = slot_dir.join(META_FILE);
    let meta = read_meta(&meta_path)?;
    validate_meta_dimensions(&meta)?;

    let chunk_map = meta
        .chunks
        .iter()
        .map(|chunk| (chunk.id.clone(), chunk.file.clone()))
        .collect::<HashMap<_, _>>();

    let world_path =
        slot_dir.join(chunk_map.get(CHUNK_WORLD_CELLS_ID).ok_or_else(|| {
            SaveError::Validation("Save meta missing world_cells chunk".to_string())
        })?);
    let gas_path =
        slot_dir.join(chunk_map.get(CHUNK_GAS_STATE_ID).ok_or_else(|| {
            SaveError::Validation("Save meta missing gas_state chunk".to_string())
        })?);
    let structures_path =
        slot_dir.join(chunk_map.get(CHUNK_GAS_STRUCTURES_ID).ok_or_else(|| {
            SaveError::Validation("Save meta missing gas_structures chunk".to_string())
        })?);

    let world_codes = read_world_cells_chunk(&world_path)?;
    let gas_file = read_gas_chunk(&gas_path)?;
    let structures_snapshot = read_gas_structures_chunk(&structures_path)?;

    if gas_file.width != WORLD_WIDTH || gas_file.height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "Gas chunk dimensions mismatch: got {}x{}, expected {}x{}",
            gas_file.width, gas_file.height, WORLD_WIDTH, WORLD_HEIGHT
        )));
    }

    let mapped_snapshot = map_saved_gas_snapshot_to_registry(&gas_file, gas_registry)?;

    Ok(LoadedSave {
        descriptor: SaveDescriptor {
            id: meta.save_id,
            display_name: meta.display_name,
            created_at_unix_ms: meta.created_at_unix_ms,
            updated_at_unix_ms: meta.updated_at_unix_ms,
        },
        state: RuntimeWorldState {
            world_cell_codes: world_codes,
            gas_snapshot: mapped_snapshot,
            gas_structures_snapshot: structures_snapshot,
            simulation_step: gas_file.simulation_step,
        },
    })
}

pub fn new_game_snapshot(gas_registry: &GasRegistry) -> RuntimeWorldState {
    let world = WorldGrid::default();
    let gas = GasField::from_registry(gas_registry);
    let structures = GasStructureGrid::default();
    RuntimeWorldState {
        world_cell_codes: world.snapshot_cell_codes(),
        gas_snapshot: gas.snapshot_state(),
        gas_structures_snapshot: structures.snapshot_state(),
        simulation_step: 0,
    }
}

#[derive(Serialize, Deserialize)]
struct SaveChunkMetaToml {
    id: String,
    file: String,
    format: String,
}

#[derive(Serialize, Deserialize)]
struct SaveMetaToml {
    schema_version: u32,
    save_id: String,
    display_name: String,
    created_at_unix_ms: i64,
    updated_at_unix_ms: i64,
    world_width: u32,
    world_height: u32,
    chunks: Vec<SaveChunkMetaToml>,
}

struct SavedGasChunk {
    width: u32,
    height: u32,
    simulation_step: u64,
    gas_ids: Vec<String>,
    species: Vec<u32>,
    velocity: Vec<[f32; 2]>,
    total_density: Vec<f32>,
}

fn validate_display_name(display_name: &str) -> Result<(), SaveError> {
    let len = display_name.chars().count();
    if len == 0 {
        return Err(SaveError::Validation(
            "Save display name must not be empty".to_string(),
        ));
    }
    if len > 64 {
        return Err(SaveError::Validation(format!(
            "Save display name is too long: {} chars (max 64)",
            len
        )));
    }
    Ok(())
}

fn generate_save_id() -> String {
    let seq = SAVE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("save_{}_{}_{}", now, std::process::id(), seq)
}

fn now_unix_ms() -> Result<i64, SaveError> {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| SaveError::Validation(format!("System clock error: {}", err)))?
        .as_millis();
    i64::try_from(ms).map_err(|_| SaveError::Validation("Unix ms timestamp overflow".to_string()))
}

fn read_meta(path: &Path) -> Result<SaveMetaToml, SaveError> {
    let content = fs::read_to_string(path).map_err(|err| {
        SaveError::Io(format!(
            "Failed to read save meta '{}': {}",
            path.display(),
            err
        ))
    })?;
    toml::from_str::<SaveMetaToml>(&content).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to parse save meta '{}': {}",
            path.display(),
            err
        ))
    })
}

fn validate_meta_dimensions(meta: &SaveMetaToml) -> Result<(), SaveError> {
    if meta.schema_version != SCHEMA_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported save schema version {}",
            meta.schema_version
        )));
    }
    if meta.world_width != WORLD_WIDTH || meta.world_height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "Save world dimensions mismatch: got {}x{}, expected {}x{}",
            meta.world_width, meta.world_height, WORLD_WIDTH, WORLD_HEIGHT
        )));
    }
    Ok(())
}

fn write_slot(
    root: &Path,
    descriptor: &SaveDescriptor,
    world: &WorldGrid,
    gas: &GasField,
    structures: &GasStructureGrid,
    gas_registry: &GasRegistry,
    simulation_step: u64,
    allow_overwrite: bool,
) -> Result<(), SaveError> {
    let slot_dir = root.join(&descriptor.id);
    if slot_dir.exists() && !allow_overwrite {
        return Err(SaveError::Validation(format!(
            "Save slot '{}' already exists",
            descriptor.id
        )));
    }

    let chunk_meta = vec![
        SaveChunkMetaToml {
            id: CHUNK_WORLD_CELLS_ID.to_string(),
            file: WORLD_CELLS_FILE.to_string(),
            format: "binary_v1".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_GAS_STATE_ID.to_string(),
            file: GAS_STATE_FILE.to_string(),
            format: "binary_v2".to_string(),
        },
        SaveChunkMetaToml {
            id: CHUNK_GAS_STRUCTURES_ID.to_string(),
            file: GAS_STRUCTURES_FILE.to_string(),
            format: "binary_v1".to_string(),
        },
    ];

    let meta = SaveMetaToml {
        schema_version: SCHEMA_VERSION,
        save_id: descriptor.id.clone(),
        display_name: descriptor.display_name.clone(),
        created_at_unix_ms: descriptor.created_at_unix_ms,
        updated_at_unix_ms: descriptor.updated_at_unix_ms,
        world_width: WORLD_WIDTH,
        world_height: WORLD_HEIGHT,
        chunks: chunk_meta,
    };

    let world_codes = world.snapshot_cell_codes();
    let gas_snapshot = gas.snapshot_state();
    let structures_snapshot = structures.snapshot_state();
    let gas_ids = gas_registry
        .all()
        .iter()
        .map(|definition| definition.id.clone())
        .collect::<Vec<_>>();
    if gas_ids.len() != gas_snapshot.gas_count {
        return Err(SaveError::Validation(format!(
            "Gas registry count ({}) does not match gas snapshot count ({})",
            gas_ids.len(),
            gas_snapshot.gas_count
        )));
    }

    let tmp_dir = root.join(format!(
        ".tmp_{}_{}_{}",
        descriptor.id,
        std::process::id(),
        SAVE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if tmp_dir.exists() {
        fs::remove_dir_all(&tmp_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to remove stale temp save directory '{}': {}",
                tmp_dir.display(),
                err
            ))
        })?;
    }
    fs::create_dir_all(&tmp_dir).map_err(|err| {
        SaveError::Io(format!(
            "Failed to create temp save directory '{}': {}",
            tmp_dir.display(),
            err
        ))
    })?;

    let result = (|| -> Result<(), SaveError> {
        write_meta(&tmp_dir.join(META_FILE), &meta)?;
        write_world_cells_chunk(&tmp_dir.join(WORLD_CELLS_FILE), &world_codes)?;
        write_gas_chunk(
            &tmp_dir.join(GAS_STATE_FILE),
            simulation_step,
            &gas_ids,
            &gas_snapshot,
        )?;
        write_gas_structures_chunk(&tmp_dir.join(GAS_STRUCTURES_FILE), &structures_snapshot)?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_dir_all(&tmp_dir);
        return result;
    }

    if !slot_dir.exists() {
        fs::rename(&tmp_dir, &slot_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to commit save '{}' from '{}' to '{}': {}",
                descriptor.id,
                tmp_dir.display(),
                slot_dir.display(),
                err
            ))
        })?;
        return Ok(());
    }

    let backup_dir = root.join(format!(
        ".bak_{}_{}_{}",
        descriptor.id,
        std::process::id(),
        SAVE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    if backup_dir.exists() {
        fs::remove_dir_all(&backup_dir).map_err(|err| {
            SaveError::Io(format!(
                "Failed to remove stale backup save directory '{}': {}",
                backup_dir.display(),
                err
            ))
        })?;
    }

    fs::rename(&slot_dir, &backup_dir).map_err(|err| {
        SaveError::Io(format!(
            "Failed to move existing save '{}' to backup '{}' before overwrite: {}",
            slot_dir.display(),
            backup_dir.display(),
            err
        ))
    })?;

    match fs::rename(&tmp_dir, &slot_dir) {
        Ok(_) => {
            let _ = fs::remove_dir_all(&backup_dir);
        }
        Err(commit_err) => {
            let restore_result = fs::rename(&backup_dir, &slot_dir);
            let _ = fs::remove_dir_all(&tmp_dir);
            return match restore_result {
                Ok(_) => Err(SaveError::Io(format!(
                    "Failed to commit overwrite for save '{}' from '{}' to '{}': {}",
                    descriptor.id,
                    tmp_dir.display(),
                    slot_dir.display(),
                    commit_err
                ))),
                Err(restore_err) => Err(SaveError::Io(format!(
                    "Failed to commit overwrite for save '{}' ({}) and failed to restore backup '{}': {}",
                    descriptor.id,
                    commit_err,
                    backup_dir.display(),
                    restore_err
                ))),
            };
        }
    }

    Ok(())
}

fn write_meta(path: &Path, meta: &SaveMetaToml) -> Result<(), SaveError> {
    let text = toml::to_string_pretty(meta).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to encode save meta for '{}': {}",
            path.display(),
            err
        ))
    })?;
    fs::write(path, text).map_err(|err| {
        SaveError::Io(format!(
            "Failed to write save meta '{}': {}",
            path.display(),
            err
        ))
    })
}

fn write_world_cells_chunk(path: &Path, codes: &[u8]) -> Result<(), SaveError> {
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if codes.len() != expected {
        return Err(SaveError::Validation(format!(
            "World cell code length mismatch while writing: got {}, expected {}",
            codes.len(),
            expected
        )));
    }

    let mut bytes = Vec::with_capacity(4 + 2 + 4 + 4 + 4 + codes.len());
    bytes.extend_from_slice(WORLD_CELLS_MAGIC);
    bytes.extend_from_slice(&WORLD_CELLS_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(codes.len() as u32).to_le_bytes());
    bytes.extend_from_slice(codes);

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_world_cells_chunk(path: &Path) -> Result<Vec<u8>, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());
    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != WORLD_CELLS_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid world chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != WORLD_CELLS_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported world chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let count = read_u32(&mut cursor)? as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT {
        return Err(SaveError::Validation(format!(
            "World chunk dimensions mismatch in '{}': got {}x{}, expected {}x{}",
            path.display(),
            width,
            height,
            WORLD_WIDTH,
            WORLD_HEIGHT
        )));
    }
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if count != expected {
        return Err(SaveError::Validation(format!(
            "World chunk cell count mismatch in '{}': got {}, expected {}",
            path.display(),
            count,
            expected
        )));
    }
    let mut codes = vec![0u8; count];
    cursor.read_exact(&mut codes).map_err(|err| {
        SaveError::Parse(format!(
            "Failed to read world chunk payload '{}': {}",
            path.display(),
            err
        ))
    })?;
    Ok(codes)
}

fn write_gas_chunk(
    path: &Path,
    simulation_step: u64,
    gas_ids: &[String],
    snapshot: &GasFieldSnapshot,
) -> Result<(), SaveError> {
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.gas_count != gas_ids.len() {
        return Err(SaveError::Validation(format!(
            "Gas id count mismatch while writing gas chunk: ids={}, snapshot={}",
            gas_ids.len(),
            snapshot.gas_count
        )));
    }
    if snapshot.species.len() != cells * snapshot.gas_count {
        return Err(SaveError::Validation(format!(
            "Gas species length mismatch while writing gas chunk: got {}, expected {}",
            snapshot.species.len(),
            cells * snapshot.gas_count
        )));
    }
    if snapshot.velocity.len() != cells || snapshot.total_density.len() != cells {
        return Err(SaveError::Validation(
            "Gas snapshot buffer length mismatch while writing gas chunk".to_string(),
        ));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(GAS_STATE_MAGIC);
    bytes.extend_from_slice(&GAS_STATE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&simulation_step.to_le_bytes());
    bytes.extend_from_slice(&(gas_ids.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(cells as u32).to_le_bytes());

    for gas_id in gas_ids {
        let id_bytes = gas_id.as_bytes();
        let id_len = u16::try_from(id_bytes.len()).map_err(|_| {
            SaveError::Validation(format!(
                "Gas id '{}' is too long for gas chunk format",
                gas_id
            ))
        })?;
        bytes.extend_from_slice(&id_len.to_le_bytes());
        bytes.extend_from_slice(id_bytes);
    }

    for value in &snapshot.species {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    for [vx, vy] in &snapshot.velocity {
        bytes.extend_from_slice(&vx.to_le_bytes());
        bytes.extend_from_slice(&vy.to_le_bytes());
    }
    for value in &snapshot.total_density {
        bytes.extend_from_slice(&value.to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_gas_chunk(path: &Path) -> Result<SavedGasChunk, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());

    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != GAS_STATE_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid gas chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != GAS_STATE_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported gas chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let simulation_step = read_u64(&mut cursor)?;
    let gas_count = read_u32(&mut cursor)? as usize;
    let cells = read_u32(&mut cursor)? as usize;
    let expected_cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if cells != expected_cells {
        return Err(SaveError::Validation(format!(
            "Gas chunk cell count mismatch in '{}': got {}, expected {}",
            path.display(),
            cells,
            expected_cells
        )));
    }
    if gas_count == 0 {
        return Err(SaveError::Validation(format!(
            "Gas chunk '{}' contains zero gases",
            path.display()
        )));
    }

    let mut gas_ids = Vec::with_capacity(gas_count);
    for _ in 0..gas_count {
        let id_len = read_u16(&mut cursor)? as usize;
        let mut id_bytes = vec![0u8; id_len];
        cursor.read_exact(&mut id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Failed to read gas id from '{}' : {}",
                path.display(),
                err
            ))
        })?;
        let id = String::from_utf8(id_bytes).map_err(|err| {
            SaveError::Parse(format!(
                "Gas chunk '{}' contains invalid UTF-8 in gas id: {}",
                path.display(),
                err
            ))
        })?;
        gas_ids.push(id);
    }

    let species_len = cells * gas_count;
    let mut species = Vec::with_capacity(species_len);
    for _ in 0..species_len {
        species.push(read_u32(&mut cursor)?);
    }

    let mut velocity = Vec::with_capacity(cells);
    for _ in 0..cells {
        let vx = read_f32(&mut cursor)?;
        let vy = read_f32(&mut cursor)?;
        velocity.push([vx, vy]);
    }

    let mut total_density = Vec::with_capacity(cells);
    for _ in 0..cells {
        total_density.push(read_f32(&mut cursor)?);
    }

    Ok(SavedGasChunk {
        width,
        height,
        simulation_step,
        gas_ids,
        species,
        velocity,
        total_density,
    })
}

fn write_gas_structures_chunk(
    path: &Path,
    snapshot: &GasStructureSnapshot,
) -> Result<(), SaveError> {
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if snapshot.kinds.len() != cells
        || snapshot.gas_indices.len() != cells
        || snapshot.amounts.len() != cells
    {
        return Err(SaveError::Validation(format!(
            "Gas structures snapshot length mismatch while writing chunk: kinds={}, gas_indices={}, amounts={}, expected={}",
            snapshot.kinds.len(),
            snapshot.gas_indices.len(),
            snapshot.amounts.len(),
            cells
        )));
    }

    let mut bytes = Vec::new();
    bytes.extend_from_slice(GAS_STRUCTURES_MAGIC);
    bytes.extend_from_slice(&GAS_STRUCTURES_VERSION.to_le_bytes());
    bytes.extend_from_slice(&WORLD_WIDTH.to_le_bytes());
    bytes.extend_from_slice(&WORLD_HEIGHT.to_le_bytes());
    bytes.extend_from_slice(&(cells as u32).to_le_bytes());
    for idx in 0..cells {
        bytes.push(snapshot.kinds[idx]);
        bytes.extend_from_slice(&snapshot.gas_indices[idx].to_le_bytes());
        bytes.extend_from_slice(&snapshot.amounts[idx].to_le_bytes());
    }

    fs::write(path, bytes)
        .map_err(|err| SaveError::Io(format!("Failed to write '{}': {}", path.display(), err)))
}

fn read_gas_structures_chunk(path: &Path) -> Result<GasStructureSnapshot, SaveError> {
    let bytes = fs::read(path)
        .map_err(|err| SaveError::Io(format!("Failed to read '{}': {}", path.display(), err)))?;
    let mut cursor = Cursor::new(bytes.as_slice());

    let magic = read_exact_array::<4>(&mut cursor)?;
    if &magic != GAS_STRUCTURES_MAGIC {
        return Err(SaveError::Validation(format!(
            "Invalid gas structures chunk magic in '{}'",
            path.display()
        )));
    }
    let version = read_u16(&mut cursor)?;
    if version != GAS_STRUCTURES_VERSION {
        return Err(SaveError::Validation(format!(
            "Unsupported gas structures chunk version {} in '{}'",
            version,
            path.display()
        )));
    }
    let width = read_u32(&mut cursor)?;
    let height = read_u32(&mut cursor)?;
    let cells = read_u32(&mut cursor)? as usize;
    let expected = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    if width != WORLD_WIDTH || height != WORLD_HEIGHT || cells != expected {
        return Err(SaveError::Validation(format!(
            "Gas structures chunk dimensions mismatch in '{}': got {}x{} cells={}, expected {}x{} cells={}",
            path.display(),
            width,
            height,
            cells,
            WORLD_WIDTH,
            WORLD_HEIGHT,
            expected
        )));
    }

    let mut kinds = Vec::with_capacity(cells);
    let mut gas_indices = Vec::with_capacity(cells);
    let mut amounts = Vec::with_capacity(cells);
    for _ in 0..cells {
        let kind = read_exact_array::<1>(&mut cursor)?[0];
        let gas_index = read_u32(&mut cursor)?;
        let amount = read_u32(&mut cursor)?;
        kinds.push(kind);
        gas_indices.push(gas_index);
        amounts.push(amount);
    }

    Ok(GasStructureSnapshot {
        kinds,
        gas_indices,
        amounts,
    })
}

fn map_saved_gas_snapshot_to_registry(
    saved: &SavedGasChunk,
    gas_registry: &GasRegistry,
) -> Result<GasFieldSnapshot, SaveError> {
    let mut seen = HashSet::new();
    for gas_id in &saved.gas_ids {
        if !seen.insert(gas_id.clone()) {
            return Err(SaveError::Validation(format!(
                "Gas chunk contains duplicate gas id '{}'",
                gas_id
            )));
        }
        if gas_registry.index_of(gas_id).is_none() {
            return Err(SaveError::Validation(format!(
                "Gas chunk references unknown gas id '{}'",
                gas_id
            )));
        }
    }

    let current_gas_count = gas_registry.count();
    let cells = (WORLD_WIDTH * WORLD_HEIGHT) as usize;
    let mut mapped_species = vec![0u32; cells * current_gas_count];
    for (saved_idx, gas_id) in saved.gas_ids.iter().enumerate() {
        let current_idx = gas_registry.index_of(gas_id).ok_or_else(|| {
            SaveError::Validation(format!("Gas chunk references unknown gas id '{}'", gas_id))
        })?;
        for cell in 0..cells {
            let saved_value = saved.species[cell * saved.gas_ids.len() + saved_idx];
            let mapped_index = cell * current_gas_count + current_idx;
            mapped_species[mapped_index] = saved_value;
        }
    }

    Ok(GasFieldSnapshot {
        gas_count: current_gas_count,
        species: mapped_species,
        total_density: saved
            .total_density
            .iter()
            .copied()
            .map(|value| value.max(0.0))
            .collect(),
        velocity: saved.velocity.clone(),
    })
}

fn read_exact_array<const N: usize>(cursor: &mut Cursor<&[u8]>) -> Result<[u8; N], SaveError> {
    let mut buf = [0u8; N];
    cursor
        .read_exact(&mut buf)
        .map_err(|err| SaveError::Parse(format!("Failed to read binary payload: {}", err)))?;
    Ok(buf)
}

fn read_u16(cursor: &mut Cursor<&[u8]>) -> Result<u16, SaveError> {
    Ok(u16::from_le_bytes(read_exact_array::<2>(cursor)?))
}

fn read_u32(cursor: &mut Cursor<&[u8]>) -> Result<u32, SaveError> {
    Ok(u32::from_le_bytes(read_exact_array::<4>(cursor)?))
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, SaveError> {
    Ok(u64::from_le_bytes(read_exact_array::<8>(cursor)?))
}

fn read_f32(cursor: &mut Cursor<&[u8]>) -> Result<f32, SaveError> {
    Ok(f32::from_le_bytes(read_exact_array::<4>(cursor)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::GasDefinition,
        world::{gas_structures::GasStructureGrid, grid::{CellMaterial, WorldGrid}},
    };

    fn test_registry() -> GasRegistry {
        GasRegistry::new(vec![
            GasDefinition {
                id: "h2".to_string(),
                label: "Hydrogen".to_string(),
                molecular_mass: 2.016,
                color: [0.8, 0.2, 0.9],
            },
            GasDefinition {
                id: "o2".to_string(),
                label: "Oxygen".to_string(),
                molecular_mass: 31.998,
                color: [0.0, 0.85, 0.85],
            },
            GasDefinition {
                id: "co2".to_string(),
                label: "Carbon dioxide".to_string(),
                molecular_mass: 44.009,
                color: [0.5, 0.5, 0.5],
            },
        ])
        .expect("valid registry")
    }

    fn temp_saves_root(prefix: &str) -> PathBuf {
        let name = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        );
        let root = std::env::temp_dir().join(name);
        fs::create_dir_all(&root).expect("create temp root");
        root
    }

    #[test]
    fn save_roundtrip_preserves_world_and_gas_state() {
        let root = temp_saves_root("flux_save_roundtrip");
        let registry = test_registry();
        let mut world = WorldGrid::default();
        assert!(world.set_solid_with_material(10, 10, CellMaterial::Brick));
        assert!(world.set_solid_with_material(11, 10, CellMaterial::Metal));

        let mut gas = GasField::from_registry(&registry);
        let mut structures = GasStructureGrid::default();
        let _ = gas.apply_species_delta_with_lbm(50, 50, 0, 7_000.0);
        let _ = gas.apply_species_delta_with_lbm(51, 50, 1, 4_000.0);
        let _ = gas.apply_species_delta_with_lbm(52, 50, 2, 2_000.0);
        assert!(structures.set_source(14, 14, 0, 10, &world));
        assert!(structures.set_sink(15, 14, 5, &world));

        let descriptor = create_save(&root, "Test Save", &world, &gas, &structures, &registry, 123)
            .expect("create save");
        let loaded = load_save(&root, &descriptor.id, &registry).expect("load save");
        assert_eq!(loaded.state.simulation_step, 123);
        assert_eq!(
            loaded.state.world_cell_codes.len(),
            (WORLD_WIDTH * WORLD_HEIGHT) as usize
        );

        let mut restored_world = WorldGrid::default();
        restored_world
            .restore_from_cell_codes(&loaded.state.world_cell_codes)
            .expect("restore world");
        assert_eq!(restored_world.cell(10, 10), world.cell(10, 10));
        assert_eq!(restored_world.cell(11, 10), world.cell(11, 10));

        let mut restored_gas = GasField::from_registry(&registry);
        restored_gas
            .restore_state(&loaded.state.gas_snapshot)
            .expect("restore gas");
        let mut restored_structures = GasStructureGrid::default();
        restored_structures
            .restore_state(&loaded.state.gas_structures_snapshot, &restored_world)
            .expect("restore structures");
        let original = gas.snapshot_state();
        let restored = restored_gas.snapshot_state();
        assert_eq!(original.gas_count, restored.gas_count);
        assert_eq!(original.species.len(), restored.species.len());
        for i in 0..original.species.len() {
            assert_eq!(original.species[i], restored.species[i]);
        }
        assert_eq!(restored_structures.cell(14, 14).is_some(), true);
        assert_eq!(restored_structures.cell(15, 14).is_some(), true);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn list_saves_sorted_from_new_to_old() {
        let root = temp_saves_root("flux_save_sort");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = GasStructureGrid::default();

        let first =
            create_save(&root, "first", &world, &gas, &structures, &registry, 1).expect("first save");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = create_save(&root, "second", &world, &gas, &structures, &registry, 2)
            .expect("second save");

        let saves = list_saves(&root).expect("list saves");
        assert_eq!(saves.len(), 2);
        assert_eq!(saves[0].id, second.id);
        assert_eq!(saves[1].id, first.id);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_rejects_unknown_saved_gas_id() {
        let root = temp_saves_root("flux_save_unknown_gas");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = GasStructureGrid::default();

        let descriptor = create_save(
            &root,
            "unknown-gas",
            &world,
            &gas,
            &structures,
            &registry,
            5,
        )
        .expect("save");
        let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
        let mut bytes = fs::read(&gas_path).expect("read gas chunk");
        // Header: magic(4)+version(2)+width(4)+height(4)+step(8)+gas_count(4)+cells(4) = 30
        // First gas id len u16 starts at 30.
        let id_len = u16::from_le_bytes([bytes[30], bytes[31]]) as usize;
        assert!(id_len >= 2);
        bytes[32] = b'z';
        bytes[33] = b'z';
        fs::write(&gas_path, bytes).expect("write patched gas chunk");

        let err =
            load_save(&root, &descriptor.id, &registry).expect_err("must fail on unknown gas");
        assert!(err.to_string().contains("unknown gas id"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_rejects_invalid_world_chunk_magic() {
        let root = temp_saves_root("flux_save_bad_world_magic");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = GasStructureGrid::default();
        let descriptor = create_save(
            &root,
            "bad-world-magic",
            &world,
            &gas,
            &structures,
            &registry,
            7,
        )
        .expect("save");

        let world_path = root.join(&descriptor.id).join(WORLD_CELLS_FILE);
        let mut bytes = fs::read(&world_path).expect("read world chunk");
        bytes[0] = b'B';
        fs::write(&world_path, bytes).expect("write patched world chunk");

        let err = load_save(&root, &descriptor.id, &registry).expect_err("must fail on bad magic");
        assert!(err.to_string().contains("Invalid world chunk magic"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_rejects_invalid_gas_chunk_version() {
        let root = temp_saves_root("flux_save_bad_gas_version");
        let registry = test_registry();
        let world = WorldGrid::default();
        let gas = GasField::from_registry(&registry);
        let structures = GasStructureGrid::default();
        let descriptor = create_save(
            &root,
            "bad-gas-version",
            &world,
            &gas,
            &structures,
            &registry,
            11,
        )
        .expect("save");

        let gas_path = root.join(&descriptor.id).join(GAS_STATE_FILE);
        let mut bytes = fs::read(&gas_path).expect("read gas chunk");
        bytes[4] = 0xFF;
        bytes[5] = 0x7F;
        fs::write(&gas_path, bytes).expect("write patched gas chunk");

        let err =
            load_save(&root, &descriptor.id, &registry).expect_err("must fail on bad gas version");
        assert!(err.to_string().contains("Unsupported gas chunk version"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn save_session_state_uses_step_only_for_dirty_check() {
        let mut session = SaveSessionState::default();
        session.mark_persisted(100, Some("slot".to_string()));
        assert!(!session.has_unsaved_changes(100));
        assert!(session.has_unsaved_changes(101));
    }

    #[test]
    fn world_load_state_starts_unloaded() {
        assert!(!WorldLoadState::default().has_world);
    }
}
