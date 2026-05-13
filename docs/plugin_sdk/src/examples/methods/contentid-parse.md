```rust
let overlay_id = ContentId::parse("flux.demo.overlay.temperature").expect("valid content id");
assert_eq!(overlay_id.as_str(), "flux.demo.overlay.temperature");
```
