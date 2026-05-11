```rust
unsafe extern "C" fn on_world_created(
    _plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    if payload.api_version != ENGINE_PLUGIN_API_VERSION_VALUE {
        return FluxStatus::FAILED;
    }
    FluxStatus::OK
}
```
