```rust
let structures = world.get_cell_structures_on_layer(
    UVec2::new(24, 18),
    LayerKind::new("flux.core.layer.appearance"),
)?;
assert!(structures.iter().all(|structure| !structure.occupied_cells.is_empty()));
```
