```rust
unsafe fn register_tool(callback: FluxRegisterToolFn, context: *mut std::ffi::c_void) -> FluxStatus {
    let descriptor = FluxToolDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.tool.paint"),
        label: FluxUtf8Slice::from_str("Paint"),
        icon_path: FluxUtf8Slice::from_str("icons/paint.png"),
        silhouette_path: FluxUtf8Slice::from_str("icons/paint_silhouette.png"),
    };
    unsafe { callback(context, &descriptor) }
}
```
