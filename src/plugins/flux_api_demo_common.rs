use std::ffi::c_void;

/// ABI version used by the v3 demo plugins.
pub const ENGINE_PLUGIN_API_VERSION: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy)]
/// Borrowed UTF-8 string slice passed across the plugin ABI.
pub struct FluxUtf8Slice {
    ptr: *const u8,
    len: usize,
}

impl FluxUtf8Slice {
    /// Builds an ABI slice from a static Rust string.
    pub fn from_str(value: &'static str) -> Self {
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
/// Gas substance descriptor accepted by the v3 registrar.
pub struct FluxGasSubstanceDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    alias: FluxUtf8Slice,
    molecular_mass: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
}

/// Registrar callback for plugin-owned gas substance declarations.
pub type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Tool descriptor accepted by the v3 registrar.
pub struct FluxToolDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    icon_path: FluxUtf8Slice,
    silhouette_path: FluxUtf8Slice,
}

/// Registrar callback for plugin-owned tool declarations.
pub type FluxRegisterToolFn =
    unsafe extern "C" fn(context: *mut c_void, descriptor: *const FluxToolDescriptor) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Overlay descriptor accepted by the v3 registrar.
pub struct FluxOverlayDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    hotkey: FluxUtf8Slice,
    render_policy: u32,
}

/// Registrar callback for plugin-owned overlay declarations.
pub type FluxRegisterOverlayFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// Save chunk descriptor accepted by the v3 registrar.
pub struct FluxSaveChunkDescriptor {
    id: FluxUtf8Slice,
    version: u32,
}

/// Registrar callback for plugin-owned save chunk declarations.
pub type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy)]
/// ABI registration table passed by the host into `flux_plugin_register`.
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub register_gas_substance: Option<FluxRegisterGasSubstanceFn>,
    pub register_event_subscription: Option<unsafe extern "C" fn(*mut c_void, u32) -> FluxStatus>,
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
    api_version: u32,
    dragging: bool,
    last_x: u32,
    last_y: u32,
    counter: u32,
    cell_counters: Vec<u32>,
}

#[repr(C)]
#[derive(Clone, Copy)]
/// Runtime event payload passed by the host to demo plugins.
pub struct FluxRuntimeEvent {
    pub struct_size: u32,
    pub api_version: u32,
    pub event_kind: u32,
    pub has_cell: u8,
    pub cell_x: u32,
    pub cell_y: u32,
    pub button: u32,
    pub world_x: f32,
    pub world_y: f32,
    pub screen_x: f32,
    pub screen_y: f32,
    pub modifiers: u32,
    pub active_tool_id: FluxUtf8Slice,
    pub overlay_id: FluxUtf8Slice,
    pub key: FluxUtf8Slice,
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

/// Declarative registration data used by each small API demo plugin.
pub struct DemoSpec {
    pub event_kinds: &'static [u32],
    pub tool: Option<(&'static str, &'static str)>,
    pub overlay: Option<(&'static str, &'static str, &'static str, u32)>,
    pub save_chunk: Option<(&'static str, u32)>,
}

/// Creates a demo plugin handle after validating the v3 host API table.
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

/// Registers the descriptors and event subscriptions described by `spec`.
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
    for event_kind in spec.event_kinds {
        let Some(register_event) = registrar.register_event_subscription else {
            return FluxStatus::FAILED;
        };
        let status = register_event(registrar.registration_context, *event_kind);
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
