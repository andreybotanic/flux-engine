```rust
unsafe extern "C" fn on_world_loaded(
    plugin: *mut FluxPluginHandle,
    payload: *const FluxEmptyEventPayload,
    host: *mut FluxRuntimeHost,
) -> FluxStatus {
    let _payload = unsafe { &*payload };
    let plugin = unsafe { &mut *plugin };
    let host = unsafe { &mut *host };
    let Some(read_save_chunk) = host.read_save_chunk else {
        return FluxStatus::FAILED;
    };

    let mut version = 0u32;
    let mut len = 0usize;
    let mut bytes = vec![0u8; 4];
    let status = unsafe {
        read_save_chunk(
            host.context,
            FluxUtf8Slice::from_str("flux.demo.save.counter"),
            &mut version,
            bytes.as_mut_ptr(),
            bytes.len(),
            &mut len,
        )
    };
    if status.is_ok() && version == 1 && len == 4 {
        plugin.counter = u32::from_le_bytes(bytes.try_into().expect("4 bytes"));
    }
    FluxStatus::OK
}
```
