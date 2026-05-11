use std::{ffi::c_void, path::PathBuf, ptr};

use bevy::prelude::*;
use libloading::Library;

use crate::{
    config::GasRegistry,
    plugins::{
        abi::{
            FluxPluginDestroyFn, FluxPluginHandle, FluxPluginOnEventFn, FluxRuntimeEvent,
            FluxRuntimeHost, FluxStatus, FluxUtf8Slice,
        },
        api::{
            events::{
                InputModifiers, MouseCellButton, MouseCellEvent, PluginEvent, PluginEventKind,
            },
            render_api::OverlayRenderPolicy,
            runtime::PluginRuntimeRegistry,
            save_api::SaveChunkStore,
            ui_api::HudBlock,
        },
        ContentId, ContentRegistry, LoadedPluginMetadata, PluginId,
    },
    render::{
        world_view::{PluginOverlayImage, PluginOverlaySprite},
        OverlayMode,
    },
    save::WorldLoadState,
    simulation::{gas::GasField, GpuRuntimeState, SimulationControl, SimulationSet},
    world::{
        grid::{is_editable_cell, CellKind, CellMaterial, WorldGrid, WORLD_HEIGHT, WORLD_WIDTH},
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
    pub fn dispatch_event(
        &mut self,
        subscriptions: &PluginRuntimeRegistry,
        event: &PluginEvent,
        context: &mut RuntimeHostContext,
    ) {
        let subscribers = subscriptions.subscribers(event.kind());
        if subscribers.is_empty() {
            return;
        }
        let Some(event_kind) = event_kind_to_abi(event.kind()) else {
            return;
        };

        for plugin in &mut self.plugins {
            if !subscribers
                .iter()
                .any(|plugin_id| plugin_id == &plugin.plugin_id)
            {
                continue;
            }
            let Some(on_event) = plugin.on_event_fn else {
                continue;
            };
            context.plugin_id = plugin.plugin_id.clone();
            dispatch_to_plugin(plugin.handle, on_event, event, event_kind, context);
        }
    }
}

/// One live DLL plugin instance and its loaded library.
pub struct RuntimeDllPlugin {
    pub plugin_id: PluginId,
    pub handle: *mut FluxPluginHandle,
    pub destroy_fn: FluxPluginDestroyFn,
    pub on_event_fn: Option<FluxPluginOnEventFn>,
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
    pub gas: Option<&'a mut GasField>,
    pub gpu_state: Option<&'a mut GpuRuntimeState>,
    pub overlay_frame: Option<&'a mut PluginOverlayFrameStore>,
    pub hud_blocks: Option<&'a mut PluginHudBlockStore>,
    pub save_chunks: Option<&'a mut SaveChunkStore>,
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
            gas: None,
            gpu_state: None,
            overlay_frame: None,
            hud_blocks: None,
            save_chunks: None,
            changed_cells: Vec::new(),
        }
    }
}

/// ABI callback that sets one world cell material by stable content id.
pub unsafe extern "C" fn set_cell_material_callback(
    context: *mut c_void,
    x: u32,
    y: u32,
    material_id: FluxUtf8Slice,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    if x >= WORLD_WIDTH || y >= WORLD_HEIGHT || !is_editable_cell(x, y) {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Ok(material_id) = read_abi_utf8(material_id) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let material = CellMaterial::from_registered_id(material_id);
    if context
        .content_registry
        .cell_by_material(material)
        .is_none()
    {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let Some(world) = context.world.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let changed = world.set_cell_kind(x, y, CellKind::Solid(material));
    if changed {
        if let Some(gas) = context.gas.as_deref_mut() {
            gas.clear_cell(x, y);
        }
        if let Some(gpu_state) = context.gpu_state.as_deref_mut() {
            gpu_state.mark_needs_full_upload();
        }
        context.changed_cells.push(UVec2::new(x, y));
    }
    FluxStatus::OK
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
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
) -> FluxStatus {
    let Some(context) = context.cast::<RuntimeHostContext>().as_mut() else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let (Ok(title), Ok(line)) = (read_abi_utf8(title), read_abi_utf8(line)) else {
        return FluxStatus::INVALID_ARGUMENT;
    };
    let Some(store) = context.hud_blocks.as_deref_mut() else {
        return FluxStatus::FAILED;
    };
    let id = match ContentId::parse(&format!("{}.hud.runtime", context.plugin_id.as_str())) {
        Ok(id) => id,
        Err(_) => return FluxStatus::FAILED,
    };
    store.blocks.push(HudBlock {
        id,
        title,
        lines: vec![line],
        sort_order: 1000,
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

fn dispatch_to_plugin(
    handle: *mut FluxPluginHandle,
    on_event: FluxPluginOnEventFn,
    event: &PluginEvent,
    event_kind: u32,
    context: &mut RuntimeHostContext,
) {
    let active_tool_id = active_tool_id_for_event(event);
    let overlay_id = overlay_id_for_event(event);
    let key = key_for_event(event);
    let mut abi_event = FluxRuntimeEvent::new(event_kind);
    abi_event.active_tool_id = FluxUtf8Slice::from_str(&active_tool_id);
    abi_event.overlay_id = FluxUtf8Slice::from_str(&overlay_id);
    abi_event.key = FluxUtf8Slice::from_str(&key);
    apply_event_payload(event, &mut abi_event);

    let mut host = FluxRuntimeHost::new((context as *mut RuntimeHostContext).cast::<c_void>());
    unsafe {
        let _ = on_event(handle, &abi_event, &mut host);
    }
}

fn apply_event_payload(event: &PluginEvent, abi_event: &mut FluxRuntimeEvent) {
    match event {
        PluginEvent::MouseDownCell(mouse)
        | PluginEvent::MouseMoveCell(mouse)
        | PluginEvent::MouseUpCell(mouse)
        | PluginEvent::MouseEnterCell(mouse)
        | PluginEvent::MouseLeaveCell(mouse) => apply_mouse_payload(mouse, abi_event),
        PluginEvent::BuildHudForCell { cell } => {
            abi_event.has_cell = 1;
            abi_event.cell_x = cell.x;
            abi_event.cell_y = cell.y;
        }
        _ => {}
    }
}

fn apply_mouse_payload(mouse: &MouseCellEvent, abi_event: &mut FluxRuntimeEvent) {
    abi_event.has_cell = 1;
    abi_event.cell_x = mouse.cell.x;
    abi_event.cell_y = mouse.cell.y;
    abi_event.button = mouse.button.map(mouse_button_to_abi).unwrap_or(0);
    abi_event.world_x = mouse.world_position.x;
    abi_event.world_y = mouse.world_position.y;
    abi_event.screen_x = mouse.screen_position.x;
    abi_event.screen_y = mouse.screen_position.y;
    abi_event.modifiers = modifiers_to_abi(mouse.modifiers);
}

fn active_tool_id_for_event(event: &PluginEvent) -> String {
    match event {
        PluginEvent::MouseDownCell(mouse)
        | PluginEvent::MouseMoveCell(mouse)
        | PluginEvent::MouseUpCell(mouse)
        | PluginEvent::MouseEnterCell(mouse)
        | PluginEvent::MouseLeaveCell(mouse) => mouse
            .active_tool_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default(),
        PluginEvent::ToolSelected { tool_id } => tool_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn overlay_id_for_event(event: &PluginEvent) -> String {
    match event {
        PluginEvent::OverlayChanged { overlay_id } => overlay_id
            .as_ref()
            .map(|id| id.as_str().to_string())
            .unwrap_or_default(),
        PluginEvent::RenderOverlay { overlay_id } => overlay_id.as_str().to_string(),
        _ => String::new(),
    }
}

fn key_for_event(event: &PluginEvent) -> String {
    match event {
        PluginEvent::KeyPressed { key, .. } | PluginEvent::KeyReleased { key, .. } => key.clone(),
        _ => String::new(),
    }
}

fn mouse_button_to_abi(button: MouseCellButton) -> u32 {
    match button {
        MouseCellButton::Left => 1,
        MouseCellButton::Right => 2,
        MouseCellButton::Middle => 3,
        MouseCellButton::Other(value) => 1000 + value as u32,
    }
}

fn modifiers_to_abi(modifiers: InputModifiers) -> u32 {
    let mut bits = 0u32;
    if modifiers.shift {
        bits |= 1;
    }
    if modifiers.ctrl {
        bits |= 2;
    }
    if modifiers.alt {
        bits |= 4;
    }
    bits
}

/// Converts a plugin event kind to the stable v3 ABI numeric tag.
pub fn event_kind_to_abi(kind: PluginEventKind) -> Option<u32> {
    Some(match kind {
        PluginEventKind::WorldCreated => 0,
        PluginEventKind::WorldLoaded => 1,
        PluginEventKind::WorldBeforeSave => 2,
        PluginEventKind::WorldAfterSave => 3,
        PluginEventKind::WorldUnloaded => 4,
        PluginEventKind::SimulationPreCellGasStep => 5,
        PluginEventKind::SimulationPostCellGasStep => 6,
        PluginEventKind::SimulationPausedChanged => 7,
        PluginEventKind::StructurePlaced => 8,
        PluginEventKind::StructureRemoved => 9,
        PluginEventKind::ToolSelected => 10,
        PluginEventKind::MouseDownCell => 11,
        PluginEventKind::MouseMoveCell => 12,
        PluginEventKind::MouseUpCell => 13,
        PluginEventKind::MouseEnterCell => 14,
        PluginEventKind::MouseLeaveCell => 15,
        PluginEventKind::KeyPressed => 16,
        PluginEventKind::KeyReleased => 17,
        PluginEventKind::OverlayChanged => 18,
        PluginEventKind::BuildHudForCell => 19,
        PluginEventKind::BuildPanel => 20,
        PluginEventKind::RenderOverlay => 21,
    })
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
        add_gas_callback, apply_event_payload, read_save_chunk_callback,
        set_cell_material_callback, submit_hud_block_callback, submit_overlay_frame_callback,
        write_save_chunk_callback, PluginHudBlockStore, PluginOverlayFrameStore,
        RuntimeHostContext,
    };
    use crate::{
        config::GasRegistry,
        plugins::{
            abi::{FluxRuntimeEvent, FluxUtf8Slice},
            default_plugin, ContentId, PluginEvent, PluginId, SaveChunkStore,
        },
        simulation::gas::GasField,
        world::grid::WorldGrid,
    };
    use bevy::prelude::*;
    use std::ffi::c_void;

    fn gas_registry() -> GasRegistry {
        GasRegistry::from_substances(default_plugin::default_substance_definitions())
            .expect("default gas registry")
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
    fn runtime_set_cell_material_records_changed_cell_for_render_sync() {
        let content = default_plugin::default_content_registry();
        let gas_registry = gas_registry();
        let mut world = WorldGrid::default();
        let mut context = RuntimeHostContext::new(&content, &gas_registry);
        context.world = Some(&mut world);

        let status = unsafe {
            set_cell_material_callback(
                (&mut context as *mut RuntimeHostContext).cast::<c_void>(),
                20,
                20,
                FluxUtf8Slice::from_str("flux.default.cell.metal"),
            )
        };

        assert!(status.is_ok());
        assert_eq!(context.changed_cells, vec![UVec2::new(20, 20)]);
    }

    #[test]
    fn build_hud_event_payload_contains_cell() {
        let mut event = FluxRuntimeEvent::new(19);
        apply_event_payload(
            &PluginEvent::BuildHudForCell {
                cell: UVec2::new(7, 9),
            },
            &mut event,
        );

        assert_eq!(event.has_cell, 1);
        assert_eq!(event.cell_x, 7);
        assert_eq!(event.cell_y, 9);
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
                FluxUtf8Slice::from_str("Demo"),
                FluxUtf8Slice::from_str("counter 1"),
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
}

fn dispatch_queued_runtime_plugin_events(
    mut events: EventReader<PluginEvent>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    mut save_chunks: ResMut<SaveChunkStore>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    for event in events.read() {
        if matches!(
            event.kind(),
            PluginEventKind::RenderOverlay | PluginEventKind::BuildHudForCell
        ) {
            continue;
        }
        let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
        context.world = Some(&mut world);
        context.gas = Some(&mut gas);
        context.gpu_state = Some(&mut gpu_state);
        context.save_chunks = Some(&mut save_chunks);
        runtime_plugins.dispatch_event(&runtime_registry, event, &mut context);
        flush_changed_cells(&context, &mut world_changed);
    }
}

fn dispatch_runtime_plugin_pre_gas_step(
    world_load_state: Res<WorldLoadState>,
    control: Res<SimulationControl>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    if !world_load_state.has_world || control.paused {
        return;
    }
    let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
    context.world = Some(&mut world);
    context.gas = Some(&mut gas);
    context.gpu_state = Some(&mut gpu_state);
    runtime_plugins.dispatch_event(
        &runtime_registry,
        &PluginEvent::SimulationPreCellGasStep,
        &mut context,
    );
    flush_changed_cells(&context, &mut world_changed);
}

fn dispatch_runtime_plugin_post_gas_step(
    world_load_state: Res<WorldLoadState>,
    control: Res<SimulationControl>,
    mut runtime_plugins: ResMut<RuntimeDllPluginRegistry>,
    runtime_registry: Res<PluginRuntimeRegistry>,
    content_registry: Res<ContentRegistry>,
    gas_registry: Res<GasRegistry>,
    mut world: ResMut<WorldGrid>,
    mut gas: ResMut<GasField>,
    mut gpu_state: ResMut<GpuRuntimeState>,
    mut world_changed: EventWriter<WorldCellChanged>,
) {
    if !world_load_state.has_world || control.paused {
        return;
    }
    let mut context = RuntimeHostContext::new(&content_registry, &gas_registry);
    context.world = Some(&mut world);
    context.gas = Some(&mut gas);
    context.gpu_state = Some(&mut gpu_state);
    runtime_plugins.dispatch_event(
        &runtime_registry,
        &PluginEvent::SimulationPostCellGasStep,
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
        runtime_plugins.dispatch_event(
            &runtime_registry,
            &PluginEvent::RenderOverlay {
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
