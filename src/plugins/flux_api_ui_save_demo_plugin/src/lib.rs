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
        let cell = UVec2::new(event.cell_x, event.cell_y);
        if let Some(index) = cell_index(cell) {
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
    submit_cell_hud(plugin, UVec2::new(event.cell_x, event.cell_y), host)
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
    let mut payload = Vec::with_capacity(plugin.cell_counters.len() * 4);
    for counter in &plugin.cell_counters {
        payload.extend_from_slice(&counter.to_le_bytes());
    }
    match host.write_save_chunk("flux.api_ui_save_demo.save.counter", 1, &payload) {
        Ok(_) => FluxStatus::OK,
        Err(status) => status,
    }
}

unsafe fn read_counter_chunk(
    plugin: &mut FluxPluginHandle,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let Some((version, payload)) = (match host.read_save_chunk("flux.api_ui_save_demo.save.counter")
    {
        Ok(value) => value,
        Err(status) => return status,
    }) else {
        return FluxStatus::OK;
    };
    if version == 1 && payload.len() == plugin.cell_counters.len() * 4 {
        for (index, chunk) in payload.chunks_exact(4).enumerate() {
            plugin.cell_counters[index] =
                u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
    }
    FluxStatus::OK
}

unsafe fn submit_cell_hud(
    plugin: &FluxPluginHandle,
    cell: UVec2,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let cell_counter = cell_index(cell)
        .and_then(|index| plugin.cell_counters.get(index).copied())
        .unwrap_or(0);
    let line = format!(
        "cell left-clicks {}, cell ({}, {})",
        cell_counter, cell.x, cell.y
    );
    match host.submit_hud_block("API UI/Save Demo", &line) {
        Ok(_) => FluxStatus::OK,
        Err(status) => status,
    }
}

unsafe fn submit_panel_summary(
    plugin: &FluxPluginHandle,
    panel_id: FluxUtf8Slice,
    host: &mut FluxRuntimeHost,
) -> FluxStatus {
    let total_clicks: u32 = plugin.cell_counters.iter().copied().sum();
    let panel_name = utf8_slice_or_empty(panel_id);
    let line = format!("panel {}, total left-clicks {}", panel_name, total_clicks);
    match host.submit_hud_block("API UI/Save Demo", &line) {
        Ok(_) => FluxStatus::OK,
        Err(status) => status,
    }
}

fn cell_index(cell: UVec2) -> Option<usize> {
    (cell.x < WORLD_WIDTH && cell.y < WORLD_HEIGHT)
        .then_some((cell.y * WORLD_WIDTH + cell.x) as usize)
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
