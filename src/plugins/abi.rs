use std::ffi::c_void;

use crate::plugins::id::ENGINE_PLUGIN_API_VERSION_VALUE;

#[path = "abi_events.rs"]
mod events;

pub use self::events::*;

/// Opaque UTF-8 string view used by the stable plugin ABI.
///
/// # Fields
/// - `ptr`: Pointer to the first UTF-8 byte, or null when the slice is empty.
/// - `len`: Number of bytes available through `ptr`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl FluxUtf8Slice {
    /// Creates a borrowed UTF-8 slice for ABI calls.
    ///
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

/// Status code returned by plugin ABI functions.
///
/// # Fields
/// - `0`: Raw integer status code returned across the stable ABI boundary.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxStatus(pub i32);

impl FluxStatus {
    pub const OK: Self = Self(0);
    pub const INVALID_ARGUMENT: Self = Self(1);
    pub const FAILED: Self = Self(2);

    /// Returns the raw numeric status code.
    ///
    pub fn code(self) -> i32 {
        self.0
    }

    /// Returns `true` when the ABI call succeeded.
    ///
    pub fn is_ok(self) -> bool {
        self == Self::OK
    }
}

/// Callback used by plugins to write a host-readable error message.
///
pub type FluxWriteErrorFn =
    unsafe extern "C" fn(context: *mut c_void, message: FluxUtf8Slice) -> FluxStatus;

/// C-compatible gas substance descriptor emitted by content plugins.
///
/// # Fields
/// - `id`: Stable substance id passed to the host for registration.
/// - `label`: Human-readable substance label shown in UI and debug output.
/// - `alias`: Legacy short alias accepted by runtime gas APIs.
/// - `molecular_mass`: Relative molecular mass used by gas simulation ordering.
/// - `color_r`: Red channel of the normalized gas display color.
/// - `color_g`: Green channel of the normalized gas display color.
/// - `color_b`: Blue channel of the normalized gas display color.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxGasSubstanceDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub alias: FluxUtf8Slice,
    pub molecular_mass: f32,
    pub color_r: f32,
    pub color_g: f32,
    pub color_b: f32,
}

/// Callback used by plugins to register one gas-capable substance.
///
pub type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus;

/// C-compatible tool descriptor emitted by plugins.
///
/// # Fields
/// - `id`: Stable content id of the tool being registered.
/// - `label`: Human-readable tool label shown in the UI.
/// - `icon_path`: Relative asset path to the main tool icon.
/// - `silhouette_path`: Relative asset path to the optional silhouette icon.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxToolDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub icon_path: FluxUtf8Slice,
    pub silhouette_path: FluxUtf8Slice,
}

/// Callback used by plugins to register one tool descriptor.
///
pub type FluxRegisterToolFn =
    unsafe extern "C" fn(context: *mut c_void, descriptor: *const FluxToolDescriptor) -> FluxStatus;

/// C-compatible overlay descriptor emitted by plugins.
///
/// # Fields
/// - `id`: Stable content id of the overlay being registered.
/// - `label`: Human-readable overlay label shown in selectors and menus.
/// - `hotkey`: Optional hotkey string requested for the overlay.
/// - `render_policy`: Raw ABI tag describing whether the overlay is core- or plugin-rendered.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxOverlayDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub hotkey: FluxUtf8Slice,
    pub render_policy: u32,
}

/// Callback used by plugins to register one overlay descriptor.
///
pub type FluxRegisterOverlayFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus;

/// C-compatible save chunk descriptor emitted by plugins.
///
/// # Fields
/// - `id`: Stable content id of the save chunk schema.
/// - `version`: Schema version written into save data for this chunk.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSaveChunkDescriptor {
    pub id: FluxUtf8Slice,
    pub version: u32,
}

/// Callback used by plugins to register one save chunk descriptor.
///
pub type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

/// Callback used by plugins to set one world cell material by stable content id.
///
pub type FluxSetCellMaterialFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    material_id: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to add free gas with a cell velocity.
///
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

/// Callback used by plugins to submit one complete RGBA8 overlay frame.
///
pub type FluxSubmitOverlayFrameFn = unsafe extern "C" fn(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to append one HUD block for the hovered cell.
///
pub type FluxSubmitHudBlockFn = unsafe extern "C" fn(
    context: *mut c_void,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to write a plugin-owned save chunk.
///
pub type FluxWriteSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to read a plugin-owned save chunk.
///
pub type FluxReadSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus;

/// Runtime host callback table passed to one typed plugin event handler.
///
/// # Fields
/// - `struct_size`: Size of this struct used for ABI validation.
/// - `api_version`: ABI version expected by both host and plugin.
/// - `context`: Opaque host-owned context passed back into callback functions.
/// - `set_cell_material`: Optional callback for changing a world cell material.
/// - `add_gas`: Optional callback for adding free gas with velocity.
/// - `submit_overlay_frame`: Optional callback for sending an RGBA8 overlay frame to the host.
/// - `submit_hud_block`: Optional callback for appending one HUD block line.
/// - `write_save_chunk`: Optional callback for writing a plugin-owned save chunk.
/// - `read_save_chunk`: Optional callback for reading a plugin-owned save chunk.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    /// Creates a runtime host callback table for a single event dispatch.
    ///
    pub fn new(context: *mut c_void) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            context,
            set_cell_material: Some(crate::plugins::runtime_dll::set_cell_material_callback),
            add_gas: Some(crate::plugins::runtime_dll::add_gas_callback),
            submit_overlay_frame: Some(crate::plugins::runtime_dll::submit_overlay_frame_callback),
            submit_hud_block: Some(crate::plugins::runtime_dll::submit_hud_block_callback),
            write_save_chunk: Some(crate::plugins::runtime_dll::write_save_chunk_callback),
            read_save_chunk: Some(crate::plugins::runtime_dll::read_save_chunk_callback),
        }
    }
}

/// Host callbacks and runtime paths exposed to one plugin instance.
///
/// # Fields
/// - `struct_size`: Size of this struct used for ABI validation.
/// - `api_version`: ABI version expected by both host and plugin.
/// - `plugin_root`: Absolute plugin package root visible to the runtime plugin.
/// - `config_root`: Absolute plugin configuration directory visible to the runtime plugin.
/// - `assets_root`: Absolute plugin asset directory visible to the runtime plugin.
/// - `write_error`: Optional callback for reporting host-readable error messages.
/// - `error_context`: Opaque host-owned context passed back into `write_error`.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    /// Creates the stage-1 host API payload.
    ///
    pub fn new(
        plugin_root: FluxUtf8Slice,
        config_root: FluxUtf8Slice,
        assets_root: FluxUtf8Slice,
        write_error: Option<FluxWriteErrorFn>,
        error_context: *mut c_void,
    ) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            plugin_root,
            config_root,
            assets_root,
            write_error,
            error_context,
        }
    }
}

/// Future-proof registrar payload passed into `flux_plugin_register`.
///
/// # Fields
/// - `struct_size`: Size of this struct used for ABI validation.
/// - `api_version`: ABI version expected by both host and plugin.
/// - `register_gas_substance`: Optional callback for registering gas-capable substances.
/// - `register_event_handler`: Optional callback for registering named event handlers.
/// - `register_tool`: Optional callback for registering tool descriptors.
/// - `register_overlay`: Optional callback for registering overlay descriptors.
/// - `register_save_chunk`: Optional callback for registering save chunk descriptors.
/// - `registration_context`: Opaque host-owned context passed back into registrar callbacks.
/// - `reserved2`: Reserved pointer slot kept for forward-compatible ABI expansion.
/// - `reserved3`: Reserved pointer slot kept for forward-compatible ABI expansion.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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
    /// Creates the registrar payload used by `flux_plugin_register`.
    ///
    pub fn new(
        register_gas_substance: Option<FluxRegisterGasSubstanceFn>,
        register_event_handler: Option<FluxRegisterEventHandlerFn>,
        register_tool: Option<FluxRegisterToolFn>,
        register_overlay: Option<FluxRegisterOverlayFn>,
        register_save_chunk: Option<FluxRegisterSaveChunkFn>,
        registration_context: *mut c_void,
    ) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            register_gas_substance,
            register_event_handler,
            register_tool,
            register_overlay,
            register_save_chunk,
            registration_context,
            reserved2: std::ptr::null_mut(),
            reserved3: std::ptr::null_mut(),
        }
    }
}

/// Opaque plugin-owned handle shared back to the host across ABI calls.
///
/// # Fields
/// - `_private`: Zero-sized private marker that prevents plugins from depending on the handle layout.
#[repr(C)]
#[derive(Debug)]
pub struct FluxPluginHandle {
    _private: [u8; 0],
}

/// Function pointer type for `flux_plugin_api_version`.
///
pub type FluxPluginApiVersionFn = unsafe extern "C" fn() -> u32;

/// Function pointer type for `flux_plugin_create`.
///
pub type FluxPluginCreateFn = unsafe extern "C" fn(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_register`.
///
pub type FluxPluginRegisterFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_destroy`.
///
pub type FluxPluginDestroyFn = unsafe extern "C" fn(plugin: *mut FluxPluginHandle);

/// Null-terminated export name for `flux_plugin_api_version`.
///
pub const FLUX_PLUGIN_API_VERSION_EXPORT_NAME: &[u8] = b"flux_plugin_api_version\0";

/// Null-terminated export name for `flux_plugin_create`.
///
pub const FLUX_PLUGIN_CREATE_EXPORT_NAME: &[u8] = b"flux_plugin_create\0";

/// Null-terminated export name for `flux_plugin_register`.
///
pub const FLUX_PLUGIN_REGISTER_EXPORT_NAME: &[u8] = b"flux_plugin_register\0";

/// Null-terminated export name for `flux_plugin_destroy`.
///
pub const FLUX_PLUGIN_DESTROY_EXPORT_NAME: &[u8] = b"flux_plugin_destroy\0";
