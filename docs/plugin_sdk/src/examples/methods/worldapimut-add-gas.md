```rust
let added = world_mut.add_gas(
    UVec2::new(42, 18),
    "oxygen",
    150,
    Vec2::new(0.0, 1.0),
)?;
assert!(added <= 150);
```
