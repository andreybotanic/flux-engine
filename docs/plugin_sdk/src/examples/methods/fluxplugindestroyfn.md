```rust
unsafe fn destroy_plugin_instance(symbol: FluxPluginDestroyFn, plugin: *mut FluxPluginHandle) {
    unsafe { symbol(plugin) };
}
```
