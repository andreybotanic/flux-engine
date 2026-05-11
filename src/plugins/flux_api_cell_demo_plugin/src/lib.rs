include!("../../flux_api_demo_common.rs");

const PAINT_BUTTON: u32 = 2;

const EVENT_HANDLERS: &[DemoEventHandler] = &[
    DemoEventHandler {
        event_kind: FluxEventKind::MouseDownCell,
        handler_name: "onMouseDownCell",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::MouseMoveCell,
        handler_name: "onMouseMoveCell",
    },
    DemoEventHandler {
        event_kind: FluxEventKind::MouseUpCell,
        handler_name: "onMouseUpCell",
    },
];

const SPEC: DemoSpec = DemoSpec {
    event_handlers: EVENT_HANDLERS,
    tool: Some(("flux.api_cell_demo.tool.paint_line", "API Cell Right Paint")),
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
pub unsafe extern "C" fn onMouseDownCell(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    if event.has_cell == 0 || event.button != PAINT_BUTTON {
        return FluxStatus::OK;
    }
    plugin.dragging = true;
    plugin.last_x = event.cell_x;
    plugin.last_y = event.cell_y;
    paint_cell(host, event.cell_x, event.cell_y)
}

#[no_mangle]
pub unsafe extern "C" fn onMouseMoveCell(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    if !plugin.dragging || event.has_cell == 0 {
        return FluxStatus::OK;
    }
    let status = paint_line(
        host,
        plugin.last_x,
        plugin.last_y,
        event.cell_x,
        event.cell_y,
    );
    if status != FluxStatus::OK {
        return status;
    }
    plugin.last_x = event.cell_x;
    plugin.last_y = event.cell_y;
    FluxStatus::OK
}

#[no_mangle]
pub unsafe extern "C" fn onMouseUpCell(
    plugin: *mut FluxPluginHandle,
    event: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let (plugin, event, host) = match validate_event_call(plugin, event, host) {
        Ok(values) => values,
        Err(status) => return status,
    };
    if !plugin.dragging {
        return FluxStatus::OK;
    }
    plugin.dragging = false;
    if event.has_cell == 0 {
        return FluxStatus::OK;
    }
    paint_line(
        host,
        plugin.last_x,
        plugin.last_y,
        event.cell_x,
        event.cell_y,
    )
}

unsafe fn paint_cell(host: &mut FluxRuntimeHost, x: u32, y: u32) -> FluxStatus {
    let Some(set_cell_material) = host.set_cell_material else {
        return FluxStatus::FAILED;
    };
    set_cell_material(
        host.context,
        x,
        y,
        FluxUtf8Slice::from_str("flux.default.cell.metal"),
    )
}

unsafe fn paint_line(
    host: &mut FluxRuntimeHost,
    from_x: u32,
    from_y: u32,
    to_x: u32,
    to_y: u32,
) -> FluxStatus {
    let mut x0 = from_x as i32;
    let mut y0 = from_y as i32;
    let x1 = to_x as i32;
    let y1 = to_y as i32;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        let status = paint_cell(host, x0 as u32, y0 as u32);
        if status != FluxStatus::OK {
            return status;
        }
        if x0 == x1 && y0 == y1 {
            return FluxStatus::OK;
        }
        let e2 = err * 2;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}
