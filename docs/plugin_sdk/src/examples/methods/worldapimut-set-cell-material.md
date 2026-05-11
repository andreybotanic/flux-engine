```rust
let changed = world_mut.set_cell_material(
    UVec2::new(20, 14),
    CellMaterial::new("flux.default.cell.metal"),
)?;
if changed {
    println!("cell material updated");
}
```
