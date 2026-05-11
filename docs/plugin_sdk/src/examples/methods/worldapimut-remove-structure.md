```rust
let removed = world_mut.remove_structure(PlacedStructureId(12))?;
assert!(removed || !removed);
```
