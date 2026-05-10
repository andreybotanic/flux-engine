use std::ffi::c_void;

use crate::plugins::id::ENGINE_PLUGIN_API_VERSION_VALUE;

/// Opaque UTF-8 string view used by the stable plugin ABI.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxUtf8Slice {
    pub ptr: *const u8,
    pub len: usize,
}

impl FluxUtf8Slice {
    /// Creates a borrowed UTF-8 slice for ABI calls.
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
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FluxStatus(pub i32);

impl FluxStatus {
    pub const OK: Self = Self(0);
    pub const INVALID_ARGUMENT: Self = Self(1);
    pub const FAILED: Self = Self(2);

    /// Returns the raw numeric status code.
    pub fn code(self) -> i32 {
        self.0
    }

    /// Returns `true` when the ABI call succeeded.
    pub fn is_ok(self) -> bool {
        self == Self::OK
    }
}

/// Callback used by plugins to write a host-readable error message.
pub type FluxWriteErrorFn =
    unsafe extern "C" fn(context: *mut c_void, message: FluxUtf8Slice) -> FluxStatus;

/// Host callbacks and runtime paths exposed to one plugin instance.
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
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub reserved0: *mut c_void,
    pub reserved1: *mut c_void,
    pub reserved2: *mut c_void,
}

impl FluxRegistrar {
    /// Creates the stage-1 registrar placeholder.
    pub fn new() -> Self {
        Self {
            struct_size: std::mem::size_of::<Self>() as u32,
            api_version: ENGINE_PLUGIN_API_VERSION_VALUE,
            reserved0: std::ptr::null_mut(),
            reserved1: std::ptr::null_mut(),
            reserved2: std::ptr::null_mut(),
        }
    }
}

/// Opaque plugin-owned handle shared back to the host across ABI calls.
#[repr(C)]
#[derive(Debug)]
pub struct FluxPluginHandle {
    _private: [u8; 0],
}

/// Function pointer type for `flux_plugin_api_version`.
pub type FluxPluginApiVersionFn = unsafe extern "C" fn() -> u32;

/// Function pointer type for `flux_plugin_create`.
pub type FluxPluginCreateFn = unsafe extern "C" fn(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_register`.
pub type FluxPluginRegisterFn = unsafe extern "C" fn(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus;

/// Function pointer type for `flux_plugin_destroy`.
pub type FluxPluginDestroyFn = unsafe extern "C" fn(plugin: *mut FluxPluginHandle);

/// Null-terminated export name for `flux_plugin_api_version`.
pub const FLUX_PLUGIN_API_VERSION_EXPORT_NAME: &[u8] = b"flux_plugin_api_version\0";

/// Null-terminated export name for `flux_plugin_create`.
pub const FLUX_PLUGIN_CREATE_EXPORT_NAME: &[u8] = b"flux_plugin_create\0";

/// Null-terminated export name for `flux_plugin_register`.
pub const FLUX_PLUGIN_REGISTER_EXPORT_NAME: &[u8] = b"flux_plugin_register\0";

/// Null-terminated export name for `flux_plugin_destroy`.
pub const FLUX_PLUGIN_DESTROY_EXPORT_NAME: &[u8] = b"flux_plugin_destroy\0";
