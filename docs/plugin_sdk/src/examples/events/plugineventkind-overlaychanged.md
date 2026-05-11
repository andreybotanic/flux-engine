```rust
unsafe extern "C" fn on_overlay_changed(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxOverlayChangedEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    plugin.dragging = payload.has_overlay_id != 0;
    FluxStatus::OK
}
```
