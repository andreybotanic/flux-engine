```rust
let abi_tag = event_kind_to_abi(PluginEvent::RenderOverlay);
assert_eq!(abi_tag, FluxEventKind::RenderOverlay.as_raw());
```
