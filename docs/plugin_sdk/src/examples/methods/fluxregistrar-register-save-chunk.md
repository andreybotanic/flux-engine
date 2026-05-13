```rust
fn register_chunk(registrar: &mut FluxRegistrar) -> FluxStatus {
    let descriptor = FluxSaveChunkDescriptor {
        id: FluxUtf8Slice::from_str("flux.demo.save.counter"),
        version: 1,
    };
    match registrar.register_save_chunk(&descriptor) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
