```rust
let host = FluxHostApi::new(
    FluxUtf8Slice::from_str("plugins/flux.demo"),
    FluxUtf8Slice::from_str("plugins/flux.demo/config"),
    FluxUtf8Slice::from_str("plugins/flux.demo/assets"),
    None,
    std::ptr::null_mut(),
);
assert_eq!(host.api_version, ENGINE_PLUGIN_API_VERSION_VALUE);
```
