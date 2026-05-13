```rust
let plugin_id = PluginId::parse("flux.demo").expect("plugin id");
let mut registry = ContentRegistry::default();
registry.register_provider_plugin(plugin_id.clone());
assert!(registry.provider_plugins().contains(&plugin_id));
```
