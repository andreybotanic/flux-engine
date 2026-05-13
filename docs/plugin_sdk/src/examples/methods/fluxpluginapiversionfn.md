```rust
unsafe fn validate_api_version(symbol: FluxPluginApiVersionFn) -> Result<(), String> {
    let version = unsafe { symbol() };
    if version != ENGINE_PLUGIN_API_VERSION_VALUE {
        return Err(format!("plugin ABI {} is unsupported", version));
    }
    Ok(())
}
```
