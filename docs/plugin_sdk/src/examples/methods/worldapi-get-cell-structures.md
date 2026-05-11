```rust
let structures = world.get_cell_structures(UVec2::new(24, 18))?;
for structure in &structures {
    println!("structure {} at {:?}", structure.kind.as_str(), structure.origin);
}
```
