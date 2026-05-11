```rust
let overlay_id = ContentId::parse("flux.demo.overlay.temperature").expect("valid overlay id");
let frame = OverlayFrame::core_default(overlay_id);
assert_eq!(frame.policy, OverlayRenderPolicy::CoreDefault);
```
