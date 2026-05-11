```rust
unsafe fn create_plugin_instance(symbol: FluxPluginCreateFn, host: &FluxHostApi) -> Result<*mut FluxPluginHandle, FluxStatus> {
    let mut plugin = std::ptr::null_mut();
    let status = unsafe { symbol(host as *const FluxHostApi, &mut plugin) };
    if status.is_ok() && !plugin.is_null() {
        Ok(plugin)
    } else {
        Err(status)
    }
}
```
