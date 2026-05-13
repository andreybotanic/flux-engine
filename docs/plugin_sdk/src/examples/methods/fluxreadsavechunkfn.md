```rust
unsafe fn read_counter_chunk(
    callback: FluxReadSaveChunkFn,
    context: *mut std::ffi::c_void,
) -> Result<u32, FluxStatus> {
    let mut version = 0u32;
    let mut len = 0usize;
    let mut bytes = [0u8; 4];
    let status = unsafe {
        callback(
            context,
            FluxUtf8Slice::from_str("flux.demo.save.counter"),
            &mut version,
            bytes.as_mut_ptr(),
            bytes.len(),
            &mut len,
        )
    };
    if !status.is_ok() || version != 1 || len != bytes.len() {
        return Err(status);
    }
    Ok(u32::from_le_bytes(bytes))
}
```
