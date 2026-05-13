```rust
unsafe extern "C" fn on_structure_placed(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxStructureEventPayload,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    if payload.cell_x < 102 && payload.cell_y < 102 {
        plugin.last_x = payload.cell_x;
        plugin.last_y = payload.cell_y;
    }
    FluxStatus::OK
}
```
