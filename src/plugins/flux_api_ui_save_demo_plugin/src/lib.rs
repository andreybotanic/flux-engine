include!("../../flux_api_demo_common.rs");

const WORLD_WIDTH: u32 = 102;
const WORLD_HEIGHT: u32 = 102;

const EVENT_HANDLERS: &[DemoEventHandler] = &[
    DemoEventHandler {
        event_kind: FluxEventKind::WorldLoaded,
        handler_name: "onWorldLoaded",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::WorldBeforeSave,
        handler_name: "onWorldBeforeSave",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::MouseDownCell,
        handler_name: "onMouseDownCell",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::BuildHudForCell,
        handler_name: "onBuildHudForCell",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::BuildPanel,
        handler_name: "onBuildPanel",
    },
];

const SPEC: DemoSpec = DemoSpec {
    event_handlers: EVENT_HANDLERS,
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
pub unsafe extern "C" fn onWorldLoaded(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, _event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    read_counter_chunk(plugin, host)
}

#[no_mangle]
pub unsafe extern "C" fn onWorldBeforeSave(
    plugin: *mut FluxPluginHandle,
    event: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, _event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    write_counter_chunk(plugin, host)
}

#[no_mangle]
pub unsafe extern "C" fn onMouseDownCell(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, _host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    if event.button == 1 && event.has_cell != 0 {
        if let Some(index) = cell_index(event.cell_x, event.cell_y) {
            plugin.cell_counters[index] = plugin.cell_counters[index].saturating_add(1);
        }
    }
    FluxStatus::OK
}

#[no_mangle]
pub unsafe extern "C" fn onBuildHudForCell(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildHudForCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    submit_cell_hud(plugin, event.cell_x, event.cell_y, host)
}

#[no_mangle]
pub unsafe extern "C" fn onBuildPanel(
    plugin: *mut FluxPluginHandle,
    event: *const FluxBuildPanelEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    submit_panel_summary(plugin, event.panel_id, host)
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

unsafe fn read_counter_chunk(
    plugin: &mut FluxPluginHandle,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
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
            plugin.cell_counters[index] =
                u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
    }
    FluxStatus::OK
}

unsafe fn submit_cell_hud(
    plugin: &FluxPluginHandle,
    cell_x: u32,
    cell_y: u32,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let Some(submit_hud_block) = host.submit_hud_block else {
        return FluxStatus::FAILED;
    };
    let cell_counter = cell_index(cell_x, cell_y)
        .and_then(|index| plugin.cell_counters.get(index).copied())
        .unwrap_or(0);
    let line = format!(
        "cell left-clicks {}, cell ({}, {})",
        cell_counter, cell_x, cell_y
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

unsafe fn submit_panel_summary(
    plugin: &FluxPluginHandle,
    panel_id: FluxUtf8Slice,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let Some(submit_hud_block) = host.submit_hud_block else {
        return FluxStatus::FAILED;
    };
    let total_clicks: u32 = plugin.cell_counters.iter().copied().sum();
    let panel_name = utf8_slice_or_empty(panel_id);
    let line = format!("panel {}, total left-clicks {}", panel_name, total_clicks);
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

fn utf8_slice_or_empty(value: FluxUtf8Slice) -> String {
    if value.len == 0 || value.ptr.is_null() {
        return String::new();
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr, value.len) };
    std::str::from_utf8(bytes)
        .map(str::to_string)
        .unwrap_or_default()
}
