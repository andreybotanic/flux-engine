```rust
unsafe extern "C" fn on_mouse_down_cell(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxMouseCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    let host = unsafe { &mut *host };
    if payload.has_cell == 0 || payload.is_over_ui != 0 || payload.button != 1 {
        return FluxStatus::OK;
    }

    plugin.dragging = true;
    plugin.last_x = payload.cell_x;
    plugin.last_y = payload.cell_y;

    let Some(set_cell_material) = host.set_cell_material else {
        return FluxStatus::FAILED;
    };
    unsafe {
        set_cell_material(
            host.context,
            payload.cell_x,
            payload.cell_y,
            FluxUtf8Slice::from_str("flux.default.cell.metal"),
        )
    }
}
```
