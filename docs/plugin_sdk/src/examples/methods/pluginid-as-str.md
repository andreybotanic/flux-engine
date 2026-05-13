```rust
let plugin_id = PluginId::parse("flux.demo").expect("valid plugin id");
assert!(plugin_id.as_str().starts_with("flux."));
```
