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
    host.set_cell_material(
        UVec2::new(payload.cell_x, payload.cell_y),
        "flux.default.cell.metal",
    )
    .map(|()| FluxStatus::OK)
    .unwrap_or_else(|status| status)
}
```
