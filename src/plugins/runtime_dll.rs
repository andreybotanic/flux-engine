use std::{ffi::c_void, path::PathBuf, ptr};

use bevy::prelude::*;
use libloading::Library;

#[path = "runtime_dll_events.rs"]
mod event_dispatch;

use self::event_dispatch::dispatch_to_plugin;

use crate::{
    config::GasRegistry,
    editor::{ActiveEditorTool, EditorTool},
    plugins::{
        abi::{FluxPluginDestroyFn, FluxPluginDispatchFn, FluxPluginHandle, FluxStatus, FluxUtf8Slice},
        api::{
            events::PluginEvent, render_api::OverlayRenderPolicy, runtime::PluginRuntimeRegistry,
            save_api::SaveChunkStore, ui_api::HudBlock,
        },
        default_plugin,
        ContentId, ContentRegistry, LoadedPluginMetadata, PluginId, PluginRuntimeEvent,
    },
    render::{
        world_view::{PluginOverlayImage, PluginOverlaySprite},
        OverlayMode,
    },
    save::WorldLoadState,
    simulation::{gas::GasField, GpuRuntimeState, SimulationControl, SimulationSet, SimulationSpeed, SimulationStep},
    world::{
        grid::{is_editable_cell, CellKind, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
        structures::{PlacedStructureId, PlacedStructureMap, StructureRotation},
        WorldCellChanged,
    },
};

/// One complete overlay frame submitted by a runtime plugin.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PluginOverlayFrameStore {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

impl PluginOverlayFrameStore {
    /// Removes the submitted overlay image from the current plugin overlay frame.
    pub fn clear(&mut self) {
        self.width = 0;
        self.height = 0;
        self.rgba8.clear();
    }

    /// Returns true when a plugin submitted one complete overlay frame.
    pub fn has_frame(&self) -> bool {
        self.width > 0 && self.height > 0 && !self.rgba8.is_empty()
    }
}

/// HUD blocks submitted by runtime plugins for the current hovered cell.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct PluginHudBlockStore {
    pub blocks: Vec<HudBlock>,
}

impl PluginHudBlockStore {
    /// Removes all plugin HUD blocks from the current frame.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }
}

/// Live runtime DLL plugins that can receive subscribed events.
#[derive(Resource, Default)]
pub struct RuntimeDllPluginRegistry {
    plugins: Vec<RuntimeDllPlugin>,
    errors: Vec<String>,
}

/// Bevy plugin that dispatches subscribed runtime events into live DLL plugins.
pub struct RuntimeDllHostPlugin;

impl Plugin for RuntimeDllHostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, dispatch_queued_runtime_plugin_events)
            .add_systems(Update, dispatch_plugin_overlay_render)
            .add_systems(
                FixedUpdate,
                dispatch_runtime_plugin_pre_gas_step.in_set(SimulationSet::PluginPreStep),
            )
            .add_systems(
                FixedUpdate,
                dispatch_runtime_plugin_post_gas_step.after(SimulationSet::CellGasStep),
            );
    }
}

impl RuntimeDllPluginRegistry {
    /// Loads live DLL instances for every enabled non-builtin plugin that has a manifest.
    pub fn from_loaded_plugins(loaded_plugins: &[LoadedPluginMetadata]) -> Self {
        let mut registry = Self::default();
        for plugin in loaded_plugins {
            if plugin.manifest.is_none() {
                continue;
            }
            match crate::plugins::loader::instantiate_runtime_plugin(plugin) {
                Ok(runtime_plugin) => registry.plugins.push(runtime_plugin),
                Err(error) => registry
                    .errors
                    .push(format!("{}: {}", plugin.plugin_id, error)),
            }
        }
        registry
    }

    /// Returns live plugin loading errors collected during the last rebuild.
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Dispatches one plugin event to every live DLL subscribed to that event kind.
    pub(crate) fn dispatch_event(
        &mut self,
        subscriptions: &PluginRuntimeRegistry,
        event: &PluginRuntimeEvent,
        context: &mut RuntimeHostContext,
    ) {
        let event_kind = event.kind();
        let subscribers = subscriptions.subscribers(event_kind);
        if subscribers.is_empty() {
            return;
        }

        for plugin in &mut self.plugins {
            if !subscribers
                .iter()
                .any(|plugin_id| plugin_id == &plugin.plugin_id)
            {
                continue;
            }
            context.plugin_id = plugin.plugin_id.clone();
            dispatch_to_plugin(plugin.handle, plugin.dispatch_fn, event, context);
        }
    }
}

/// One live DLL plugin instance and its loaded library.
pub struct RuntimeDllPlugin {
    pub plugin_id: PluginId,
    pub handle: *mut FluxPluginHandle,
    pub dispatch_fn: FluxPluginDispatchFn,
    pub destroy_fn: FluxPluginDestroyFn,
    pub cache_root: PathBuf,
    pub library: Library,
}

unsafe impl Send for RuntimeDllPlugin {}
unsafe impl Sync for RuntimeDllPlugin {}

impl Drop for RuntimeDllPlugin {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                (self.destroy_fn)(self.handle);
            }
            self.handle = ptr::null_mut();
        }
        let _ = std::fs::remove_dir_all(&self.cache_root);
    }
}

/// Mutable host context exposed to plugin ABI callbacks during one dispatch.
pub struct RuntimeHostContext<'a> {
    pub plugin_id: PluginId,
    pub content_registry: &'a ContentRegistry,
    pub gas_registry: &'a GasRegistry,
    pub world: Option<&'a mut WorldGrid>,
    pub structures: Option<&'a mut PlacedStructureMap>,
    pub gas: Option<&'a mut GasField>,
    pub gpu_state: Option<&'a mut GpuRuntimeState>,
    pub overlay_frame: Option<&'a mut PluginOverlayFrameStore>,
    pub hud_blocks: Option<&'a mut PluginHudBlockStore>,
    pub save_chunks: Option<&'a mut SaveChunkStore>,
    pub simulation_control: Option<&'a mut SimulationControl>,
    pub simulation_step: u64,
    pub active_tool: Option<&'a mut ActiveEditorTool>,
    pub changed_cells: Vec<UVec2>,
}

impl<'a> RuntimeHostContext<'a> {
    /// Creates a runtime host context with no mutable world resources attached.
    pub fn new(content_registry: &'a ContentRegistry, gas_registry: &'a GasRegistry) -> Self {
        Self {
            plugin_id: PluginId::default_plugin(),
            content_registry,
            gas_registry,
            world: None,
            structures: None,
            gas: None,
            gpu_state: None,
            overlay_frame: None,
            hud_blocks: None,
            save_chunks: None,
            simulation_control: None,
            simulation_step: 0,
            active_tool: None,
            changed_cells: Vec::new(),
        }
    }
}

/// ABI callback that places one entity by stable kind id.
pub unsafe extern "C" fn entity_place_callback(
    context: *mut c_void,
    kind_id: FluxUtf8Slice,
    origin_x: u32,
    origin_y: u32,
    rotation: u32,
    out_entity_id: *mut u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if origin_x >= WORLD_WIDTH || origin_y >= WORLD_HEIGHT || out_entity_id.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(kind_id) = read_abi_utf8(kind_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let rotation = decode_rotation(rotation);

    if let Some(material) = cell_material_for_entity_kind(&kind_id) {
        let Some(world) = context.world.as_deref_mut() else {
            return FluxStatus::FAILED;
        };
        if !is_editable_cell(origin_x, origin_y) {
            return FluxStatus::INVALID_ARGUMENT;
        }
        if world.set_cell_kind(origin_x, origin_y, CellKind::Solid(material)) {
            if let Some(gas) = context.gas.as_deref_mut() {
                gas.clear_cell(origin_x, origin_y);
            }
            if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
                gpu_state.mark_needs_full_upload();
            }
            context.changed_cells.push(UVec2::new(origin_x, origin_y));
        }
        *out_entity_id = encode_cell_entity_id(origin_x, origin_y);
        return FluxStatus::OK;
    }

    let (Some(structures), Some(world)) = (context.structures.as_deref_mut(), context.world.as_deref()) else {
        return FluxStatus::FAILED;
    };
    let origin = UVec2::new(origin_x, origin_y);
    let placed = match kind_id.as_str() {
        default_plugin::ENTITY_PIPE_ID => {
            structures.place_pipe(origin_x, origin_y, world).then_some(encode_structure_entity_id(
                structures
                    .structure_ids_at(origin_x, origin_y)
                    .last()
                    .copied()
                    .unwrap_or(PlacedStructureId(0)),
            ))
        }
        default_plugin::ENTITY_VENT_ID => {
            structures.place_vent(origin_x, origin_y, world).then_some(encode_structure_entity_id(
                structures
                    .structure_ids_at(origin_x, origin_y)
                    .last()
                    .copied()
                    .unwrap_or(PlacedStructureId(0)),
            ))
        }
        default_plugin::ENTITY_GAS_SOURCE_ID => structures
            .place_gas_source(origin_x, origin_y, 0, 100, world)
            .map(encode_structure_entity_id),
        default_plugin::ENTITY_GAS_SINK_ID => structures
            .place_gas_sink(origin_x, origin_y, 100, world)
            .map(encode_structure_entity_id),
        default_plugin::ENTITY_GAS_PIPE_BRIDGE_ID => {
            structures.place_bridge(origin, rotation, world).map(encode_structure_entity_id)
        }
        _ => None,
    };
    let Some(entity_id) = placed else {
        return FluxStatus::FAILED;
    };
    *out_entity_id = entity_id;
    context.changed_cells.push(origin);
    FluxStatus::OK
}

/// ABI callback that removes one entity by runtime id.
pub unsafe extern "C" fn entity_remove_callback(
    context: *mut c_void,
    entity_id: u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if is_cell_entity_id(entity_id) {
        let Some((x, y)) = decode_cell_entity_id(entity_id) else {
            return FluxStatus::INVALID_ARGUMENT;
        };
        let Some(world) = context.world.as_deref_mut() else {
            return FluxStatus::FAILED;
        };
        if world.set_empty(x, y) {
            context.changed_cells.push(UVec2::new(x, y));
        }
        return FluxStatus::OK;
    }
    let Some(structures) = context.structures.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let Some(structure_id) = decode_structure_entity_id(entity_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let changed_cells = structures
        .structure(structure_id)
        .map(|structure| structure.occupied_cells())
        .unwrap_or_default();
    if !structures.remove_structure(structure_id) {
        return FluxStatus::FAILED;
    }
    context.changed_cells.extend(changed_cells);
    FluxStatus::OK
}

/// ABI callback that removes an entity from one cell and layer.
pub unsafe extern "C" fn entity_remove_at_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    _layer_id: FluxUtf8Slice,
    out_entity_id: *mut u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_entity_id.is_null() || x >= WORLD_WIDTH || y >= WORLD_HEIGHT {
        return FluxStatus::INVALID_ARGUMENT;
    }
    if let Some(world) = context.world.as_deref_mut() {
        if world.is_solid(x, y) && is_editable_cell(x, y) {
            world.set_empty(x, y);
            context.changed_cells.push(UVec2::new(x, y));
            *out_entity_id = encode_cell_entity_id(x, y);
            return FluxStatus::OK;
        }
    }
    let Some(structures) = context.structures.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let removed = structures.clear_cell(x, y);
    if let Some(id) = removed.first().copied() {
        *out_entity_id = encode_structure_entity_id(id);
        context.changed_cells.push(UVec2::new(x, y));
        FluxStatus::OK
    } else {
        *out_entity_id = 0;
        FluxStatus::FAILED
    }
}

/// ABI callback that rotates one structure entity.
pub unsafe extern "C" fn entity_set_rotation_callback(
    _context: *mut c_void,
    _entity_id: u32,
    _rotation: u32,
) -> FluxStatus {
    FluxStatus::UNSUPPORTED
}

/// ABI callback that enables or disables one entity.
pub unsafe extern "C" fn entity_set_enabled_callback(
    _context: *mut c_void,
    _entity_id: u32,
    _enabled: u8,
) -> FluxStatus {
    FluxStatus::UNSUPPORTED
}

/// ABI callback that sets one entity label.
pub unsafe extern "C" fn entity_set_label_callback(
    _context: *mut c_void,
    _entity_id: u32,
    _label: FluxUtf8Slice,
) -> FluxStatus {
    FluxStatus::UNSUPPORTED
}

/// ABI callback that adds free gas with a requested cell velocity.
pub unsafe extern "C" fn add_gas_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    amount: u32,
    velocity_x: f32,
    velocity_y: f32,
    out_added: *mut u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if x >= WORLD_WIDTH || y >= WORLD_HEIGHT {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(substance) = read_abi_utf8(substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(gas_index) = context.gas_registry.index_of(&substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let (Some(gas), Some(world)) = (context.gas.as_deref_mut(), context.world.as_deref()) else {
        return FluxStatus::FAILED;
    };
    let added = gas.add_particles_with_velocity(
        x,
        y,
        gas_index,
        amount,
        Vec2::new(velocity_x, velocity_y),
        world,
    );
    if !out_added.is_null() {
        *out_added = added;
    }
    if added > 0 {
        if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
            gpu_state.mark_needs_full_upload();
        }
    }
    FluxStatus::OK
}

/// ABI callback that removes free gas from one cell.
pub unsafe extern "C" fn remove_gas_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    amount: u32,
    out_removed: *mut u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if x >= WORLD_WIDTH || y >= WORLD_HEIGHT {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(substance) = read_abi_utf8(substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(gas_index) = context.gas_registry.index_of(&substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let (Some(gas), Some(world)) = (context.gas.as_deref_mut(), context.world.as_deref()) else {
        return FluxStatus::FAILED;
    };
    let available = gas.amount_rounded(x, y, gas_index);
    let removed = gas.remove_particles_proportional(x, y, amount.min(available), world);
    if !out_removed.is_null() {
        *out_removed = removed;
    }
    if removed > 0 {
        if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
            gpu_state.mark_needs_full_upload();
        }
    }
    FluxStatus::OK
}

/// ABI callback that clears all free gas from one cell.
pub unsafe extern "C" fn clear_gas_cell_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(gas) = context.gas.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    gas.clear_cell(x, y);
    if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
        gpu_state.mark_needs_full_upload();
    }
    FluxStatus::OK
}

/// ABI callback that returns free-gas pressure in one cell.
pub unsafe extern "C" fn gas_pressure_at_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    out_pressure: *mut f32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_pressure.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(gas) = context.gas.as_deref() else {
        return FluxStatus::FAILED;
    };
    *out_pressure = gas.total_amount(x, y);
    FluxStatus::OK
}

/// ABI callback that returns one substance amount in a cell.
pub unsafe extern "C" fn gas_amount_at_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    out_amount: *mut u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_amount.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(substance) = read_abi_utf8(substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(gas_index) = context.gas_registry.index_of(&substance) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(gas) = context.gas.as_deref() else {
        return FluxStatus::FAILED;
    };
    *out_amount = gas.amount_rounded(x, y, gas_index);
    FluxStatus::OK
}

/// ABI callback that submits one complete RGBA8 image for the active plugin overlay.
pub unsafe extern "C" fn submit_overlay_frame_callback(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if width != WORLD_WIDTH || height != WORLD_HEIGHT {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(expected_len) = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
    else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if len != expected_len || rgba8.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(frame) = context.overlay_frame.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    frame.width = width;
    frame.height = height;
    frame.rgba8.clear();
    frame
        .rgba8
        .extend_from_slice(std::slice::from_raw_parts(rgba8, len));
    FluxStatus::OK
}

/// ABI callback that appends a HUD block for the current hovered cell.
pub unsafe extern "C" fn submit_hud_block_callback(
    context: *mut c_void,
    block_id: FluxUtf8Slice,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
    sort_order: i32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let (Ok(raw_block_id), Ok(title), Ok(line)) =
        (read_abi_utf8(block_id), read_abi_utf8(title), read_abi_utf8(line))
    else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(store) = context.hud_blocks.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let id = match ContentId::parse(&raw_block_id) {
        Ok(id) => id,
        Err(_) => return FluxStatus::FAILED,
    };
    store.blocks.push(HudBlock {
        id,
        title,
        lines: vec![line],
        sort_order,
    });
    FluxStatus::OK
}

/// ABI callback that writes one plugin-owned save chunk into the host chunk store.
pub unsafe extern "C" fn write_save_chunk_callback(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Ok(raw_chunk_id) = read_abi_utf8(chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if len > 0 && bytes.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(chunk_id) = ContentId::parse(&raw_chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let payload = if len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(bytes, len).to_vec()
    };
    let Some(store) = context.save_chunks.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    store.write_plugin_chunk(context.plugin_id.clone(), chunk_id, version, payload);
    FluxStatus::OK
}

/// ABI callback that deletes one plugin-owned save chunk.
pub unsafe extern "C" fn delete_save_chunk_callback(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Ok(raw_chunk_id) = read_abi_utf8(chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Ok(chunk_id) = ContentId::parse(&raw_chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(store) = context.save_chunks.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    store.delete_plugin_chunk(&context.plugin_id, &chunk_id);
    FluxStatus::OK
}

/// ABI callback that reads one plugin-owned save chunk from the host chunk store.
pub unsafe extern "C" fn read_save_chunk_callback(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Ok(raw_chunk_id) = read_abi_utf8(chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_version.is_null() || out_len.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    if len > 0 && bytes.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(chunk_id) = ContentId::parse(&raw_chunk_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(store) = context.save_chunks.as_deref() else {
        return FluxStatus::FAILED;
    };
    let Some(chunk) = store.read_plugin_chunk(&context.plugin_id, &chunk_id) else {
        *out_version = 0;
        *out_len = 0;
        return FluxStatus::FAILED;
    };
    *out_version = chunk.version;
    *out_len = chunk.bytes.len();
    if len < chunk.bytes.len() {
        return FluxStatus::FAILED;
    }
    if !chunk.bytes.is_empty() {
        std::ptr::copy_nonoverlapping(chunk.bytes.as_ptr(), bytes, chunk.bytes.len());
    }
    FluxStatus::OK
}

/// ABI callback that returns the current simulation time snapshot.
pub unsafe extern "C" fn get_time_snapshot_callback(
    context: *mut c_void,
    out_snapshot: *mut crate::plugins::abi::FluxTimeSnapshot,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_snapshot.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(control) = context.simulation_control.as_deref() else {
        return FluxStatus::FAILED;
    };
    *out_snapshot = crate::plugins::abi::FluxTimeSnapshot {
        tick: context.simulation_step,
        paused: control.paused as u8,
        speed: encode_speed(control.speed),
        delta_seconds: 0.0,
    };
    FluxStatus::OK
}

/// ABI callback that sets the pause flag.
pub unsafe extern "C" fn set_paused_callback(
    context: *mut c_void,
    paused: u8,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(control) = context.simulation_control.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    control.paused = paused != 0;
    FluxStatus::OK
}

/// ABI callback that toggles pause.
pub unsafe extern "C" fn toggle_pause_callback(
    context: *mut c_void,
    out_paused: *mut u8,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if out_paused.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(control) = context.simulation_control.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    control.paused = !control.paused;
    *out_paused = control.paused as u8;
    FluxStatus::OK
}

/// ABI callback that sets the simulation speed.
pub unsafe extern "C" fn set_speed_callback(
    context: *mut c_void,
    speed: u32,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(control) = context.simulation_control.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    control.speed = match speed {
        2 => SimulationSpeed::X2,
        5 => SimulationSpeed::X5,
        _ => SimulationSpeed::X1,
    };
    FluxStatus::OK
}

/// ABI callback that selects the active tool by stable id.
pub unsafe extern "C" fn set_active_tool_callback(
    context: *mut c_void,
    tool_id: FluxUtf8Slice,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(active_tool) = context.active_tool.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let Ok(tool_id) = read_abi_utf8(tool_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    active_tool.selected = editor_tool_from_id(&tool_id);
    FluxStatus::OK
}

/// ABI callback that forwards plugin runtime log messages into the engine stderr log.
pub unsafe extern "C" fn write_runtime_log_callback(
    context: *mut c_void,
    level: u32,
    message: FluxUtf8Slice,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Ok(message) = read_abi_utf8(message) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if message.trim().is_empty() {
        return FluxStatus::OK;
    }

    eprintln!(
        "Runtime plugin [{}] {}: {}",
        context.plugin_id,
        runtime_log_level_label(level),
        message
    );
    FluxStatus::OK
}

unsafe fn read_abi_utf8(slice: FluxUtf8Slice) -> Result<String, ()> {
    if slice.len == 0 {
        return Ok(String::new());
    }
    if slice.ptr.is_null() {
        return Err(());
    }
    let bytes = std::slice::from_raw_parts(slice.ptr, slice.len);
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .map_err(|_| ())
}

fn cell_material_for_entity_kind(kind_id: &str) -> Option<CellMaterial> {
    match kind_id {
        default_plugin::CELL_BRICK_ID => Some(default_plugin::brick_cell_material()),
        default_plugin::CELL_METAL_ID => Some(default_plugin::metal_cell_material()),
        default_plugin::CELL_BOUNDARY_ID => Some(default_plugin::boundary_cell_material()),
        _ => None,
    }
}

fn decode_rotation(raw: u32) -> StructureRotation {
    match raw {
        1 => StructureRotation::Deg90,
        2 => StructureRotation::Deg180,
        3 => StructureRotation::Deg270,
        _ => StructureRotation::Deg0,
    }
}

fn encode_speed(speed: SimulationSpeed) -> u32 {
    match speed {
        SimulationSpeed::X1 => 1,
        SimulationSpeed::X2 => 2,
        SimulationSpeed::X5 => 5,
    }
}

fn editor_tool_from_id(tool_id: &str) -> Option<EditorTool> {
    Some(match tool_id {
        "" => return None,
        "flux.core.tool.build_solid" => EditorTool::BuildSolid,
        "flux.default.tool.gases" => EditorTool::Gases,
        "flux.core.tool.erase_solid" => EditorTool::EraseSolid,
        "flux.default.tool.scissors" => EditorTool::Scissors,
        "flux.core.tool.add_gas" => EditorTool::AddGas,
        "flux.core.tool.clear_gas" => EditorTool::ClearGas,
        "flux.default.tool.gas_source" => EditorTool::CreateGasSource,
        "flux.default.tool.gas_sink" => EditorTool::CreateGasSink,
        _ => return None,
    })
}

fn runtime_log_level_label(level: u32) -> &'static str {
    match level {
        1 => "ERROR",
        2 => "WARN",
        3 => "INFO",
        4 => "DEBUG",
        _ => "LOG",
    }
}

fn encode_cell_entity_id(x: u32, y: u32) -> u32 {
    (((y * WORLD_WIDTH + x) + 1) << 1) | 1
}

fn decode_cell_entity_id(value: u32) -> Option<(u32, u32)> {
    if !is_cell_entity_id(value) {
        return None;
    }
    let linear = (value >> 1).checked_sub(1)?;
    Some((linear % WORLD_WIDTH, linear / WORLD_WIDTH))
}

fn is_cell_entity_id(value: u32) -> bool {
    value & 1 == 1
}

fn encode_structure_entity_id(id: PlacedStructureId) -> u32 {
    id.0 << 1
}

fn decode_structure_entity_id(value: u32) -> Option<PlacedStructureId> {
    (value != 0 && !is_cell_entity_id(value)).then_some(PlacedStructureId(value >> 1))
}

/// Returns true when one overlay should use plugin-controlled drawing.
pub fn overlay_is_plugin_controlled(
    registry: &PluginRuntimeRegistry,
    overlay_id: &ContentId,
) -> bool {
    registry
        .overlays()
        .get(overlay_id)
        .map(|descriptor| descriptor.render_policy == OverlayRenderPolicy::PluginControlled)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{
        add_gas_callback, entity_place_callback, read_save_chunk_callback,
        submit_hud_block_callback, submit_overlay_frame_callback, write_save_chunk_callback,
        PluginHudBlockStore, PluginOverlayFrameStore, RuntimeDllPluginRegistry,
        RuntimeHostContext,
    };
    use crate::{
        config::GasRegistry,
        editor::ActiveEditorTool,
        plugins::{
            abi::FluxUtf8Slice, build_plugin_runtime_registry, default_plugin, ContentId,
            LoadedPluginMetadata, PluginId, PluginSourceKind,
            SaveChunkStore,
        },
        simulation::SimulationControl,
        simulation::gas::GasField,
        world::{
            grid::WorldGrid,
            structures::PlacedStructureMap,
        },
    };
    use bevy::prelude::*;
    use std::{
        env,
        ffi::c_void,
        fs,
        path::{Path, PathBuf},
        process::Command,
    };

    fn gas_registry() -> GasRegistry {
        GasRegistry::from_substances(default_plugin::default_substance_definitions())
            .expect("default gas registry")
    }

    fn cargo_binary() -> PathBuf {
        if let Ok(path) = env::var("CARGO") {
            let path = PathBuf::from(path);
            if path.exists() {
                return path;
            }
        }

        let fallback = PathBuf::from(r"C:\Users\andreybotanic\.cargo\bin\cargo.exe");
        if fallback.exists() {
            return fallback;
        }

        PathBuf::from("cargo")
    }

    fn demo_plugin_root(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("plugins")
            .join(name)
    }

    fn build_demo_plugin_cdylib(crate_name: &str, dll_name: &str) -> PathBuf {
        let crate_root = demo_plugin_root(crate_name);
        let status = Command::new(cargo_binary())
            .arg("build")
            .arg("--manifest-path")
            .arg(crate_root.join("Cargo.toml"))
            .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")))
            .status()
            .expect("spawn cargo build for demo plugin");
        assert!(status.success(), "demo plugin cargo build failed");

        crate_root.join("target").join("debug").join(dll_name)
    }

    fn make_temp_plugin_root(prefix: &str) -> PathBuf {
        let unique = format!(
            "{}_{}_{}",
            prefix,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn copy_directory_recursive(source_dir: &Path, destination_dir: &Path) {
        fs::create_dir_all(destination_dir).expect("create destination directory");
        for entry in fs::read_dir(source_dir).expect("read source directory") {
            let entry = entry.expect("read source entry");
            let source_path = entry.path();
            let destination_path = destination_dir.join(entry.file_name());
            if source_path.is_dir() {
                copy_directory_recursive(&source_path, &destination_path);
            } else {
                if let Some(parent) = destination_path.parent() {
                    fs::create_dir_all(parent).expect("create destination parent");
                }
                fs::copy(&source_path, &destination_path).expect("copy source file");
            }
        }
    }

    fn create_working_demo_plugin_directory(crate_name: &str, dll_name: &str) -> PathBuf {
        let crate_root = demo_plugin_root(crate_name);
        let template_root = crate_root.join("package_template");
        let dll_path = build_demo_plugin_cdylib(crate_name, dll_name);
        let root = make_temp_plugin_root(crate_name);
        copy_directory_recursive(&template_root, &root);
        let bin_dir = root.join("bin");
        fs::create_dir_all(&bin_dir).expect("create plugin bin directory");
        fs::copy(&dll_path, bin_dir.join(dll_name)).expect("copy demo plugin dll");
        root
    }

    fn load_demo_plugin_from_dev_root(root: &Path) -> LoadedPluginMetadata {
        let (manifest, registration) =
            crate::plugins::validate_expanded_plugin_root(root).expect("validate dev plugin root");
        LoadedPluginMetadata {
            plugin_id: manifest.id.clone(),
            display_name: manifest.display_name.clone(),
            version: manifest.version.clone(),
            source_kind: PluginSourceKind::Dev,
            content: manifest.content,
            locked: false,
            source_name: manifest.id.as_str().to_string(),
            source_path: Some(root.to_path_buf()),
            manifest: Some(manifest),
            registration,
        }
    }

    #[test]
    fn runtime_add_gas_callback_sets_amount_and_velocity() {
        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let mut world = WorldGrid::default();
        let mut gas = GasField::from_registry(&gas_registry);
        let mut context = RuntimeHostContext::new(&content, &gas_registry);
        context.world = Some(&mut world);
        context.gas = Some(&mut gas);
        let mut added = 0u32;

        let status = unsafe {
            add_gas_callback(
                (&mut context as *mut RuntimeHostContext).cast::<c_void>(),
                20,
                20,
                FluxUtf8Slice::from_str("h2"),
                25,
                2.5,
                0.0,
                &mut added,
            )
        };

        assert!(status.is_ok());
        assert_eq!(added, 25);
        assert_eq!(
            context.gas.as_ref().expect("gas").amount_rounded(20, 20, 0),
            25
        );
        assert_eq!(
            context.gas.as_ref().expect("gas").velocity(20, 20),
            Vec2::new(2.5, 0.0)
        );
    }

    #[test]
    fn runtime_entity_place_records_changed_cell_for_render_sync() {
        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let mut world = WorldGrid::default();
        let mut gas = GasField::from_registry(&gas_registry);
        let mut context = RuntimeHostContext::new(&content, &gas_registry);
        context.world = Some(&mut world);
        context.gas = Some(&mut gas);
        let mut entity_id = 0u32;

        let status = unsafe {
            entity_place_callback(
                (&mut context as *mut RuntimeHostContext).cast::<c_void>(),
                FluxUtf8Slice::from_str("flux.default.cell.metal"),
                20,
                20,
                0,
                &mut entity_id,
            )
        };

        assert!(status.is_ok());
        assert_ne!(entity_id, 0);
        assert_eq!(context.changed_cells, vec![UVec2::new(20, 20)]);
    }

    #[test]
    fn runtime_callbacks_collect_overlay_hud_and_save_chunk_data() {
        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let mut overlay = PluginOverlayFrameStore::default();
        let mut hud = PluginHudBlockStore::default();
        let mut chunks = SaveChunkStore::default();
        let mut context = RuntimeHostContext::new(&content, &gas_registry);
        context.plugin_id = PluginId::parse("flux.api_ui_save_demo").expect("plugin id");
        context.overlay_frame = Some(&mut overlay);
        context.hud_blocks = Some(&mut hud);
        context.save_chunks = Some(&mut chunks);

        let context_ptr = (&mut context as *mut RuntimeHostContext).cast::<c_void>();
        let overlay_bytes = vec![
            128u8;
            (crate::world::grid::WORLD_WIDTH as usize)
                * (crate::world::grid::WORLD_HEIGHT as usize)
                * 4
        ];
        assert!(unsafe {
            submit_overlay_frame_callback(
                context_ptr,
                crate::world::grid::WORLD_WIDTH,
                crate::world::grid::WORLD_HEIGHT,
                overlay_bytes.as_ptr(),
                overlay_bytes.len(),
            )
        }
        .is_ok());
        assert!(unsafe {
            submit_hud_block_callback(
                context_ptr,
                FluxUtf8Slice::from_str("flux.api_ui_save_demo.hud.counter"),
                FluxUtf8Slice::from_str("Demo"),
                FluxUtf8Slice::from_str("counter 1"),
                10,
            )
        }
        .is_ok());
        assert!(unsafe {
            write_save_chunk_callback(
                context_ptr,
                FluxUtf8Slice::from_str("flux.api_ui_save_demo.save.counter"),
                1,
                7u32.to_le_bytes().as_ptr(),
                4,
            )
        }
        .is_ok());
        let mut version = 0u32;
        let mut len = 0usize;
        let mut bytes = [0u8; 4];
        assert!(unsafe {
            read_save_chunk_callback(
                context_ptr,
                FluxUtf8Slice::from_str("flux.api_ui_save_demo.save.counter"),
                &mut version,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut len,
            )
        }
        .is_ok());

        assert_eq!(
            context.overlay_frame.as_ref().expect("overlay").rgba8,
            overlay_bytes
        );
        assert_eq!(context.hud_blocks.as_ref().expect("hud").blocks.len(), 1);
        let chunk_id = ContentId::parse("flux.api_ui_save_demo.save.counter").expect("chunk id");
        let chunk = context
            .save_chunks
            .as_ref()
            .expect("chunks")
            .read_plugin_chunk(&context.plugin_id, &chunk_id)
            .expect("chunk");
        assert_eq!(chunk.bytes, 7u32.to_le_bytes().to_vec());
        assert_eq!(version, 1);
        assert_eq!(len, 4);
        assert_eq!(bytes, 7u32.to_le_bytes());
    }

    #[test]
    fn runtime_dispatches_live_tick_demo_plugin_into_gas_host_api() {
        let plugin_root =
            create_working_demo_plugin_directory("flux_api_tick_demo_plugin", "flux_api_tick_demo_plugin.dll");
        let loaded = load_demo_plugin_from_dev_root(&plugin_root);
        let runtime_registry = build_plugin_runtime_registry(&[loaded.clone()]);
        let mut runtime_plugins = RuntimeDllPluginRegistry::from_loaded_plugins(&[loaded]);
        assert!(runtime_plugins.errors().is_empty());

        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let h2_index = gas_registry.index_of("h2").expect("h2 gas index");
        let mut world = WorldGrid::default();
        let mut structures = PlacedStructureMap::default();
        let mut gas = GasField::from_registry(&gas_registry);
        let mut simulation_control = SimulationControl::default();
        let mut active_tool = ActiveEditorTool::default();
        {
            let mut context = RuntimeHostContext::new(&content, &gas_registry);
            context.world = Some(&mut world);
            context.structures = Some(&mut structures);
            context.gas = Some(&mut gas);
            context.simulation_control = Some(&mut simulation_control);
            context.simulation_step = 1;
            context.active_tool = Some(&mut active_tool);
            runtime_plugins.dispatch_event(
                &runtime_registry,
                &crate::plugins::PluginRuntimeEvent::SimulationPreCellGasStep,
                &mut context,
            );
        }

        assert_eq!(gas.amount_rounded(50, 50, h2_index), 20);

        drop(runtime_plugins);
        let _ = fs::remove_dir_all(plugin_root);
    }

    #[test]
    fn runtime_dispatches_live_ui_save_demo_plugin_mouse_and_hud_events() {
        let plugin_root = create_working_demo_plugin_directory(
            "flux_api_ui_save_demo_plugin",
            "flux_api_ui_save_demo_plugin.dll",
        );
        let loaded = load_demo_plugin_from_dev_root(&plugin_root);
        let runtime_registry = build_plugin_runtime_registry(&[loaded.clone()]);
        let mut runtime_plugins = RuntimeDllPluginRegistry::from_loaded_plugins(&[loaded]);
        assert!(runtime_plugins.errors().is_empty());

        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let mut hud = PluginHudBlockStore::default();
        let mut mouse_context = RuntimeHostContext::new(&content, &gas_registry);
        runtime_plugins.dispatch_event(
            &runtime_registry,
            &crate::plugins::PluginRuntimeEvent::MouseDownCell(crate::plugins::MouseCellEvent {
                button: Some(crate::plugins::MouseButton::Left),
                cell: UVec2::new(10, 20),
                world_position: Vec2::ZERO,
                screen_position: Vec2::ZERO,
                modifiers: crate::plugins::InputModifiers::default(),
                active_tool_id: None,
                is_over_ui: false,
            }),
            &mut mouse_context,
        );

        {
            let mut hud_context = RuntimeHostContext::new(&content, &gas_registry);
            hud_context.hud_blocks = Some(&mut hud);
            runtime_plugins.dispatch_event(
                &runtime_registry,
                &crate::plugins::PluginRuntimeEvent::BuildHudForCell {
                    cell: UVec2::new(10, 20),
                },
                &mut hud_context,
            );
        }

        assert_eq!(hud.blocks.len(), 1);
        assert_eq!(hud.blocks[0].title, "API UI/Save Demo");
        assert_eq!(hud.blocks[0].lines, vec!["cell left-clicks 1, cell (10, 20)"]);

        drop(runtime_plugins);
        let _ = fs::remove_dir_all(plugin_root);
    }
}

fn dispatch_queued_runtime_plugin_events(
    mut events: EventReader<PluginRuntimeEvent>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<PlacedStructureMap>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    mut simulation_control: ResMut<SimulationControl>,
    simulation_step: Res<SimulationStep>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut save_chunks: ResMut<SaveChunkStore>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    for event in events.read() {
        if matches!(
            event.kind(),
            PluginEvent::RenderOverlay | PluginEvent::BuildHudForCell
        ) {
            continue;
        }
        let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
        context.world = Some(&mut world);
        context.structures = Some(&mut structures);
        context.gas = Some(&mut gas);
        context.gpu_state = Some(&mut gpu_state);
        context.simulation_control = Some(&mut simulation_control);
        context.simulation_step = simulation_step.0;
        context.active_tool = Some(&mut active_tool);
        context.save_chunks = Some(&mut save_chunks);
        runtime_plugins.dispatch_event(&runtime_registry, event, &mut context);
        flush_changed_cells(&context, &mut world_changed);
    }
}

fn dispatch_runtime_plugin_pre_gas_step(
    world_load_state: Res<WorldLoadState>,
    mut control: ResMut<SimulationControl>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<PlacedStructureMap>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    simulation_step: Res<SimulationStep>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    if !world_load_state.has_world || control.paused {
        return;
    }
    let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
    context.world = Some(&mut world);
    context.structures = Some(&mut structures);
    context.gas = Some(&mut gas);
    context.gpu_state = Some(&mut gpu_state);
    context.simulation_control = Some(&mut control);
    context.simulation_step = simulation_step.0;
    context.active_tool = Some(&mut active_tool);
    runtime_plugins.dispatch_event(
        &runtime_registry,
        &PluginRuntimeEvent::SimulationPreCellGasStep,
        &mut context,
    );
    flush_changed_cells(&context, &mut world_changed);
}

fn dispatch_runtime_plugin_post_gas_step(
    world_load_state: Res<WorldLoadState>,
    mut control: ResMut<SimulationControl>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut structures: ResMut<PlacedStructureMap>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    simulation_step: Res<SimulationStep>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    if !world_load_state.has_world || control.paused {
        return;
    }
    let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
    context.world = Some(&mut world);
    context.structures = Some(&mut structures);
    context.gas = Some(&mut gas);
    context.gpu_state = Some(&mut gpu_state);
    context.simulation_control = Some(&mut control);
    context.simulation_step = simulation_step.0;
    context.active_tool = Some(&mut active_tool);
    runtime_plugins.dispatch_event(
        &runtime_registry,
        &PluginRuntimeEvent::SimulationPostCellGasStep,
        &mut context,
    );
    flush_changed_cells(&context, &mut world_changed);
}

fn flush_changed_cells(
    context: &RuntimeHostContext,
    world_changed: &mut EventWriter<WorldCellChanged>,
) {
    for cell in &context.changed_cells {
        world_changed.write(WorldCellChanged { cell: *cell });
    }
}

fn dispatch_plugin_overlay_render(
    overlay_mode: Res<OverlayMode>,
    world_load_state: Res<WorldLoadState>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut simulation_control: ResMut<SimulationControl>,
    simulation_step: Res<SimulationStep>,
    mut active_tool: ResMut<ActiveEditorTool>,
    mut overlay_frame: ResMut<PluginOverlayFrameStore>,
    overlay_image: Res<PluginOverlayImage>,
    mut images: ResMut<Assets<Image>>,
    mut overlay_sprites: Query<&mut Visibility, With<PluginOverlaySprite>>,
) {
    overlay_frame.clear();
    if !world_load_state.has_world {
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Hidden);
        return;
    }
    let OverlayMode::Plugin(raw_overlay_id) = *overlay_mode else {
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Hidden);
        return;
    };
    let Ok(overlay_id) = ContentId::parse(raw_overlay_id) else {
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Hidden);
        return;
    };
    if !overlay_is_plugin_controlled(&runtime_registry, &overlay_id) {
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Hidden);
        return;
    }

    {
        let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
        context.overlay_frame = Some(&mut overlay_frame);
        context.simulation_control = Some(&mut simulation_control);
        context.simulation_step = simulation_step.0;
        context.active_tool = Some(&mut active_tool);
        runtime_plugins.dispatch_event(
            &runtime_registry,
            &PluginRuntimeEvent::RenderOverlay {
                overlay_id: overlay_id.clone(),
            },
            &mut context,
        );
    }

    if overlay_frame.has_frame() {
        if let Some(image) = images.get_mut(&overlay_image.texture) {
            if let Some(data) = image.data.as_mut() {
                data.clear();
                data.extend_from_slice(&overlay_frame.rgba8);
            }
        }
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Visible);
    } else {
        set_plugin_overlay_visibility(&mut overlay_sprites, Visibility::Hidden);
    }
}

fn set_plugin_overlay_visibility(
    overlay_sprites: &mut Query<&mut Visibility, With<PluginOverlaySprite>>,
    visibility: Visibility,
) {
    for mut sprite_visibility in overlay_sprites.iter_mut() {
        *sprite_visibility = visibility;
    }
}
