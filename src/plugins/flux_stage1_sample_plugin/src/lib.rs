use std::ffi::c_void;

const ENGINE_PLUGIN_API_VERSION: u32 = 4;

/// Borrowed UTF-8 string view shared across the stage-1 plugin ABI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxUtf8Slice {
    ptr: *const u8,
    len: usize,
}

impl FluxUtf8Slice {
    /// Builds one borrowed UTF-8 view for ABI callbacks.
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

/// Integer status code returned by every stage-1 ABI function.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct FluxStatus(i32);

impl FluxStatus {
    /// Successful ABI result.
    pub const OK: Self = Self(0);

    /// Invalid input pointer or malformed host payload.
    pub const INVALID_ARGUMENT: Self = Self(1);

    /// Generic plugin-side failure.
    pub const FAILED: Self = Self(2);
}

/// Host callback used by the plugin to report a readable error string.
pub type FluxWriteErrorFn =
    unsafe extern "C" fn(context: *mut c_void, message: FluxUtf8Slice) -> FluxStatus;

/// Host API payload passed into `flux_plugin_create`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxHostApi {
    pub struct_size: u32,
    pub api_version: u32,
    pub plugin_root: FluxUtf8Slice,
    pub config_root: FluxUtf8Slice,
    pub assets_root: FluxUtf8Slice,
    pub write_error: Option<FluxWriteErrorFn>,
    pub error_context: *mut c_void,
}

/// Future-proof registrar payload passed into `flux_plugin_register`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxRegistrar {
    pub struct_size: u32,
    pub api_version: u32,
    pub register_gas_substance:
        Option<unsafe extern "C" fn(*mut c_void, *const FluxGasSubstanceDescriptor) -> FluxStatus>,
    pub register_event_handler:
        Option<unsafe extern "C" fn(*mut c_void, *const FluxEventHandlerDescriptor) -> FluxStatus>,
    pub register_tool:
        Option<unsafe extern "C" fn(*mut c_void, *const FluxToolDescriptor) -> FluxStatus>,
    pub register_overlay:
        Option<unsafe extern "C" fn(*mut c_void, *const FluxOverlayDescriptor) -> FluxStatus>,
    pub register_save_chunk:
        Option<unsafe extern "C" fn(*mut c_void, *const FluxSaveChunkDescriptor) -> FluxStatus>,
    pub registration_context: *mut c_void,
    pub reserved2: *mut c_void,
    pub reserved3: *mut c_void,
}

/// C-compatible gas substance descriptor supported by the v2 registrar.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxGasSubstanceDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    alias: FluxUtf8Slice,
    molecular_mass: f32,
    color_r: f32,
    color_g: f32,
    color_b: f32,
}

/// C-compatible tool descriptor supported by the v4 registrar.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxToolDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    icon_path: FluxUtf8Slice,
    silhouette_path: FluxUtf8Slice,
}

/// C-compatible overlay descriptor supported by the v4 registrar.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxOverlayDescriptor {
    id: FluxUtf8Slice,
    label: FluxUtf8Slice,
    hotkey: FluxUtf8Slice,
    render_policy: u32,
}

/// C-compatible save chunk descriptor supported by the v4 registrar.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxSaveChunkDescriptor {
    id: FluxUtf8Slice,
    version: u32,
}

/// C-compatible event handler descriptor supported by the v4 registrar.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FluxEventHandlerDescriptor {
    event_kind: u32,
    handler_name: FluxUtf8Slice,
}

/// Opaque sample plugin handle stored between create/register/destroy.
#[repr(C)]
pub struct FluxPluginHandle {
    api_version: u32,
    saw_non_empty_plugin_root: bool,
}

/// Returns the plugin ABI version supported by this sample DLL.
#[no_mangle]
pub extern "C" fn flux_plugin_api_version() -> u32 {
    ENGINE_PLUGIN_API_VERSION
}

/// Creates one sample plugin instance after validating the host payload.
#[no_mangle]
pub unsafe extern "C" fn flux_plugin_create(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus {
    if host.is_null() || out_plugin.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let host = &*host;
    if host.struct_size != std::mem::size_of::<FluxHostApi>() as u32 {
        write_host_error(host, "host struct_size mismatch");
        return FluxStatus::FAILED;
    }
    if host.api_version != ENGINE_PLUGIN_API_VERSION {
        write_host_error(host, "host api_version mismatch");
        return FluxStatus::FAILED;
    }

    let handle = Box::new(FluxPluginHandle {
        api_version: host.api_version,
        saw_non_empty_plugin_root: !host.plugin_root.ptr.is_null() && host.plugin_root.len > 0,
    });
    *out_plugin = Box::into_raw(handle);
    FluxStatus::OK
}

/// Validates the registrar payload for the sample plugin.
#[no_mangle]
pub unsafe extern "C" fn flux_plugin_register(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus {
    if plugin.is_null() || registrar.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }

    let plugin = &mut *plugin;
    let registrar = &mut *registrar;
    if plugin.api_version != ENGINE_PLUGIN_API_VERSION {
        return FluxStatus::FAILED;
    }
    if registrar.struct_size != std::mem::size_of::<FluxRegistrar>() as u32 {
        return FluxStatus::FAILED;
    }
    if registrar.api_version != ENGINE_PLUGIN_API_VERSION {
        return FluxStatus::FAILED;
    }
    if !plugin.saw_non_empty_plugin_root {
        return FluxStatus::FAILED;
    }

    FluxStatus::OK
}

/// Destroys one sample plugin instance allocated by `flux_plugin_create`.
#[no_mangle]
pub unsafe extern "C" fn flux_plugin_destroy(plugin: *mut FluxPluginHandle) {
    if plugin.is_null() {
        return;
    }

    let _ = Box::from_raw(plugin);
}

fn write_host_error(host: &FluxHostApi, message: &str) {
    if let Some(callback) = host.write_error {
        let _ = unsafe { callback(host.error_context, FluxUtf8Slice::from_str(message)) };
    }
}
