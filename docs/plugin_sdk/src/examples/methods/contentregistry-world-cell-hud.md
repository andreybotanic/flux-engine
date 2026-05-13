```rust
let hud = registry.world_cell_hud().expect("world cell HUD should be configured");
assert!(!hud.label.is_empty());
```
