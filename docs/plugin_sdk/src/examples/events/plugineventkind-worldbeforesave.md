```rust
unsafe extern "C" fn on_world_before_save(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let _payload = unsafe { &*payload };
    let plugin = unsafe { &*plugin };
    let host = unsafe { &mut *host };
    let Some(write_save_chunk) = host.write_save_chunk else {
        return FluxStatus::FAILED;
    };

    let bytes = plugin.counter.to_le_bytes();
    unsafe {
        write_save_chunk(
            host.context,
            FluxUtf8Slice::from_str("flux.demo.save.counter"),
            1,
            bytes.as_ptr(),
            bytes.len(),
        )
    }
}
```
