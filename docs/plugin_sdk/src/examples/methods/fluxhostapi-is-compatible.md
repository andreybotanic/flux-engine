```rust
fn validate_host(host: &FluxHostApi) -> FluxStatus {
    if !host.is_compatible() {
        return FluxStatus::FAILED;
    }
    FluxStatus::OK
}
```
