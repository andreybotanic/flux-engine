```rust
unsafe extern "C" fn on_build_hud_for_cell(
    _plugin: *mut FluxPluginHandle,
    payload: *const FluxBuildHudForCellEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let host = unsafe { &mut *host };
    let Some(submit_hud_block) = host.submit_hud_block else {
        return FluxStatus::FAILED;
    };
    let line = format!("cell ({}, {})", payload.cell_x, payload.cell_y);
    unsafe {
        submit_hud_block(
            host.context,
            FluxUtf8Slice::from_str("Demo"),
            FluxUtf8Slice::from_str(line.as_str()),
        )
    }
}
```
