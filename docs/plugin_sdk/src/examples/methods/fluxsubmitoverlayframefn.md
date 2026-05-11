```rust
unsafe fn submit_checkerboard(
    callback: FluxSubmitOverlayFrameFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    let rgba8 = [
        255, 0, 0, 255, 0, 0, 0, 0,
        0, 0, 0, 0, 255, 0, 0, 255,
    ];
    unsafe { callback(context, 2, 2, rgba8.as_ptr(), rgba8.len()) }
}
```
