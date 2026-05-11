```rust
unsafe fn register_handler(
    callback: FluxRegisterEventHandlerFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    let descriptor = FluxEventHandlerDescriptor::new(
        FluxEventKind::MouseDownCell,
        FluxUtf8Slice::from_str("onMouseDownCell"),
    );
    unsafe { callback(context, &descriptor) }
}
```
