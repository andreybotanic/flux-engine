use std::ffi::c_void;

use crate::plugins::id::ENGINE_PLUGIN_API_VERSION_VALUE;

/// Opaque UTF-8 string view used by the stable plugin ABI.
///
/// # Fields
/// Public fields of `FluxUtf8Slice` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl FluxUtf8Slice {
    /// Creates a borrowed UTF-8 slice for ABI calls.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `from_str` from plugin-facing code when this operation is available in context.
    /// ```
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
/// Public fields of `FluxStatus` are part of the generated SDK reference.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxStatus(pub i32);

impl FluxStatus {
    pub const OK: Self = Self(0);
    pub const INVALID_ARGUMENT: Self = Self(1);
    pub const FAILED: Self = Self(2);

    /// Returns the raw numeric status code.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `code` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn code(self) -> i32 {
        self.0
    }

    /// Returns `true` when the ABI call succeeded.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `is_ok` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn is_ok(self) -> bool {
        self == Self::OK
    }
}

/// Callback used by plugins to write a host-readable error message.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxWriteErrorFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxWriteErrorFn =
    unsafe extern "C" fn(context: *mut c_void, message: FluxUtf8Slice) -> FluxStatus;

/// C-compatible gas substance descriptor emitted by content plugins.
///
/// # Fields
/// Public fields of `FluxGasSubstanceDescriptor` are part of the generated SDK reference.
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
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterGasSubstanceFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus;

/// Callback used by plugins to subscribe to one runtime event kind.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterEventSubscriptionFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterEventSubscriptionFn =
    unsafe extern "C" fn(context: *mut c_void, event_kind: u32) -> FluxStatus;

/// C-compatible tool descriptor emitted by plugins.
///
/// # Fields
/// Public fields of `FluxToolDescriptor` are part of the generated SDK reference.
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
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterToolFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterToolFn =
    unsafe extern "C" fn(context: *mut c_void, descriptor: *const FluxToolDescriptor) -> FluxStatus;

/// C-compatible overlay descriptor emitted by plugins.
///
/// # Fields
/// Public fields of `FluxOverlayDescriptor` are part of the generated SDK reference.
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
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterOverlayFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterOverlayFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus;

/// C-compatible save chunk descriptor emitted by plugins.
///
/// # Fields
/// Public fields of `FluxSaveChunkDescriptor` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSaveChunkDescriptor {
    pub id: FluxUtf8Slice,
    pub version: u32,
}

/// Callback used by plugins to register one save chunk descriptor.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxRegisterSaveChunkFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

/// Runtime event payload passed to a live DLL plugin.
///
/// # Fields
/// Public fields of `FluxRuntimeEvent` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
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

impl FluxRuntimeEvent {
    /// Creates an empty runtime event payload for one ABI event kind.
    ///
    /// # SDK Example
    /// ```rust
    /// // Call `new` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn new(event_kind: u32) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            event_kind,
            has_cell: 0,
            cell_x: 0,
            cell_y: 0,
            button: 0,
            world_x: 0.0,
            world_y: 0.0,
            screen_x: 0.0,
            screen_y: 0.0,
            modifiers: 0,
            active_tool_id: FluxUtf8Slice::from_str(""),
            overlay_id: FluxUtf8Slice::from_str(""),
            key: FluxUtf8Slice::from_str(""),
        }
    }
}

/// Callback used by plugins to set one world cell material by stable content id.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxSetCellMaterialFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxSetCellMaterialFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    material_id: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to add free gas with a cell velocity.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxAddGasFn` ABI signature supplied by FluxEngine.
/// ```
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
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxSubmitOverlayFrameFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxSubmitOverlayFrameFn = unsafe extern "C" fn(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to append one HUD block for the hovered cell.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxSubmitHudBlockFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxSubmitHudBlockFn = unsafe extern "C" fn(
    context: *mut c_void,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
) -> FluxStatus;

/// Callback used by plugins to write a plugin-owned save chunk.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxWriteSaveChunkFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxWriteSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus;

/// Callback used by plugins to read a plugin-owned save chunk.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxReadSaveChunkFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxReadSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus;

/// Runtime host callback table passed to `flux_plugin_on_event`.
///
/// # Fields
/// Public fields of `FluxRuntimeHost` are part of the generated SDK reference.
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
    /// # SDK Example
    /// ```rust
    /// // Call `new` from plugin-facing code when this operation is available in context.
    /// ```
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
/// Public fields of `FluxHostApi` are part of the generated SDK reference.
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
    /// # SDK Example
    /// ```rust
    /// // Call `new` from plugin-facing code when this operation is available in context.
    /// ```
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
/// Public fields of `FluxRegistrar` are part of the generated SDK reference.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub register_gas_substance: Option<FluxRegisterGasSubstanceFn>,
    pub register_event_subscription: Option<FluxRegisterEventSubscriptionFn>,
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
    /// # SDK Example
    /// ```rust
    /// // Call `new` from plugin-facing code when this operation is available in context.
    /// ```
    pub fn new(
        register_gas_substance: Option<FluxRegisterGasSubstanceFn>,
        register_event_subscription: Option<FluxRegisterEventSubscriptionFn>,
        register_tool: Option<FluxRegisterToolFn>,
        register_overlay: Option<FluxRegisterOverlayFn>,
        register_save_chunk: Option<FluxRegisterSaveChunkFn>,
        registration_context: *mut c_void,
    ) -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            register_gas_substance,
            register_event_subscription,
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
/// Public fields of `FluxPluginHandle` are part of the generated SDK reference.
#[repr(C)]
#[derive(Debug)]
pub struct FluxPluginHandle {
    _private: [u8; 0],
}

/// Function pointer type for `flux_plugin_api_version`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxPluginApiVersionFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxPluginApiVersionFn = unsafe extern "C" fn() -> u32;

/// Function pointer type for `flux_plugin_create`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxPluginCreateFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxPluginCreateFn = unsafe extern "C" fn(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_register`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxPluginRegisterFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxPluginRegisterFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_destroy`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxPluginDestroyFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxPluginDestroyFn = unsafe extern "C" fn(plugin: *mut FluxPluginHandle);

/// Function pointer type for `flux_plugin_on_event`.
///
/// # SDK Example
/// ```rust
/// // Store or call the callback through the `FluxPluginOnEventFn` ABI signature supplied by FluxEngine.
/// ```
pub type FluxPluginOnEventFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event: *const FluxRuntimeEvent,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;

/// Null-terminated export name for `flux_plugin_api_version`.
///
/// # SDK Example
/// ```rust
/// // Use `FLUX_PLUGIN_API_VERSION_EXPORT_NAME` when validating the Plugin SDK ABI contract.
/// ```
pub const FLUX_PLUGIN_API_VERSION_EXPORT_NAME: &[u8] = b"flux_plugin_api_version\0";

/// Null-terminated export name for `flux_plugin_create`.
///
/// # SDK Example
/// ```rust
/// // Use `FLUX_PLUGIN_CREATE_EXPORT_NAME` when validating the Plugin SDK ABI contract.
/// ```
pub const FLUX_PLUGIN_CREATE_EXPORT_NAME: &[u8] = b"flux_plugin_create\0";

/// Null-terminated export name for `flux_plugin_register`.
///
/// # SDK Example
/// ```rust
/// // Use `FLUX_PLUGIN_REGISTER_EXPORT_NAME` when validating the Plugin SDK ABI contract.
/// ```
pub const FLUX_PLUGIN_REGISTER_EXPORT_NAME: &[u8] = b"flux_plugin_register\0";

/// Null-terminated export name for `flux_plugin_destroy`.
///
/// # SDK Example
/// ```rust
/// // Use `FLUX_PLUGIN_DESTROY_EXPORT_NAME` when validating the Plugin SDK ABI contract.
/// ```
pub const FLUX_PLUGIN_DESTROY_EXPORT_NAME: &[u8] = b"flux_plugin_destroy\0";

/// Null-terminated export name for `flux_plugin_on_event`.
///
/// # SDK Example
/// ```rust
/// // Use `FLUX_PLUGIN_ON_EVENT_EXPORT_NAME` when validating the Plugin SDK ABI contract.
/// ```
pub const FLUX_PLUGIN_ON_EVENT_EXPORT_NAME: &[u8] = b"flux_plugin_on_event\0";
