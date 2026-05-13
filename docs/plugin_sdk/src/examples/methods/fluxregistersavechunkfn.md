```rust
unsafe fn register_chunk(
    callback: FluxRegisterSaveChunkFn,
    context: *mut std::ffi::c_void,
) -> FluxStatus {
    let descriptor = FluxSaveChunkDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.save.counter"),
        version: 1,
    };
    unsafe { callback(context, &descriptor) }
}
```
