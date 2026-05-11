```rust
let mut registry = ContentRegistry::default();
let descriptor = OverlayContentDescriptor {
    id: ContentId::parse("flux.demo.overlay.temperature").expect("id"),
    plugin_id: PluginId::parse("flux.demo").expect("plugin id"),
    mode: OverlayMode::plugin("flux.demo.overlay.temperature"),
    label: "Temperature",
    hotkey: "F9",
    storage: LegacyStorageDescriptor::OverlayMode("temperature"),
};
registry.register_overlay(descriptor);
assert!(registry.overlay_by_mode(OverlayMode::plugin("flux.demo.overlay.temperature")).is_some());
```
