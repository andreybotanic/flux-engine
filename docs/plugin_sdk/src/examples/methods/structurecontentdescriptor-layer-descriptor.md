```rust
let descriptor = registry
    .structure_by_kind(StructureKind::new("flux.demo.structure.filter"))
    .expect("registered structure");
let layer_descriptor = descriptor.layer_descriptor(StructureRotation::Deg90);
assert!(layer_descriptor.size_in_cells().x >= 1);
```
