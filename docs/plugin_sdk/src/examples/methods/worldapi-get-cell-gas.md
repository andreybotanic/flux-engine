```rust
let gas = world.get_cell_gas(UVec2::new(24, 18))?;
let non_zero_species = gas.species.iter().filter(|entry| entry.amount > 0).count();
assert!(gas.total_amount >= non_zero_species as u32);
```
