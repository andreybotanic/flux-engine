```rust
unsafe extern "C" fn on_simulation_post_cell_gas_step(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.api_version == ENGINE_PLUGIN_API_VERSION_VALUE {
        plugin.counter = plugin.counter.saturating_add(1);
    }
    FluxStatus::OK
}
```
