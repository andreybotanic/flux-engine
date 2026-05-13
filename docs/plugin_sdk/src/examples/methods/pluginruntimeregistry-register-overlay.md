```rust
let mut registry = PluginRuntimeRegistry::default();
let descriptor = RuntimeOverlayDescriptor {
    id: ContentId::parse("flux.demo.overlay.temperature").expect("overlay id"),
    plugin_id: PluginId::parse("flux.demo").expect("plugin id"),
    label: "Temperature".to_string(),
    hotkey: Some("F9".to_string()),
    render_policy: OverlayRenderPolicy::PluginControlled,
};
registry.register_overlay(descriptor);
assert!(registry.overlays().contains_key(&ContentId::parse("flux.demo.overlay.temperature").expect("overlay id")));
```
