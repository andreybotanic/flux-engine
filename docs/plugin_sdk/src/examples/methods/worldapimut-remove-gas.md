```rust
let removed = world_mut.remove_gas(UVec2::new(42, 18), 100)?;
if removed.total_amount > 0 {
    println!("removed {} particles", removed.total_amount);
}
```
