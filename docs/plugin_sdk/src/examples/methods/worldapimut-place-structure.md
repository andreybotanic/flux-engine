```rust
let placed = world_mut.place_structure(StructurePlacement {
    kind: StructureKind::new("flux.default.structure.filter"),
    origin: UVec2::new(30, 12),
    rotation: StructureRotation::Deg90,
    params: StructureParams::None,
})?;
if let Some(id) = placed {
    println!("placed structure id {}", id.0);
}
```
