```rust
unsafe extern "C" fn on_world_unloaded(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let _payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    plugin.cell_counters.clear();
    FluxStatus::OK
}
```
