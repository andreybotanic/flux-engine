```rust
unsafe extern "C" fn on_world_after_save(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.struct_size as usize == std::mem::size_of::<FluxEmptyEventPayload>() {
        plugin.counter = 0;
    }
    FluxStatus::OK
}
```
