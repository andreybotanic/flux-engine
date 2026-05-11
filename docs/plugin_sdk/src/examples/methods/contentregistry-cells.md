```rust
let cells = registry.cells();
assert!(cells.keys().any(|id| id.as_str().starts_with("flux.")));
```
