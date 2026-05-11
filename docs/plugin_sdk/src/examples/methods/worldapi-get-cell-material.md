```rust
if let Some(material) = world.get_cell_material(UVec2::new(24, 18))? {
    println!("solid material: {}", material.as_str());
}
```
