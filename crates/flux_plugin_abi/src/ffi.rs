use std::ffi::c_void;

pub const ENGINE_PLUGIN_API_VERSION_VALUE: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl FluxUtf8Slice {
    pub fn from_str(value: &str) -> Self {
        if value.is_empty() {
            Self {
                ptr: std::ptr::null(),
                len: 0,
            }
        } else {
            Self {
                ptr: value.as_ptr(),
                len: value.len(),
            }
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxStatus(pub i32);

impl FluxStatus {
    pub const OK: Self = Self(0);
    pub const INVALID_ARGUMENT: Self = Self(1);
    pub const FAILED: Self = Self(2);
    pub const API_UNAVAILABLE: Self = Self(3);
    pub const UNSUPPORTED: Self = Self(4);

    pub fn code(self) -> i32 {
        self.0
    }

    pub fn is_ok(self) -> bool {
        self == Self::OK
    }

    pub fn into_result(self) -> Result<(), Self> {
        if self.is_ok() {
            Ok(())
        } else {
            Err(self)
        }
    }
}

pub type FluxWriteLogFn =
    unsafe extern "C" fn(context: *mut c_void, level: u32, message: FluxUtf8Slice) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxHostApi {
    pub struct_size: u32,
    pub api_version: u32,
    pub plugin_id: FluxUtf8Slice,
    pub engine_version: FluxUtf8Slice,
    pub plugin_root: FluxUtf8Slice,
    pub config_root: FluxUtf8Slice,
    pub assets_root: FluxUtf8Slice,
    pub write_log_fn: Option<FluxWriteLogFn>,
    pub log_context: *mut c_void,
}

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

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxEntityDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub icon_path: FluxUtf8Slice,
    pub silhouette_path: FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxToolDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub icon_path: FluxUtf8Slice,
    pub silhouette_path: FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxPanelDescriptor {
    pub id: FluxUtf8Slice,
    pub title: FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxOverlayDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub hotkey: FluxUtf8Slice,
    pub render_policy: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxOverlayMaterialDescriptor {
    pub id: FluxUtf8Slice,
    pub label: FluxUtf8Slice,
    pub shader_path: FluxUtf8Slice,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSaveChunkDescriptor {
    pub id: FluxUtf8Slice,
    pub version: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxSubscriptionDescriptor {
    pub event_kind: u32,
}

pub(crate) type FluxRegisterGasSubstanceFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxGasSubstanceDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterEntityFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxEntityDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterToolFn =
    unsafe extern "C" fn(context: *mut c_void, descriptor: *const FluxToolDescriptor) -> FluxStatus;

pub(crate) type FluxRegisterPanelFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxPanelDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterOverlayFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterOverlayMaterialFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxOverlayMaterialDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSaveChunkDescriptor,
) -> FluxStatus;

pub(crate) type FluxRegisterSubscriptionFn = unsafe extern "C" fn(
    context: *mut c_void,
    descriptor: *const FluxSubscriptionDescriptor,
) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub register_gas_substance_fn: Option<FluxRegisterGasSubstanceFn>,
    pub register_entity_fn: Option<FluxRegisterEntityFn>,
    pub register_tool_fn: Option<FluxRegisterToolFn>,
    pub register_panel_fn: Option<FluxRegisterPanelFn>,
    pub register_overlay_fn: Option<FluxRegisterOverlayFn>,
    pub register_save_chunk_fn: Option<FluxRegisterSaveChunkFn>,
    pub register_subscription_fn: Option<FluxRegisterSubscriptionFn>,
    pub registration_context: *mut c_void,
    pub register_overlay_material_fn: Option<FluxRegisterOverlayMaterialFn>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FluxTimeSnapshot {
    pub tick: u64,
    pub paused: u8,
    pub speed: u32,
    pub delta_seconds: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FluxCellBounds {
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FluxCellSnapshot {
    pub exists: u8,
    pub solid_entity_id: u32,
    pub structure_count: u32,
}

pub(crate) type FluxEntityPlaceFn = unsafe extern "C" fn(
    context: *mut c_void,
    kind_id: FluxUtf8Slice,
    origin_x: u32,
    origin_y: u32,
    rotation: u32,
    out_entity_id: *mut u32,
) -> FluxStatus;

pub(crate) type FluxEntityRemoveFn =
    unsafe extern "C" fn(context: *mut c_void, entity_id: u32) -> FluxStatus;

pub(crate) type FluxEntityRemoveAtFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    layer_id: FluxUtf8Slice,
    out_entity_id: *mut u32,
) -> FluxStatus;

pub(crate) type FluxEntitySetRotationFn =
    unsafe extern "C" fn(context: *mut c_void, entity_id: u32, rotation: u32) -> FluxStatus;

pub(crate) type FluxEntitySetEnabledFn =
    unsafe extern "C" fn(context: *mut c_void, entity_id: u32, enabled: u8) -> FluxStatus;

pub(crate) type FluxEntitySetLabelFn =
    unsafe extern "C" fn(context: *mut c_void, entity_id: u32, label: FluxUtf8Slice) -> FluxStatus;

pub(crate) type FluxGasAddFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    amount: u32,
    velocity_x: f32,
    velocity_y: f32,
    out_added: *mut u32,
) -> FluxStatus;

pub(crate) type FluxGasRemoveFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    amount: u32,
    out_removed: *mut u32,
) -> FluxStatus;

pub(crate) type FluxGasClearCellFn =
    unsafe extern "C" fn(context: *mut c_void, x: u32, y: u32) -> FluxStatus;

pub(crate) type FluxGasPressureAtFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    out_pressure: *mut f32,
) -> FluxStatus;

pub(crate) type FluxGasAmountAtFn = unsafe extern "C" fn(
    context: *mut c_void,
    x: u32,
    y: u32,
    substance: FluxUtf8Slice,
    out_amount: *mut u32,
) -> FluxStatus;

pub(crate) type FluxSubmitHudBlockFn = unsafe extern "C" fn(
    context: *mut c_void,
    block_id: FluxUtf8Slice,
    title: FluxUtf8Slice,
    line: FluxUtf8Slice,
    sort_order: i32,
) -> FluxStatus;

/// Reserved callback slot kept for binary compatibility with previously built plugins.
pub(crate) type FluxReservedOverlaySubmitSlotFn = unsafe extern "C" fn(
    context: *mut c_void,
    width: u32,
    height: u32,
    rgba8: *const u8,
    len: usize,
) -> FluxStatus;

pub(crate) type FluxSubmitOverlayGraphFn =
    unsafe extern "C" fn(context: *mut c_void, graph_json: FluxUtf8Slice) -> FluxStatus;

pub(crate) type FluxWriteSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    version: u32,
    bytes: *const u8,
    len: usize,
) -> FluxStatus;

pub(crate) type FluxReadSaveChunkFn = unsafe extern "C" fn(
    context: *mut c_void,
    chunk_id: FluxUtf8Slice,
    out_version: *mut u32,
    bytes: *mut u8,
    len: usize,
    out_len: *mut usize,
) -> FluxStatus;

pub(crate) type FluxDeleteSaveChunkFn =
    unsafe extern "C" fn(context: *mut c_void, chunk_id: FluxUtf8Slice) -> FluxStatus;

pub(crate) type FluxGetTimeSnapshotFn =
    unsafe extern "C" fn(context: *mut c_void, out_snapshot: *mut FluxTimeSnapshot) -> FluxStatus;

pub(crate) type FluxSetPausedFn =
    unsafe extern "C" fn(context: *mut c_void, paused: u8) -> FluxStatus;

pub(crate) type FluxTogglePauseFn =
    unsafe extern "C" fn(context: *mut c_void, out_paused: *mut u8) -> FluxStatus;

pub(crate) type FluxSetSpeedFn =
    unsafe extern "C" fn(context: *mut c_void, speed: u32) -> FluxStatus;

pub(crate) type FluxSetActiveToolFn =
    unsafe extern "C" fn(context: *mut c_void, tool_id: FluxUtf8Slice) -> FluxStatus;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRuntimeHost {
    pub struct_size: u32,
    pub api_version: u32,
    pub context: *mut c_void,
    pub entity_place_fn: Option<FluxEntityPlaceFn>,
    pub entity_remove_fn: Option<FluxEntityRemoveFn>,
    pub entity_remove_at_fn: Option<FluxEntityRemoveAtFn>,
    pub entity_set_rotation_fn: Option<FluxEntitySetRotationFn>,
    pub entity_set_enabled_fn: Option<FluxEntitySetEnabledFn>,
    pub entity_set_label_fn: Option<FluxEntitySetLabelFn>,
    pub gas_add_fn: Option<FluxGasAddFn>,
    pub gas_remove_fn: Option<FluxGasRemoveFn>,
    pub gas_clear_cell_fn: Option<FluxGasClearCellFn>,
    pub gas_pressure_at_fn: Option<FluxGasPressureAtFn>,
    pub gas_amount_at_fn: Option<FluxGasAmountAtFn>,
    pub submit_hud_block_fn: Option<FluxSubmitHudBlockFn>,
    pub reserved_overlay_submit_slot_fn: Option<FluxReservedOverlaySubmitSlotFn>,
    pub submit_overlay_graph_fn: Option<FluxSubmitOverlayGraphFn>,
    pub write_save_chunk_fn: Option<FluxWriteSaveChunkFn>,
    pub read_save_chunk_fn: Option<FluxReadSaveChunkFn>,
    pub delete_save_chunk_fn: Option<FluxDeleteSaveChunkFn>,
    pub get_time_snapshot_fn: Option<FluxGetTimeSnapshotFn>,
    pub set_paused_fn: Option<FluxSetPausedFn>,
    pub toggle_pause_fn: Option<FluxTogglePauseFn>,
    pub set_speed_fn: Option<FluxSetSpeedFn>,
    pub set_active_tool_fn: Option<FluxSetActiveToolFn>,
    pub write_log_fn: Option<FluxWriteLogFn>,
}

#[repr(C)]
#[derive(Debug)]
pub struct FluxPluginHandle {
    _private: [u8; 0],
}

pub type FluxPluginApiVersionFn = unsafe extern "C" fn() -> u32;
pub type FluxPluginCreateFn = unsafe extern "C" fn(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus;
pub type FluxPluginRegisterFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus;
pub type FluxPluginDispatchFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    event_kind: u32,
    payload: *const u8,
    payload_len: usize,
    host: *mut FluxRuntimeHost,
) -> FluxStatus;
pub type FluxPluginDestroyFn = unsafe extern "C" fn(plugin: *mut FluxPluginHandle);

pub const FLUX_PLUGIN_API_VERSION_EXPORT_NAME: &[u8] = b"flux_plugin_api_version\0";
pub const FLUX_PLUGIN_CREATE_EXPORT_NAME: &[u8] = b"flux_plugin_create\0";
pub const FLUX_PLUGIN_REGISTER_EXPORT_NAME: &[u8] = b"flux_plugin_register\0";
pub const FLUX_PLUGIN_DISPATCH_EXPORT_NAME: &[u8] = b"flux_plugin_dispatch\0";
pub const FLUX_PLUGIN_DESTROY_EXPORT_NAME: &[u8] = b"flux_plugin_destroy\0";
