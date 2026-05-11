```rust
let pumps = world.get_structures_by_type(StructureKind::new("flux.default.structure.pump"));
assert!(pumps.iter().all(|structure| structure.kind.as_str().contains("pump")));
```
