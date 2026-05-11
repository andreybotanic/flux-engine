```rust
unsafe fn write_counter_chunk(
    callback: FluxWriteSaveChunkFn,
    context: *mut std::ffi::c_void,
    counter: u32,
) -> FluxStatus {
    let bytes = counter.to_le_bytes();
    unsafe {
        callback(
            context,
            FluxUtf8Slice::from_str("flux.demo.save.counter"),
            1,
            bytes.as_ptr(),
            bytes.len(),
        )
    }
}
```
