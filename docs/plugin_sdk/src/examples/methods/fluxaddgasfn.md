```rust
unsafe fn puff_oxygen(callback: FluxAddGasFn, context: *mut std::ffi::c_void) -> FluxStatus {
    let mut added = 0u32;
    let status = unsafe {
        callback(
            context,
            42,
            18,
            FluxUtf8Slice::from_str("flux.default.gas.oxygen"),
            120,
            0.0,
            1.5,
            &mut added,
        )
    };
    if status.is_ok() && added > 0 { FluxStatus::OK } else { status }
}
```
