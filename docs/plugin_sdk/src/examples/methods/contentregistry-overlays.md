```rust
let overlays = registry.overlays();
assert!(overlays.values().any(|overlay| overlay.hotkey.starts_with('F')));
```
