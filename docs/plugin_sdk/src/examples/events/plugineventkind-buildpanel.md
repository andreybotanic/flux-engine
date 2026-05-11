```rust
unsafe extern "C" fn on_build_panel(
    _plugin: *mut FluxPluginHandle,
    payload: *const FluxBuildPanelEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let host = unsafe { &mut *host };
    let Some(submit_hud_block) = host.submit_hud_block else {
        return FluxStatus::FAILED;
    };
    unsafe {
        submit_hud_block(
            host.context,
            FluxUtf8Slice::from_str("Panel"),
            payload.panel_id,
        )
    }
}
```
