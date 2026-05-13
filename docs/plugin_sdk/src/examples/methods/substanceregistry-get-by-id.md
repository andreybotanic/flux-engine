```rust
let oxygen_id = SubstanceId::parse("flux.demo.gas.oxygen").expect("id");
let oxygen = registry.get_by_id(&oxygen_id).expect("oxygen is registered");
assert_eq!(oxygen.label, "Oxygen");
```
