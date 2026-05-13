```rust
fn register_overlay(registrar: &mut FluxRegistrar) -> FluxStatus {
    let descriptor = FluxOverlayDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.overlay.temperature"),
        label: FluxUtf8Slice::from_str("Demo Temperature"),
        hotkey: FluxUtf8Slice::from_str("F6"),
        render_policy: 1,
    };
    match registrar.register_overlay(&descriptor) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
