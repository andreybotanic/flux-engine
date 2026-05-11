```rust
unsafe extern "C" fn on_mouse_up_cell(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxMouseCellEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.button == 1 {
        plugin.dragging = false;
    }
    FluxStatus::OK
}
```
