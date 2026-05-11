```rust
unsafe extern "C" fn on_tool_selected(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxToolSelectedEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    plugin.dragging = payload.has_tool_id != 0;
    FluxStatus::OK
}
```
