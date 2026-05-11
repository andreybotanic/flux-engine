use std::ffi::c_void;

use bevy::prelude::{UVec2, Vec2};

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

    /// Copies this ABI UTF-8 slice into an owned Rust string.
    ///
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

    /// Converts the raw ABI status into a Rust result.
    ///
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

/// Callback used by plugins to write a host-readable error message.
///
pub(crate) type FluxWriteErrorFn =
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
pub(crate) type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
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
pub(crate) type FluxRegisterToolFn =
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
pub(crate) type FluxRegisterOverlayFn = unsafe extern "C" fn(
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
pub(crate) type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

/// Callback used by plugins to set one world cell material by stable content id.
///
pub(crate) type FluxSetCellMaterialFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    material_id: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to add free gas with a cell velocity.
///
pub(crate) type FluxAddGasFn = unsafe extern "C" fn(
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
pub(crate) type FluxSubmitOverlayFrameFn = unsafe extern "C" fn(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to append one HUD block for the hovered cell.
///
pub(crate) type FluxSubmitHudBlockFn = unsafe extern "C" fn(
    context: *mut c_void,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to write a plugin-owned save chunk.
///
pub(crate) type FluxWriteSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to read a plugin-owned save chunk.
///
pub(crate) type FluxReadSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus;

/// Runtime-facing host API passed to one typed plugin event handler.
///
/// Plugins should treat this struct as an opaque capability object and call its
/// methods instead of reading the underlying callback table directly.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRuntimeHost {
    struct_size: u32,
    api_version: u32,
    context: *mut c_void,
    set_cell_material_fn: Option<FluxSetCellMaterialFn>,
    add_gas_fn: Option<FluxAddGasFn>,
    submit_overlay_frame_fn: Option<FluxSubmitOverlayFrameFn>,
    submit_hud_block_fn: Option<FluxSubmitHudBlockFn>,
    write_save_chunk_fn: Option<FluxWriteSaveChunkFn>,
    read_save_chunk_fn: Option<FluxReadSaveChunkFn>,
}

impl FluxRuntimeHost {
    /// Creates a runtime host callback table for a single event dispatch.
    ///
    pub(crate) fn new(context: *mut c_void) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            context,
            set_cell_material_fn: Some(crate::plugins::runtime_dll::set_cell_material_callback),
            add_gas_fn: Some(crate::plugins::runtime_dll::add_gas_callback),
            submit_overlay_frame_fn: Some(
                crate::plugins::runtime_dll::submit_overlay_frame_callback,
            ),
            submit_hud_block_fn: Some(crate::plugins::runtime_dll::submit_hud_block_callback),
            write_save_chunk_fn: Some(crate::plugins::runtime_dll::write_save_chunk_callback),
            read_save_chunk_fn: Some(crate::plugins::runtime_dll::read_save_chunk_callback),
        }
    }

    /// Returns `true` when this runtime host matches the expected ABI layout.
    ///
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<Self>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION_VALUE
    }

    /// Sets one editable world cell to the requested stable material id.
    ///
    pub fn set_cell_material(&mut self, cell: UVec2, material_id: &str) -> Result<(), FluxStatus> {
        let Some(callback) = self.set_cell_material_fn else {
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

    /// Adds free gas with the requested velocity and returns the amount the host accepted.
    ///
    pub fn add_gas(
        &mut self,
        cell: UVec2,
        substance: &str,
        amount: u32,
        velocity: Vec2,
    ) -> Result<u32, FluxStatus> {
        let Some(callback) = self.add_gas_fn else {
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

    /// Submits one complete RGBA8 overlay frame for the active plugin overlay.
    ///
    pub fn submit_overlay_frame(
        &mut self,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.submit_overlay_frame_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.context, width, height, rgba8.as_ptr(), rgba8.len()) }.into_result()
    }

    /// Appends one HUD block line under the plugin-specific runtime block title.
    ///
    pub fn submit_hud_block(&mut self, title: &str, line: &str) -> Result<(), FluxStatus> {
        let Some(callback) = self.submit_hud_block_fn else {
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

    /// Writes one plugin-owned save chunk into the host chunk store.
    ///
    pub fn write_save_chunk(
        &mut self,
        chunk_id: &str,
        version: u32,
        bytes: &[u8],
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.write_save_chunk_fn else {
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
    ///
    pub fn read_save_chunk(
        &mut self,
        chunk_id: &str,
    ) -> Result<Option<(u32, Vec<u8>)>, FluxStatus> {
        let Some(callback) = self.read_save_chunk_fn else {
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

/// Creation-time host API exposed to one plugin instance.
///
/// Plugins should use these methods for path discovery and startup error reporting.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxHostApi {
    struct_size: u32,
    api_version: u32,
    plugin_root: FluxUtf8Slice,
    config_root: FluxUtf8Slice,
    assets_root: FluxUtf8Slice,
    write_error_fn: Option<FluxWriteErrorFn>,
    error_context: *mut c_void,
}

impl FluxHostApi {
    /// Creates the stage-1 host API payload.
    ///
    pub(crate) fn new(
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
            write_error_fn: write_error,
            error_context,
        }
    }

    /// Returns `true` when this host payload matches the expected ABI layout.
    ///
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<Self>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION_VALUE
    }

    /// Returns the absolute plugin package root visible to the runtime plugin.
    ///
    pub fn plugin_root(&self) -> Result<String, FluxStatus> {
        self.plugin_root.try_to_string()
    }

    /// Returns the absolute plugin configuration directory visible to the runtime plugin.
    ///
    pub fn config_root(&self) -> Result<String, FluxStatus> {
        self.config_root.try_to_string()
    }

    /// Returns the absolute plugin asset directory visible to the runtime plugin.
    ///
    pub fn assets_root(&self) -> Result<String, FluxStatus> {
        self.assets_root.try_to_string()
    }

    /// Reports one human-readable startup error message back to the host.
    ///
    pub fn write_error(&self, message: &str) -> Result<(), FluxStatus> {
        let Some(callback) = self.write_error_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.error_context, FluxUtf8Slice::from_str(message)) }.into_result()
    }
}

/// Registration-time host API passed into `flux_plugin_register`.
///
/// Plugins should use these methods to declare event handlers and plugin-owned content.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRegistrar {
    struct_size: u32,
    api_version: u32,
    register_gas_substance_fn: Option<FluxRegisterGasSubstanceFn>,
    register_event_handler_fn: Option<FluxRegisterEventHandlerFn>,
    register_tool_fn: Option<FluxRegisterToolFn>,
    register_overlay_fn: Option<FluxRegisterOverlayFn>,
    register_save_chunk_fn: Option<FluxRegisterSaveChunkFn>,
    registration_context: *mut c_void,
    reserved2: *mut c_void,
    reserved3: *mut c_void,
}

impl FluxRegistrar {
    /// Creates the registrar payload used by `flux_plugin_register`.
    ///
    pub(crate) fn new(
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
            register_gas_substance_fn: register_gas_substance,
            register_event_handler_fn: register_event_handler,
            register_tool_fn: register_tool,
            register_overlay_fn: register_overlay,
            register_save_chunk_fn: register_save_chunk,
            registration_context,
            reserved2: std::ptr::null_mut(),
            reserved3: std::ptr::null_mut(),
        }
    }

    /// Returns `true` when this registrar payload matches the expected ABI layout.
    ///
    pub fn is_compatible(&self) -> bool {
        self.struct_size == std::mem::size_of::<Self>() as u32
            && self.api_version == ENGINE_PLUGIN_API_VERSION_VALUE
    }

    /// Registers one gas-capable substance descriptor for this plugin.
    ///
    pub fn register_gas_substance(
        &mut self,
        descriptor: &FluxGasSubstanceDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_gas_substance_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one named plugin event handler for the requested event kind.
    ///
    pub fn register_event_handler(
        &mut self,
        event_kind: FluxEventKind,
        handler_name: &str,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_event_handler_fn else {
            return Err(FluxStatus::FAILED);
        };
        let descriptor =
            FluxEventHandlerDescriptor::new(event_kind, FluxUtf8Slice::from_str(handler_name));
        unsafe { callback(self.registration_context, &descriptor) }.into_result()
    }

    /// Registers one tool descriptor owned by this plugin.
    ///
    pub fn register_tool(&mut self, descriptor: &FluxToolDescriptor) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_tool_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one overlay descriptor owned by this plugin.
    ///
    pub fn register_overlay(
        &mut self,
        descriptor: &FluxOverlayDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_overlay_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
    }

    /// Registers one save-chunk descriptor owned by this plugin.
    ///
    pub fn register_save_chunk(
        &mut self,
        descriptor: &FluxSaveChunkDescriptor,
    ) -> Result<(), FluxStatus> {
        let Some(callback) = self.register_save_chunk_fn else {
            return Err(FluxStatus::FAILED);
        };
        unsafe { callback(self.registration_context, descriptor) }.into_result()
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
