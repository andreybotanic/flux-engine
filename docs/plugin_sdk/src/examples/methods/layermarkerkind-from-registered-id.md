```rust
let marker = LayerMarkerKind::from_registered_id("flux.demo.marker.filter".to_string());
assert_eq!(marker.as_str(), "flux.demo.marker.filter");
```
