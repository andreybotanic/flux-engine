```rust
fn report_startup_error(host: &FluxHostApi, message: &str) -> FluxStatus {
    match host.write_error(message) {
        Ok(()) => FluxStatus::OK,
        Err(status) => status,
    }
}
```
