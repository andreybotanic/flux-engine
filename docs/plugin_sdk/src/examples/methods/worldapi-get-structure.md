```rust
if let Some(structure) = world.get_structure(PlacedStructureId(12)) {
    println!("{} occupies {} cells", structure.kind.as_str(), structure.occupied_cells.len());
}
```
