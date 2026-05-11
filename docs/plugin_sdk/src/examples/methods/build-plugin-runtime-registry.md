```rust
let registry = build_plugin_runtime_registry(&[]);
let overlay_count = registry.overlays().len();
let hud_subscribers = registry.subscribers(PluginEvent::BuildHudForCell);
assert!(overlay_count >= hud_subscribers.len());
```
