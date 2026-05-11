```rust
let symbol = std::ffi::CStr::from_bytes_with_nul(FLUX_PLUGIN_API_VERSION_EXPORT_NAME)
    .expect("export name must stay null-terminated");
assert_eq!(symbol.to_str().unwrap(), "flux_plugin_api_version");
```
