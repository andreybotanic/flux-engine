```rust
let id = SubstanceId::parse("flux.demo.gas.oxygen").expect("valid substance id");
assert_eq!(id.leaf(), "oxygen");
```
