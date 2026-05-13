```rust
let runtime_host = FluxRuntimeHost::new(std::ptr::null_mut());
assert_eq!(runtime_host.api_version, ENGINE_PLUGIN_API_VERSION_VALUE);
assert!(runtime_host.set_cell_material.is_some());
```
