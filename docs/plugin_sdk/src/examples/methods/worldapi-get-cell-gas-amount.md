```rust
let amount = world.get_cell_gas_amount(UVec2::new(24, 18), "oxygen")?;
if amount > 0 {
    println!("oxygen count: {}", amount);
}
```
