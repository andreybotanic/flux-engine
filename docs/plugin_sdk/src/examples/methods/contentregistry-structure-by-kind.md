```rust
let descriptor = registry
    .structure_by_kind(StructureKind::new("flux.demo.structure.filter"))
    .expect("registered structure kind should resolve");
assert_eq!(descriptor.kind.as_str(), "flux.demo.structure.filter");
```
