include!("../../flux_api_demo_common.rs");

const EVENT_HANDLERS: &[DemoEventHandler] = &[DemoEventHandler {
    event_kind: FluxEventKind::SimulationPreCellGasStep,
    handler_name: "onSimulationPreCellGasStep",
}];

const SPEC: DemoSpec = DemoSpec {
    event_handlers: EVENT_HANDLERS,
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
pub unsafe extern "C" fn onSimulationPreCellGasStep(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, _event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    plugin.counter = plugin.counter.wrapping_add(1);
    let cell = UVec2::new(50, 50);
    let velocity = Vec2::new(3.0, 0.0);
    match host.add_gas(cell, "h2", 20, velocity) {
        Ok(_) => FluxStatus::OK,
        Err(status) => status,
    }
}
