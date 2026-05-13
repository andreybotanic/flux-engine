```rust
let overlays = registry.overlays();
assert!(overlays.values().any(|overlay| overlay.render_policy == OverlayRenderPolicy::PluginControlled));
```
