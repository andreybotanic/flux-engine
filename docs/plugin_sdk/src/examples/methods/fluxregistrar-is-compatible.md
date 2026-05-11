```rust
fn validate_registrar(registrar: &FluxRegistrar) -> FluxStatus {
    if !registrar.is_compatible() {
        return FluxStatus::FAILED;
    }
    FluxStatus::OK
}
```
