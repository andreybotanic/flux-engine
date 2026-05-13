```rust
let id = SubstanceId::parse("flux.demo.gas.oxygen").expect("valid substance id");
assert_eq!(id.as_str(), "flux.demo.gas.oxygen");
```
