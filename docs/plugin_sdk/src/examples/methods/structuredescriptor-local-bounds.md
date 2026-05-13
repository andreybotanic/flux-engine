```rust
let bounds = descriptor.local_bounds().expect("descriptor should occupy at least one cell");
assert_eq!(bounds.0, IVec2::ZERO);
```
