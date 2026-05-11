```rust
let event = PluginEvent::BuildHudForCell { cell: UVec2::new(12, 8) };
assert_eq!(event.kind(), PluginEventKind::BuildHudForCell);
```
