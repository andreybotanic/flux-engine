```rust
unsafe fn register_overlay(
    callback: FluxRegisterOverlayFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    let descriptor = FluxOverlayDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.overlay.temperature"),
        label: FluxUtf8Slice::from_str("Temperature"),
        hotkey: FluxUtf8Slice::from_str("F9"),
        render_policy: 1,
    };
    unsafe { callback(context, &descriptor) }
}
```
