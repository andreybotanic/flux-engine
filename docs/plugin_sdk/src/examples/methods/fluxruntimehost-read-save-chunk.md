```rust
fn read_counter(host: &mut FluxRuntimeHost) -> FluxStatus {
    let Some((version, bytes)) = (match host.read_save_chunk("flux.demo.save.counter") {
        Ok(chunk) => chunk,
        Err(status) => return status,
    }) else {
        println!("counter chunk is missing");
        return FluxStatus::OK;
    };

    if version == 1 && bytes.len() == 4 {
        let counter = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        println!("loaded counter {counter}");
    }
    FluxStatus::OK
}
```
