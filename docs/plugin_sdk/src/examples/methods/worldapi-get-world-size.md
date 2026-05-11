```rust
let size = world.get_world_size();
let center = UVec2::new(size.x / 2, size.y / 2);
let info = world.get_cell_info(center)?;
assert!(info.in_bounds);
```
