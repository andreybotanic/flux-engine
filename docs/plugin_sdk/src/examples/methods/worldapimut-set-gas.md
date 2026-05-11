```rust
world_mut.set_gas(
    UVec2::new(42, 18),
    "oxygen",
    300,
    Vec2::ZERO,
)?;
let amount = world.get_cell_gas_amount(UVec2::new(42, 18), "oxygen")?;
assert_eq!(amount, 300);
```
