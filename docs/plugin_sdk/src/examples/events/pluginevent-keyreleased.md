```rust
unsafe extern "C" fn on_key_released(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxKeyEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    let key = unsafe { std::slice::from_raw_parts(payload.key.ptr, payload.key.len) };
    if key == b"Escape" {
        plugin.dragging = false;
    }
    FluxStatus::OK
}
```
