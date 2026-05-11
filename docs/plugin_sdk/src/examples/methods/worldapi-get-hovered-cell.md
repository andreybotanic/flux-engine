```rust
if let Some(cell) = world.get_hovered_cell() {
    let info = world.get_cell_info(cell)?;
    println!("hovered cell {:?} contains {} structures", cell, info.structures.len());
}
```
