```rust
let id = registry.stable_id_by_index(0).expect("registry has index 0");
assert!(id.as_str().starts_with("flux.demo.gas."));
```
