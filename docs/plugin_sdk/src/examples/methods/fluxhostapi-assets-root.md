```rust
fn print_assets_root(host: &FluxHostApi) -> FluxStatus {
    let assets_root = match host.assets_root() {
        Ok(path) => path,
        Err(status) => return status,
    };
    println!("assets root: {assets_root}");
    FluxStatus::OK
}
```
