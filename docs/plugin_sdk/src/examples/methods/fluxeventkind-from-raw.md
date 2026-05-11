```rust
let kind = FluxEventKind::from_raw(21).expect("known event tag");
assert_eq!(kind, FluxEventKind::RenderOverlay);
```
