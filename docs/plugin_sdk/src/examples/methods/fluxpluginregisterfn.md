```rust
unsafe fn register_plugin_instance(
    symbol: FluxPluginRegisterFn,
    plugin: *mut FluxPluginHandle,
    registrar: &mut FluxRegistrar,
) -> FluxStatus {
    unsafe { symbol(plugin, registrar as *mut FluxRegistrar) }
}
```
