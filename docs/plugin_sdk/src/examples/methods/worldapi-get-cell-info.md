```rust
let info = world.get_cell_info(UVec2::new(24, 18))?;
if info.is_editable {
    println!("cell {:?} can be modified", info.cell);
}
```
