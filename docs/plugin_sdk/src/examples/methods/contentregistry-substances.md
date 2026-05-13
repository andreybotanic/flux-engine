```rust
let substances = registry.substances();
assert!(substances.values().all(|definition| definition.flags.gas));
```
