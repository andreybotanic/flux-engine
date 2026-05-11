```rust
fn validate_runtime_host(host: &FluxRuntimeHost) -> FluxStatus {
    if !host.is_compatible() {
        return FluxStatus::FAILED;
    }
    FluxStatus::OK
}
```
