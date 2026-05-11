```rust
let plugin_id = PluginId::parse("flux.demo").expect("plugin id");
let mut registry = PluginRuntimeRegistry::default();
registry.subscribe(plugin_id.clone(), PluginEventKind::MouseDownCell);
assert_eq!(registry.subscribers(PluginEventKind::MouseDownCell), vec![plugin_id]);
```
