```rust
let structures = registry.structures();
assert!(structures.values().all(|descriptor| !descriptor.allowed_rotations.is_empty()));
```
