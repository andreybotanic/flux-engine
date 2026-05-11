```rust
let panels = registry.panels();
assert!(panels.values().all(|panel| !panel.title.is_empty()));
```
