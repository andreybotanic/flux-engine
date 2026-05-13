```rust
let provider_plugins = registry.provider_plugins();
assert!(provider_plugins.iter().any(|plugin_id| plugin_id.as_str() == "flux.demo"));
```
