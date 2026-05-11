```rust
let plugin_id = PluginId::parse("flux.demo").expect("valid plugin id");
assert_eq!(plugin_id.as_str(), "flux.demo");
```
