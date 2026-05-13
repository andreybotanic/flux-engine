```rust
fn print_plugin_root(host: &FluxHostApi) -> FluxStatus {
    let plugin_root = match host.plugin_root() {
        Ok(path) => path,
        Err(status) => return status,
    };
    println!("plugin root: {plugin_root}");
    FluxStatus::OK
}
```
