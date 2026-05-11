```rust
unsafe fn submit_counter_block(
    callback: FluxSubmitHudBlockFn,
    context: *mut std::ffi::c_void,
    count: u32,
) -> FluxStatus {
    let line = format!("painted cells: {}", count);
    unsafe {
        callback(
            context,
            FluxUtf8Slice::from_str("Paint Demo"),
            FluxUtf8Slice::from_str(line.as_str()),
        )
    }
}
```
