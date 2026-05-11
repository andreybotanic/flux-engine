```rust
unsafe extern "C" fn on_mouse_leave_cell(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxMouseCellEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.has_cell == 0 {
        plugin.dragging = false;
    }
    FluxStatus::OK
}
```
