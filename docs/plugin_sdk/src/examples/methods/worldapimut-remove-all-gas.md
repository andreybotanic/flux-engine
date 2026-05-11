```rust
world_mut.remove_all_gas(UVec2::new(42, 18))?;
let gas = world.get_cell_gas(UVec2::new(42, 18))?;
assert_eq!(gas.total_amount, 0);
```
