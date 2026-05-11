```rust
if host.api_version != ENGINE_PLUGIN_API_VERSION_VALUE {
    return Err(format!(
        "plugin was loaded with ABI {}, but the engine exports {}",
        host.api_version,
        ENGINE_PLUGIN_API_VERSION_VALUE,
    ));
}
```
