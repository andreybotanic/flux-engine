```rust
let changed = world_mut.remove_cell_material(UVec2::new(20, 14))?;
if changed {
    println!("cell was cleared");
}
```
