```rust
let registrar = FluxRegistrar::new(None, None, None, None, None, std::ptr::null_mut());
assert_eq!(registrar.api_version, ENGINE_PLUGIN_API_VERSION_VALUE);
assert!(registrar.register_event_handler.is_none());
```
