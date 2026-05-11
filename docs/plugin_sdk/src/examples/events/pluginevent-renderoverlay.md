```rust
unsafe extern "C" fn on_render_overlay(
    _plugin: *mut FluxPluginHandle,
    payload: *const FluxRenderOverlayEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let payload = unsafe { &*payload };
    let host = unsafe { &mut *host };
    let Some(submit_overlay_frame) = host.submit_overlay_frame else {
        return FluxStatus::FAILED;
    };

    let width = 2u32;
    let height = 2u32;
    let rgba8 = if payload.overlay_id.len == 0 {
        vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    } else {
        vec![255, 64, 32, 255, 255, 64, 32, 255, 255, 64, 32, 255, 255, 64, 32, 255]
    };
    unsafe { submit_overlay_frame(host.context, width, height, rgba8.as_ptr(), rgba8.len()) }
}
```
