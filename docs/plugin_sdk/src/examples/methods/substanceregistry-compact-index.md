```rust
let oxygen_index = registry.compact_index("o2").expect("alias should resolve");
let oxygen = registry.get(oxygen_index).expect("index should be valid");
assert_eq!(oxygen.id.leaf(), "oxygen");
```
