```rust
let subscribers = registry.subscribers(PluginEvent::RenderOverlay);
assert!(subscribers.iter().all(|plugin_id| plugin_id.as_str().starts_with("flux.")));
```
