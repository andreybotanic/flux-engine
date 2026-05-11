```rust
unsafe extern "C" fn on_mouse_enter_cell(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxMouseCellEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.has_cell != 0 {
        plugin.last_x = payload.cell_x;
        plugin.last_y = payload.cell_y;
    }
    FluxStatus::OK
}
```
