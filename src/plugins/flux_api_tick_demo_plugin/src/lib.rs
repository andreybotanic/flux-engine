include!("../../flux_api_demo_common.rs");

const SPEC: DemoSpec = DemoSpec {
    event_kinds: &[5, 6],
    tool: None,
    overlay: None,
    save_chunk: None,
};

#[no_mangle]
pub extern "C" fn flux_plugin_api_version() -> u32 {
    ENGINE_PLUGIN_API_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn flux_plugin_create(
    host: *const FluxHostApi,
    out_plugin: *mut *mut FluxPluginHandle,
) -> FluxStatus {
    create(host, out_plugin)
}

#[no_mangle]
pub unsafe extern "C" fn flux_plugin_register(
    plugin: *mut FluxPluginHandle,
    registrar: *mut FluxRegistrar,
) -> FluxStatus {
    register(plugin, registrar, &SPEC)
}

#[no_mangle]
pub unsafe extern "C" fn flux_plugin_destroy(plugin: *mut FluxPluginHandle) {
    destroy(plugin);
}

#[no_mangle]
pub unsafe extern "C" fn flux_plugin_on_event(
    plugin: *mut FluxPluginHandle,
    event: *const FluxRuntimeEvent,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    if plugin.is_null() || event.is_null() || host.is_null() {
        return FluxStatus::INVALID_ARGUMENT;
    }
    let plugin = &mut *plugin;
    let event = &*event;
    let host = &mut *host;
    if event.api_version != ENGINE_PLUGIN_API_VERSION || host.api_version != ENGINE_PLUGIN_API_VERSION {
        return FluxStatus::FAILED;
    }
    if event.event_kind != 5 {
        return FluxStatus::OK;
    }
    plugin.counter = plugin.counter.wrapping_add(1);
    let Some(add_gas) = host.add_gas else {
        return FluxStatus::FAILED;
    };
    let x = 50;
    let y = 50;
    let mut added = 0u32;
    add_gas(
        host.context,
        x,
        y,
        FluxUtf8Slice::from_str("h2"),
        20,
        3.0,
        0.0,
        &mut added,
    )
}
