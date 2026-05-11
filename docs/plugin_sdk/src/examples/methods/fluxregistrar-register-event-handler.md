```rust
fn register_mouse_handler(registrar: &mut FluxRegistrar) -> FluxStatus {
    match registrar.register_event_handler(FluxEventKind::MouseDownCell, "onMouseDownCell") {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
