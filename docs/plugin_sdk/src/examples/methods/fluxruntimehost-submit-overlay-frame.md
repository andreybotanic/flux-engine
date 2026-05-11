```rust
fn send_overlay(host: &mut FluxRuntimeHost) -> FluxStatus {
    let rgba8 = vec![
        255, 64, 64, 128, 64, 64, 255, 128,
        64, 255, 64, 128, 255, 255, 64, 128,
    ];
    match host.submit_overlay_frame(2, 2, &rgba8) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
