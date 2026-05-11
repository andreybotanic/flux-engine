use std::ffi::c_void;

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
    if host.struct_size != std::mem::size_of::<FluxHostApi>() as u32
        || host.api_version != ENGINE_PLUGIN_API_VERSION
    {
        return FluxStatus::FAILED;
    }
    *out_plugin = Box::into_raw(Box::new(FluxPluginHandle {
        api_version: host.api_version,
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
    if plugin.api_version != ENGINE_PLUGIN_API_VERSION
        || registrar.struct_size != std::mem::size_of::<FluxRegistrar>() as u32
        || registrar.api_version != ENGINE_PLUGIN_API_VERSION
    {
        return FluxStatus::FAILED;
    }
    for handler in spec.event_handlers {
        let Some(register_event_handler) = registrar.register_event_handler else {
            return FluxStatus::FAILED;
        };
        let descriptor = FluxEventHandlerDescriptor::new(
            handler.event_kind,
            FluxUtf8Slice::from_str(handler.handler_name),
        );
        let status = register_event_handler(registrar.registration_context, &descriptor);
        if status != FluxStatus::OK {
            return status;
        }
    }
    if let Some((id, label)) = spec.tool {
        let Some(register_tool) = registrar.register_tool else {
            return FluxStatus::FAILED;
        };
        let descriptor = FluxToolDescriptor {
            id: FluxUtf8Slice::from_str(id),
            label: FluxUtf8Slice::from_str(label),
            icon_path: FluxUtf8Slice::from_str("assets/placeholder.txt"),
            silhouette_path: FluxUtf8Slice::from_str(""),
        };
        let status = register_tool(registrar.registration_context, &descriptor);
        if status != FluxStatus::OK {
            return status;
        }
    }
    if let Some((id, label, hotkey, render_policy)) = spec.overlay {
        let Some(register_overlay) = registrar.register_overlay else {
            return FluxStatus::FAILED;
        };
        let descriptor = FluxOverlayDescriptor {
            id: FluxUtf8Slice::from_str(id),
            label: FluxUtf8Slice::from_str(label),
            hotkey: FluxUtf8Slice::from_str(hotkey),
            render_policy,
        };
        let status = register_overlay(registrar.registration_context, &descriptor);
        if status != FluxStatus::OK {
            return status;
        }
    }
    if let Some((id, version)) = spec.save_chunk {
        let Some(register_save_chunk) = registrar.register_save_chunk else {
            return FluxStatus::FAILED;
        };
        let descriptor = FluxSaveChunkDescriptor {
            id: FluxUtf8Slice::from_str(id),
            version,
        };
        let status = register_save_chunk(registrar.registration_context, &descriptor);
        if status != FluxStatus::OK {
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
        || host.api_version != ENGINE_PLUGIN_API_VERSION
    {
        return Err(FluxStatus::FAILED);
    }
    Ok((plugin, event, host))
}
