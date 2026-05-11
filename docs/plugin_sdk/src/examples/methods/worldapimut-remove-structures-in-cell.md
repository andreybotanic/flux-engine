```rust
let removed = world_mut.remove_structures_in_cell(UVec2::new(30, 12))?;
println!("removed {} structure(s)", removed.len());
```
