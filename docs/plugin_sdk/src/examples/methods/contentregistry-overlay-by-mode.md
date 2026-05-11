```rust
let overlay = registry
    .overlay_by_mode(OverlayMode::plugin("flux.demo.overlay.temperature"))
    .expect("registered overlay should resolve");
assert_eq!(overlay.label, "Temperature");
```
