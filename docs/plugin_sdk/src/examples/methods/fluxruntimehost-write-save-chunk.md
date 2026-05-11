```rust
fn write_counter(host: &mut FluxRuntimeHost, counter: u32) -> FluxStatus {
    let payload = counter.to_le_bytes();
    match host.write_save_chunk("flux.demo.save.counter", 1, &payload) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
