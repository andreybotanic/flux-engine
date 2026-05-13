```rust
let descriptor = registry
    .cell_by_material(CellMaterial::new("flux.demo.cell.steel"))
    .expect("registered material should resolve");
assert_eq!(descriptor.id.as_str(), "flux.demo.cell.steel");
```
