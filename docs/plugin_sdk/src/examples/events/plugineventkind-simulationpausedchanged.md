```rust
unsafe extern "C" fn on_simulation_paused_changed(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxSimulationPausedChangedEvent,
    _host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    plugin.dragging = payload.paused == 0 && plugin.dragging;
    FluxStatus::OK
}
```
