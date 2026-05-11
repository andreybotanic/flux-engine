include!("../../flux_api_demo_common.rs");

const WORLD_WIDTH: u32 = 102;
const WORLD_HEIGHT: u32 = 102;

const SPEC: DemoSpec = DemoSpec {
    event_kinds: &[1, 2, 11, 19, 20],
    tool: None,
    overlay: None,
    save_chunk: Some(("flux.api_ui_save_demo.save.counter", 1)),
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
    match event.event_kind {
        1 => read_counter_chunk(plugin, host),
        2 => write_counter_chunk(plugin, host),
        11 if event.button == 1 && event.has_cell != 0 => {
            if let Some(index) = cell_index(event.cell_x, event.cell_y) {
                plugin.cell_counters[index] = plugin.cell_counters[index].saturating_add(1);
            }
            FluxStatus::OK
        }
        19 => submit_hud(plugin, event, host),
        20 => submit_hud(plugin, event, host),
        _ => FluxStatus::OK,
    }
}

unsafe fn write_counter_chunk(plugin: &FluxPluginHandle, host: &mut FluxRuntimeHost) -> FluxStatus {
    let Some(write_save_chunk) = host.write_save_chunk else {
        return FluxStatus::FAILED;
    };
    let mut payload = Vec::with_capacity(plugin.cell_counters.len() * 4);
    for counter in &plugin.cell_counters {
        payload.extend_from_slice(&counter.to_le_bytes());
    }
    write_save_chunk(
        host.context,
        FluxUtf8Slice::from_str("flux.api_ui_save_demo.save.counter"),
        1,
        payload.as_ptr(),
        payload.len(),
    )
}

unsafe fn read_counter_chunk(plugin: &mut FluxPluginHandle, host: &mut FluxRuntimeHost) -> FluxStatus {
    let Some(read_save_chunk) = host.read_save_chunk else {
        return FluxStatus::FAILED;
    };
    let mut version = 0u32;
    let mut len = 0usize;
    let mut payload = vec![0u8; plugin.cell_counters.len() * 4];
    let status = read_save_chunk(
        host.context,
        FluxUtf8Slice::from_str("flux.api_ui_save_demo.save.counter"),
        &mut version,
        payload.as_mut_ptr(),
        payload.len(),
        &mut len,
    );
    if status != FluxStatus::OK {
        return FluxStatus::OK;
    }
    if version == 1 && len == payload.len() {
        for (index, chunk) in payload.chunks_exact(4).enumerate() {
            plugin.cell_counters[index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
    }
    FluxStatus::OK
}

unsafe fn submit_hud(
    plugin: &FluxPluginHandle,
    event: &FluxRuntimeEvent,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let Some(submit_hud_block) = host.submit_hud_block else {
        return FluxStatus::FAILED;
    };
    let cell_counter = cell_index(event.cell_x, event.cell_y)
        .and_then(|index| plugin.cell_counters.get(index).copied())
        .unwrap_or(0);
    let line = format!(
        "cell left-clicks {}, cell ({}, {})",
        cell_counter, event.cell_x, event.cell_y
    );
    submit_hud_block(
        host.context,
        FluxUtf8Slice::from_str("API UI/Save Demo"),
        FluxUtf8Slice {
            ptr: line.as_ptr(),
            len: line.len(),
        },
    )
}

fn cell_index(x: u32, y: u32) -> Option<usize> {
    (x < WORLD_WIDTH && y < WORLD_HEIGHT).then_some((y * WORLD_WIDTH + x) as usize)
}
