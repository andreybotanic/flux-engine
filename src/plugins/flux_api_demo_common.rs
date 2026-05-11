use std::ffi::c_void;

use bevy_math::{UVec2, Vec2};

/// ABI version used by the v4 demo plugins.
pub const ENGINE_PLUGIN_API_VERSION: u32 = 4;

#[repr(C)]
#[derive(Clone, Copy)]
/// Borrowed UTF-8 string slice passed across the plugin ABI.
pub struct FluxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl FluxUtf8Slice {
    /// Builds an ABI slice from a Rust string.
    pub fn from_str(value: &str) -> Self {
        if value.is_empty() {
            return Self {
                ptr: std::ptr::null(),
                len: 0,
            };
        }
        Self {
            ptr: value.as_ptr(),
            len: value.len(),
        }
    }

    /// Copies this ABI UTF-8 slice into an owned Rust string.
    pub fn try_to_string(self) -> Result<String, FluxStatus> {
        if self.len == 0 {
            return Ok(String::new());
        }
        if self.ptr.is_null() {
            return Err(FluxStatus::INVALID_ARGUMENT);
        }
        let bytes = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
        std::str::from_utf8(bytes)
            .map(str::to_string)
            .map_err(|_| FluxStatus::INVALID_ARGUMENT)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
/// C-compatible status code returned by demo plugin callbacks.
pub struct FluxStatus(i32);

impl FluxStatus {
    /// Successful callback result.
    pub const OK: Self = Self(0);
    /// Invalid pointer or argument callback result.
    pub const INVALID_ARGUMENT: Self = Self(1);
    /// Generic callback failure result.
    pub const FAILED: Self = Self(2);

    /// Returns `true` when the ABI call succeeded.
    pub fn is_ok(self) -> bool {
        self == Self::OK
    }

    /// Converts the raw ABI status into a Rust result.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_ok() { Ok(()) } else { Err(self) }
    }
}

/// Host callback used by plugins to report a textual error.
pub type FluxWriteErrorFn =
    unsafe extern "C" fn(context: *mut c_void, message: FluxUtf8Slice) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Host API table received by a runtime plugin during creation.
pub struct FluxHostApi {
    pub struct_size: u32,
    pub api_version: u32,
    pub plugin_root: FluxUtf8Slice,
    pub config_root: FluxUtf8Slice,
    pub assets_root: FluxUtf8Slice,
    pub write_error: Option<FluxWriteErrorFn>,
    pub error_context: *mut c_void,
}

impl FluxHostApi {
    /// Returns `true` when the host payload matches the expected ABI layout.
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<FluxHostApi>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION
    }

    /// Reports one human-readable startup error back to the engine.
    pub fn write_error(&self, message: &str) -> Result<(), FluxStatus> {
        let Some(callback) = self.write_error else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.error_context, FluxUtf8Slice::from_str(message)) }.into_result()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Gas substance descriptor accepted by the v4 registrar.
pub struct FluxGasSubstanceDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub alias: FluxUtf8Slice,
    pub molecular_mass: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
}

/// Registrar callback for plugin-owned gas substance declarations.
pub type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Tool descriptor accepted by the v4 registrar.
pub struct FluxToolDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub icon_path: FluxUtf8Slice,
    pub silhouette_path: FluxUtf8Slice,
}

/// Registrar callback for plugin-owned tool declarations.
pub type FluxRegisterToolFn =
    unsafe extern "C" fn(context: *mut c_void, descriptor: *const FluxToolDescriptor) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Overlay descriptor accepted by the v4 registrar.
pub struct FluxOverlayDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub hotkey: FluxUtf8Slice,
    pub render_policy: u32,
}

/// Registrar callback for plugin-owned overlay declarations.
pub type FluxRegisterOverlayFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Save chunk descriptor accepted by the v4 registrar.
pub struct FluxSaveChunkDescriptor {
    pub id: FluxUtf8Slice,
    pub version: u32,
}

/// Registrar callback for plugin-owned save chunk declarations.
pub type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Stable plugin-visible event kind used by the demo ABI shim.
pub enum FluxEventKind {
    WorldCreated = 0,
    WorldLoaded = 1,
    WorldBeforeSave = 2,
    WorldAfterSave = 3,
    WorldUnloaded = 4,
    SimulationPreCellGasStep = 5,
    SimulationPostCellGasStep = 6,
    SimulationPausedChanged = 7,
    StructurePlaced = 8,
    StructureRemoved = 9,
    ToolSelected = 10,
    MouseDownCell = 11,
    MouseMoveCell = 12,
    MouseUpCell = 13,
    MouseEnterCell = 14,
    MouseLeaveCell = 15,
    KeyPressed = 16,
    KeyReleased = 17,
    OverlayChanged = 18,
    BuildHudForCell = 19,
    BuildPanel = 20,
    RenderOverlay = 21,
}

impl FluxEventKind {
    /// Returns the raw ABI tag used by the demo registration shim.
    pub fn as_raw(self) -> u32 {
        self as u32
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Event-handler descriptor accepted by the v4 registrar.
pub struct FluxEventHandlerDescriptor {
    pub event_kind: u32,
    pub handler_name: FluxUtf8Slice,
}

impl FluxEventHandlerDescriptor {
    /// Creates one event-handler descriptor from the typed demo event enum.
    pub fn new(event_kind: FluxEventKind, handler_name: FluxUtf8Slice) -> Self {
        Self {
            event_kind: event_kind.as_raw(),
            handler_name,
        }
    }
}

/// Registrar callback for plugin-owned event handler declarations.
pub type FluxRegisterEventHandlerFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxEventHandlerDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// ABI registration table passed by the host into `flux_plugin_register`.
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub register_gas_substance: Option<FluxRegisterGasSubstanceFn>,
    pub register_event_handler: Option<FluxRegisterEventHandlerFn>,
    pub register_tool: Option<FluxRegisterToolFn>,
    pub register_overlay: Option<FluxRegisterOverlayFn>,
    pub register_save_chunk: Option<FluxRegisterSaveChunkFn>,
    pub registration_context: *mut c_void,
    pub reserved2: *mut c_void,
    pub reserved3: *mut c_void,
}

impl FluxRegistrar {
    /// Returns `true` when the registrar payload matches the expected ABI layout.
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<FluxRegistrar>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION
    }

    /// Registers one gas-capable substance descriptor.
    pub fn register_gas_substance(
        &mut self,
        descriptor: &FluxGasSubstanceDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_gas_substance else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one event handler binding by event kind and export name.
    pub fn register_event_handler(
        &mut self,
        event_kind: FluxEventKind,
        handler_name: &str,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_event_handler else {
            return Err(FluxStatus::FAILED);
        };
        let descriptor =
            FluxEventHandlerDescriptor::new(event_kind, FluxUtf8Slice::from_str(handler_name));
        unsafe { callback(self.registration_context, &descriptor) }.into_result()
    }

    /// Registers one tool descriptor.
    pub fn register_tool(&mut self, descriptor: &FluxToolDescriptor) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_tool else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one overlay descriptor.
    pub fn register_overlay(
        &mut self,
        descriptor: &FluxOverlayDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_overlay else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one save chunk descriptor.
    pub fn register_save_chunk(
        &mut self,
        descriptor: &FluxSaveChunkDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_save_chunk else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }
}

#[repr(C)]
/// Opaque demo plugin instance handle.
pub struct FluxPluginHandle {
    pub api_version: u32,
    pub dragging: bool,
    pub last_x: u32,
    pub last_y: u32,
    pub counter: u32,
    pub cell_counters: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Common ABI header shared by all typed demo event payloads.
pub struct FluxEventHeader {
    pub struct_size: u32,
    pub api_version: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Empty payload used by lifecycle and tick demo handlers.
pub struct FluxEmptyEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Mouse-cell payload used by demo handlers.
pub struct FluxMouseCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub button: u32,
    pub has_cell: u8,
    pub cell_x: u32,
    pub cell_y: u32,
    pub world_x: f32,
    pub world_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub modifiers: u32,
    pub has_active_tool_id: u8,
    pub active_tool_id: FluxUtf8Slice,
    pub is_over_ui: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// HUD build payload used by demo handlers.
pub struct FluxBuildHudForCellEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub cell_x: u32,
    pub cell_y: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Panel build payload used by demo handlers.
pub struct FluxBuildPanelEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub panel_id: FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Overlay render payload used by demo handlers.
pub struct FluxRenderOverlayEventPayload {
    pub struct_size: u32,
    pub api_version: u32,
    pub overlay_id: FluxUtf8Slice,
}

/// Host callback used by demo plugins to set a world cell material.
pub type FluxSetCellMaterialFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    material_id: FluxUtf8Slice,
) -> FluxStatus;

/// Host callback used by demo plugins to add gas with velocity.
pub type FluxAddGasFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    amount: u32,
    velocity_x: f32,
    velocity_y: f32,
    out_added: *mut u32,
) -> FluxStatus;

/// Host callback used by demo plugins to submit one complete RGBA8 overlay frame.
pub type FluxSubmitOverlayFrameFn = unsafe extern "C" fn(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus;

/// Host callback used by demo plugins to submit one HUD block.
pub type FluxSubmitHudBlockFn = unsafe extern "C" fn(
    context: *mut c_void,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
) -> FluxStatus;

/// Host callback used by demo plugins to write one save chunk.
pub type FluxWriteSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus;

/// Host callback used by demo plugins to read one plugin-owned save chunk.
pub type FluxReadSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Runtime host callback table passed by the engine to demo plugins.
pub struct FluxRuntimeHost {
    pub struct_size: u32,
    pub api_version: u32,
    pub context: *mut c_void,
    pub set_cell_material: Option<FluxSetCellMaterialFn>,
    pub add_gas: Option<FluxAddGasFn>,
    pub submit_overlay_frame: Option<FluxSubmitOverlayFrameFn>,
    pub submit_hud_block: Option<FluxSubmitHudBlockFn>,
    pub write_save_chunk: Option<FluxWriteSaveChunkFn>,
    pub read_save_chunk: Option<FluxReadSaveChunkFn>,
}

impl FluxRuntimeHost {
    /// Returns `true` when the runtime host payload matches the expected ABI layout.
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<FluxRuntimeHost>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION
    }

    /// Sets one editable world cell to the requested material id.
    pub fn set_cell_material(
        &mut self,
        cell: UVec2,
        material_id: &str,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.set_cell_material else {
            return Err(FluxStatus::FAILED);
        };
        unsafe {
            callback(
                self.context,
                cell.x,
                cell.y,
                FluxUtf8Slice::from_str(material_id),
            )
        }
        .into_result()
    }

    /// Adds free gas with velocity and returns the amount accepted by the host.
    pub fn add_gas(
        &mut self,
        cell: UVec2,
        substance: &str,
        amount: u32,
        velocity: Vec2,
    ) -> Result<u32, FluxStatus> {
        let Some(callback) = self.add_gas else {
            return Err(FluxStatus::FAILED);
        };
        let mut added = 0u32;
        unsafe {
            callback(
                self.context,
                cell.x,
                cell.y,
                FluxUtf8Slice::from_str(substance),
                amount,
                velocity.x,
                velocity.y,
                &mut added,
            )
        }
        .into_result()?;
        Ok(added)
    }

    /// Submits one complete RGBA8 overlay frame.
    pub fn submit_overlay_frame(
        &mut self,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.submit_overlay_frame else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.context, width, height, rgba8.as_ptr(), rgba8.len()) }.into_result()
    }

    /// Appends one HUD block line.
    pub fn submit_hud_block(&mut self, title: &str, line: &str) -> Result<(), FluxStatus> {
        let Some(callback) = self.submit_hud_block else {
            return Err(FluxStatus::FAILED);
        };
        unsafe {
            callback(
                self.context,
                FluxUtf8Slice::from_str(title),
                FluxUtf8Slice::from_str(line),
            )
        }
        .into_result()
    }

    /// Writes one plugin-owned save chunk.
    pub fn write_save_chunk(
        &mut self,
        chunk_id: &str,
        version: u32,
        bytes: &[u8],
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.write_save_chunk else {
            return Err(FluxStatus::FAILED);
        };
        unsafe {
            callback(
                self.context,
                FluxUtf8Slice::from_str(chunk_id),
                version,
                bytes.as_ptr(),
                bytes.len(),
            )
        }
        .into_result()
    }

    /// Reads one plugin-owned save chunk and returns its version plus payload bytes.
    pub fn read_save_chunk(&mut self, chunk_id: &str) -> Result<Option<(u32, Vec<u8>)>, FluxStatus> {
        let Some(callback) = self.read_save_chunk else {
            return Err(FluxStatus::FAILED);
        };
        let chunk_id = FluxUtf8Slice::from_str(chunk_id);
        let mut version = 0u32;
        let mut required_len = 0usize;
        let status = unsafe {
            callback(
                self.context,
                chunk_id,
                &mut version,
                std::ptr::null_mut(),
                0,
                &mut required_len,
            )
        };
        if status.is_ok() {
            return Ok(Some((version, Vec::new())));
        }
        if required_len == 0 {
            return Ok(None);
        }
        let mut bytes = vec![0u8; required_len];
        let mut actual_version = 0u32;
        let mut actual_len = 0usize;
        unsafe {
            callback(
                self.context,
                chunk_id,
                &mut actual_version,
                bytes.as_mut_ptr(),
                bytes.len(),
                &mut actual_len,
            )
        }
        .into_result()?;
        bytes.truncate(actual_len);
        Ok(Some((actual_version, bytes)))
    }
}

/// Declarative event binding used by each small API demo plugin.
pub struct DemoEventHandler {
    pub event_kind: FluxEventKind,
    pub handler_name: &'static str,
}

/// Declarative registration data used by each small API demo plugin.
pub struct DemoSpec {
    pub event_handlers: &'static [DemoEventHandler],
    pub tool: Option<(&'static str, &'static str)>,
    pub overlay: Option<(&'static str, &'static str, &'static str, u32)>,
    pub save_chunk: Option<(&'static str, u32)>,
}

/// Creates a demo plugin handle after validating the v4 host API table.
pub unsafe fn create(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus {
    if host.is_null() || out_plugin.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let host = &*host;
    if !host.is_compatible() {
        return FluxStatus::FAILED;
    }
    *out_plugin = Box::into_raw(Box::new(FluxPluginHandle {
        api_version: ENGINE_PLUGIN_API_VERSION,
        dragging: false,
        last_x: 0,
        last_y: 0,
        counter: 0,
        cell_counters: vec![0; 102 * 102],
    }));
    FluxStatus::OK
}

/// Registers the descriptors and event handlers described by `spec`.
pub unsafe fn register(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
    spec: &DemoSpec,
) -> FluxStatus {
    if plugin.is_null() || registrar.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let plugin = &mut *plugin;
    let registrar = &mut *registrar;
    if plugin.api_version != ENGINE_PLUGIN_API_VERSION || !registrar.is_compatible() {
        return FluxStatus::FAILED;
    }
    for handler in spec.event_handlers {
        if let Err(status) =
            registrar.register_event_handler(handler.event_kind, handler.handler_name)
        {
            return status;
        }
    }
    if let Some((id, label)) = spec.tool {
        let descriptor = FluxToolDescriptor {
            id: FluxUtf8Slice::from_str(id),
            label: FluxUtf8Slice::from_str(label),
            icon_path: FluxUtf8Slice::from_str("assets/placeholder.txt"),
            silhouette_path: FluxUtf8Slice::from_str(""),
        };
        if let Err(status) = registrar.register_tool(&descriptor) {
            return status;
        }
    }
    if let Some((id, label, hotkey, render_policy)) = spec.overlay {
        let descriptor = FluxOverlayDescriptor {
            id: FluxUtf8Slice::from_str(id),
            label: FluxUtf8Slice::from_str(label),
            hotkey: FluxUtf8Slice::from_str(hotkey),
            render_policy,
        };
        if let Err(status) = registrar.register_overlay(&descriptor) {
            return status;
        }
    }
    if let Some((id, version)) = spec.save_chunk {
        let descriptor = FluxSaveChunkDescriptor {
            id: FluxUtf8Slice::from_str(id),
            version,
        };
        if let Err(status) = registrar.register_save_chunk(&descriptor) {
            return status;
        }
    }
    FluxStatus::OK
}

/// Releases a plugin handle previously returned by `create`.
pub unsafe fn destroy(plugin: *mut FluxPluginHandle) {
    if !plugin.is_null() {
        let _ = Box::from_raw(plugin);
    }
}

/// Validates one typed event callback payload and host table.
pub unsafe fn validate_event_call<'a, T>(
    plugin: *mut FluxPluginHandle,
    event: *const T,
    host: *mut FluxRuntimeHost,
) -> Result<(&'a mut FluxPluginHandle, &'a T, &'a mut FluxRuntimeHost), FluxStatus> {
    if plugin.is_null() || event.is_null() || host.is_null() {
        return Err(FluxStatus::INVALID_ARGUMENT);
    }
    let plugin = &mut *plugin;
    let event = &*event;
    let host = &mut *host;
    let header = &*(event as *const T).cast::<FluxEventHeader>();
    if plugin.api_version != ENGINE_PLUGIN_API_VERSION
        || header.api_version != ENGINE_PLUGIN_API_VERSION
        || !host.is_compatible()
    {
        return Err(FluxStatus::FAILED);
    }
    Ok((plugin, event, host))
}
