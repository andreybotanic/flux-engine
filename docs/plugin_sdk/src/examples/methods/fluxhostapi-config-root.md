```rust
fn print_config_root(host: &FluxHostApi) -> FluxStatus {
    let config_root = match host.config_root() {
        Ok(path) => path,
        Err(status) => return status,
    };
    println!("config root: {config_root}");
    FluxStatus::OK
}
```
