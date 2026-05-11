include!("../../flux_api_demo_common.rs");

const OVERLAY_WIDTH: u32 = 102;
const OVERLAY_HEIGHT: u32 = 102;

const EVENT_HANDLERS: &[DemoEventHandler] = &[DemoEventHandler {
    event_kind: FluxEventKind::RenderOverlay,
    handler_name: "onRenderOverlay",
}];

const SPEC: DemoSpec = DemoSpec {
    event_handlers: EVENT_HANDLERS,
    tool: None,
    overlay: Some((
        "flux.api_temperature_overlay.overlay.temperature",
        "Temperature",
        "",
        1,
    )),
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
pub unsafe extern "C" fn onRenderOverlay(
    plugin: *mut FluxPluginHandle,
    event: *const FluxRenderOverlayEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, _event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    plugin.counter = plugin.counter.wrapping_add(1);
    let mut frame = vec![0u8; (OVERLAY_WIDTH as usize) * (OVERLAY_HEIGHT as usize) * 4];
    for y in 0..OVERLAY_HEIGHT {
        for x in 0..OVERLAY_WIDTH {
            let wave = (((x as f32 * 0.12) + (plugin.counter as f32 * 0.03)).sin() + 1.0) * 0.5;
            let vertical = y as f32 / (OVERLAY_HEIGHT - 1) as f32;
            let heat = (wave * 0.55 + vertical * 0.45).clamp(0.0, 1.0);
            let cold = 1.0 - heat;
            let index = ((y * OVERLAY_WIDTH + x) as usize) * 4;
            frame[index] = (heat * 255.0).round().clamp(0.0, 255.0) as u8;
            frame[index + 1] = ((0.18 + cold * 0.25) * 255.0).round().clamp(0.0, 255.0) as u8;
            frame[index + 2] = (cold * 255.0).round().clamp(0.0, 255.0) as u8;
            frame[index + 3] = (0.48_f32 * 255.0).round() as u8;
        }
    }
    match host.submit_overlay_frame(OVERLAY_WIDTH, OVERLAY_HEIGHT, &frame) {
        Ok(_) => FluxStatus::OK,
        Err(status) => status,
    }
}
