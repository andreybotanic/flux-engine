```rust
unsafe extern "C" fn on_structure_removed(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxStructureEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.structure_id != 0 {
        plugin.counter = plugin.counter.saturating_add(1);
    }
    FluxStatus::OK
}
```
