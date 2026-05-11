```rust
let masses = registry.molecular_masses();
assert!(masses.windows(2).all(|pair| pair[0] <= pair[1]));
```
