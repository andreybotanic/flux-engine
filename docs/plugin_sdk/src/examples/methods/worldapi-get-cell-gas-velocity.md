```rust
let velocity = world.get_cell_gas_velocity(UVec2::new(24, 18), "oxygen")?;
if velocity.length() > 0.0 {
    println!("gas drift: {:?}", velocity);
}
```
